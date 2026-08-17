use std::ops::{Deref, DerefMut};

use crate::{
    index::index_type,
    instructions::{InvArgs, Invocation, INS_THROW},
    Catch, Label, MethodLine, RawLabel, Register, RegisterNumber, SwitchData,
};

use super::{BuildError, BuildResult};

index_type!(
    MethodLineId,
    "An opaque reference into a [crate::Method::line] vec"
);

index_type!(
    BlockId,
    "Identifies a basic block. Block 0 is always the entry block."
);

index_type!(
    InstructionId,
    "An opaque reference to a smali instruction, used as an index into an instruction vector"
);

index_type!(VariableId, "An opaque reference to a smali register");

/// Data for a control flow graph
///
/// This type only contains the data needed to run Braun on the associated method, it doesn't
/// contain any of the actual Braun state, that happens when the SSA form is constructed via the
/// [SSABuilder].
#[derive(Debug)]
pub struct MethodCfg<'a> {
    pub regs: MethodRegSpace,
    pub method: Method<'a>,
    pub blocks: Vec<MethodBlock>,
    // Blocks in reverse post order
    pub blocks_rpo: Vec<BlockId>,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Method<'a> {
    #[cfg_attr(feature = "serde", serde(borrow))]
    pub method: crate::Method<'a>,
    pub instructions: Vec<MethodLineId>,
}

impl<'a> Method<'a> {
    pub fn line_for_instruction(&self, ins: InstructionId) -> &MethodLine<'a> {
        let idx = self.instructions[ins.index()];
        self.get_line(idx)
    }

    pub fn get_line(&self, idx: MethodLineId) -> &MethodLine<'a> {
        &self.method.lines[idx.index()]
    }

    fn lines(&self) -> impl Iterator<Item = &MethodLine<'a>> {
        self.method.lines.iter()
    }

    fn packed_switch_data(&self) -> &[SwitchData] {
        &self.method.packed_switch_data
    }

    fn sparse_switch_data(&self) -> &[SwitchData] {
        &self.method.sparse_switch_data
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MethodRegSpace {
    pub locals: RegisterNumber,
    pub params: RegisterNumber,
}

impl VariableId {
    /// Psuedo result register, note that there are actually two psuedo result registers, but since
    /// the pair is used so rarely this is an alias for the first one. For wide instructions, make
    /// sure you use RESULT_FIRST and RESULT_SECOND
    pub const RESULT: VariableId = VariableId(0);
    /// The first psuedo result register: this one is set by all instructions
    pub const RESULT_FIRST: VariableId = VariableId(0);
    /// The second pseudo result register: this one is only set when a method call returns a
    /// potential wide value
    pub const RESULT_SECOND: VariableId = VariableId(1);

    /// The number of psuedo registers
    pub const NUM_PSEUDO_REGISTERS: usize = 2;
}

impl MethodRegSpace {
    /// Number of distinct variables, including the slot reserved for the
    /// implicit `move-result` register.
    pub fn count(&self) -> usize {
        self.locals as usize + self.params as usize + VariableId::NUM_PSEUDO_REGISTERS
    }

    /// Map a smali register into the dense variable space
    ///
    /// These values should be treated as opaque with no concept of what register they correspond to
    /// other than that they are unique
    /// The variables holding the incoming parameters, `p0` first.
    pub fn param_variables(&self) -> impl Iterator<Item = VariableId> {
        let first = VariableId::NUM_PSEUDO_REGISTERS + self.locals as usize;
        (first..first + self.params as usize).map(VariableId::new)
    }

    pub fn variable(&self, reg: Register) -> VariableId {
        let offset = if reg.is_param() {
            self.locals as usize + reg.num() as usize
        } else {
            reg.num() as usize
        };

        VariableId::new(VariableId::NUM_PSEUDO_REGISTERS + offset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeKind {
    FallThrough,
    Goto,
    Conditional,
    Switch(i32),
    SwitchDefault,
    Exception { line: MethodLineId },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Successor {
    pub id: BlockId,
    pub kind: EdgeKind,
}

#[derive(Debug)]
pub struct MethodBlock {
    /// Instruction index where this block starts
    pub first: InstructionId,
    /// Instruction index where this block ends, exclusive
    pub end: InstructionId,
    /// Successors of this block (blocks that this block has an outgoing edge into)
    pub successors: Vec<Successor>,
    /// Blocks with an outgoing edge into this one, deduplicated and restricted to blocks reachable
    /// from block 0. Derived from [Self::successors] at construction and not maintained afterwards.
    pub inbound_edges: Vec<BlockId>,
    /// A deduplicated view of the blocks that this block reaches. [MethodBlock::successors] may
    /// contain duplicates so use this when deduplication is required.
    pub outbound_edges: Vec<BlockId>,
}

impl MethodBlock {
    pub fn unique_outbound_block_ids(&self) -> impl Iterator<Item = BlockId> {
        self.outbound_edges.iter().copied()
    }
}

impl<'a> MethodCfg<'a> {
    pub fn from_method(method: crate::Method<'a>) -> BuildResult<Self> {
        let method = Method {
            method,
            instructions: Vec::new(),
        };

        MethodMapCreator::new(method)
            .run()
            .and_then(|it| it.into_block_spec())
            .and_then(|it| it.into_cfg())
    }

    pub fn get_block(&self, block_id: BlockId) -> &MethodBlock {
        &self.blocks[block_id.index()]
    }

    pub fn instructions_for_block(&self, block_id: BlockId) -> InstructionIterator<'_, 'a> {
        let block = self.get_block(block_id);
        InstructionIterator {
            at: block.first,
            end: block.end,
            method: &self.method,
        }
    }
}

pub struct InstructionIterator<'m, 'a> {
    at: InstructionId,
    end: InstructionId,
    method: &'m Method<'a>,
}

impl<'m, 'a> Iterator for InstructionIterator<'m, 'a> {
    type Item = (InstructionId, &'m Invocation<'a>);
    fn next(&mut self) -> Option<Self::Item> {
        if self.at == self.end {
            return None;
        }
        let next = self.at;
        self.at = InstructionId::new(next.index() + 1);
        if let MethodLine::Instruction(inv) = self.method.line_for_instruction(next) {
            return Some((next, inv));
        }
        unreachable!("instructions vec contained a non-instruction line")
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = self.end.index().saturating_sub(self.at.index());
        (size, Some(size))
    }
}

impl<'m, 'a> ExactSizeIterator for InstructionIterator<'m, 'a> {}

struct LabelCollection {
    labels: Vec<(Label, InstructionId)>,
}

impl Deref for LabelCollection {
    type Target = Vec<(Label, InstructionId)>;
    fn deref(&self) -> &Self::Target {
        &self.labels
    }
}

impl DerefMut for LabelCollection {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.labels
    }
}

impl LabelCollection {
    fn new() -> Self {
        Self { labels: Vec::new() }
    }
    fn find_label(&self, label: Label) -> BuildResult<InstructionId> {
        self.labels
            .iter()
            .find_map(|(l, at)| (l == &label).then_some(*at))
            .ok_or_else(|| BuildError::invalid_input(format!("missing label: {label}")))
    }
}

/// The first step in building the Method CFG is to walk all lines and get instructions and labels
/// collected for later block creation. Since smali can jump to a goto that we haven't seen yet,
/// there is no way to assign successor blocks without this walk.
struct MethodMapCreator<'a> {
    regs: MethodRegSpace,
    method: Method<'a>,
    labels: LabelCollection,
    catches: Vec<(Catch<'a>, MethodLineId)>,
}

impl<'a> MethodMapCreator<'a> {
    fn new(method: Method<'a>) -> Self {
        Self {
            method,
            regs: MethodRegSpace::default(),
            labels: LabelCollection::new(),
            catches: Vec::new(),
        }
    }

    fn run(mut self) -> BuildResult<Self> {
        let mut instructions = Vec::new();
        for (line_num, line) in self.method.lines().enumerate() {
            match line {
                MethodLine::Instruction(inv) => {
                    // Store all instructions as a reference to their method line
                    instructions.push(MethodLineId::new(line_num));

                    // We want to know how many of each register type is used
                    for reg in inv
                        .written_registers()
                        .iter()
                        .chain(inv.read_registers().iter())
                    {
                        if reg.is_param() {
                            self.regs.params = self.regs.params.max(reg.num() + 1);
                        } else {
                            self.regs.locals = self.regs.locals.max(reg.num() + 1);
                        }
                    }
                }
                // If we see a label, associate it with the next instruction, this is for things
                // like:
                //
                //     :goto_0
                //     <INSTUCTION>
                //
                // Where <INSTRUCTION> is then the start of a block
                MethodLine::LabelDef(label) => self
                    .labels
                    .push((*label, InstructionId::new(instructions.len()))),
                MethodLine::Catch(catch) => {
                    self.catches.push((*catch, MethodLineId::new(line_num)))
                }
                MethodLine::Unset => {
                    return Err(BuildError::invalid_input("undefined method line"));
                }
            }
        }
        self.method.instructions = instructions;
        Ok(self)
    }

    fn into_block_spec(self) -> BuildResult<MethodBlockSpecification<'a>> {
        if self.method.instructions.is_empty() {
            return Ok(MethodBlockSpecification::empty(self));
        }

        let mut catch_ranges = Vec::with_capacity(self.catches.len());
        // Collect all catch ranges for this pass and the next
        for (catch, line) in &self.catches {
            let (start_label, end_label, dest_label) = match catch {
                Catch::All(c) => (c.start_label, c.end_label, c.dest_label),
                Catch::Named(c) => (c.start_label, c.end_label, c.dest_label),
            };
            let start = self.labels.find_label(start_label)?;
            let end = self.labels.find_label(end_label)?;
            if start > end {
                return Err(BuildError::invalid_input(format!(
                    "label {start_label} comes after {end_label}"
                )));
            }
            let handler = self.labels.find_label(dest_label)?;
            if handler.index() >= self.method.instructions.len() {
                return Err(BuildError::invalid_input(format!(
                    "invalid dest label {dest_label} with no body"
                )));
            }
            catch_ranges.push(CatchRange {
                start,
                end,
                handler,
                line: *line,
            });
        }

        // Find all blocks by walking instructions and determining if that instruction can start a
        // block.
        let mut blocks: Vec<BlockBounds> = Vec::new();
        let mut inst_to_block: Vec<BlockId> = Vec::new();

        let mut start = InstructionId::new(0);
        let mut prev_terminated = false;

        // View into collected.labels for checking if the instruction is a label
        let mut label_idx = 0;

        for (i, line_id) in self.method.instructions.iter().enumerate() {
            let ins_id = InstructionId::new(i);

            let mut has_label = false;
            // See if the instruction was labelled, note we're potentially consuming multiple labels
            // here that is why it's a loop
            while label_idx < self.labels.len() && self.labels[label_idx].1 == ins_id {
                has_label = true;
                label_idx += 1;
            }

            let line = self.method.get_line(*line_id);
            let MethodLine::Instruction(inv) = line else {
                unreachable!("instructions vec contained a non-instruction line")
            };

            // Block leaders are defined as:
            //
            // (1) The first instruction in a method (implicit, we don't handle that directly)
            // (2) An instruction with a label
            // (3) An instruction following a terminating instruction
            // (4) An instruction that can throw while inside of a try/catch
            let is_leader = has_label
                || prev_terminated
                || (inv.can_throw() && catch_ranges.iter().any(|it| it.contains(ins_id)));

            if is_leader && ins_id.index() > 0 {
                blocks.push(BlockBounds { start, end: ins_id });
                start = ins_id;
            }

            inst_to_block.push(BlockId::new(blocks.len()));
            prev_terminated =
                inv.is_return() || inv.instruction() == INS_THROW || inv.is_cond() || inv.is_jump();
        }

        // Close the last block
        blocks.push(BlockBounds {
            start,
            end: InstructionId::new(self.method.instructions.len()),
        });

        Ok(MethodBlockSpecification {
            regs: self.regs,
            method: self.method,
            labels: self.labels,
            catch_ranges,
            blocks,
            inst_to_block,
        })
    }
}

#[derive(Clone, Copy)]
struct BlockBounds {
    start: InstructionId,
    end: InstructionId,
}

impl BlockBounds {
    fn last_instruction(&self) -> InstructionId {
        InstructionId::new(self.end.index().saturating_sub(1))
    }
}

#[derive(Clone, Copy)]
struct CatchRange {
    start: InstructionId,
    end: InstructionId,
    handler: InstructionId,
    line: MethodLineId,
}

impl CatchRange {
    fn contains(&self, id: InstructionId) -> bool {
        id >= self.start && id < self.end
    }
}

/// Return the BlockId's in reverse post order
///
/// The idea here is that this should speed up Braun later by allowing us to place less incomplete
/// phis since a block's predecessors will likely be filled before it is. If the given method has no
/// loops this should completely remove the need for those since the blocks will be topologically
/// sorted.
fn reverse_post_order(blocks: &mut [MethodBlock]) -> Vec<BlockId> {
    if blocks.is_empty() {
        return Vec::new();
    }

    // First let's deduplicate edges to the same block inside of a given MethodBlock
    // and extract the BlockIds per block, this is kinda a flattening.
    let successors: Vec<Vec<BlockId>> = blocks
        .iter()
        .map(|block| {
            let mut distinct: Vec<BlockId> = Vec::with_capacity(block.successors.len());
            for succ in &block.successors {
                if !distinct.contains(&succ.id) {
                    distinct.push(succ.id);
                }
            }
            distinct
        })
        .collect();

    let mut visited = vec![false; blocks.len()];

    // Post-order Vec
    let mut post = Vec::with_capacity(blocks.len());

    let mut stack = vec![Frame::new(BlockId::new(0))];
    visited[0] = true;

    while let Some(mut frame) = stack.pop() {
        let block = frame.block;

        // The successors of this block in source order
        let succs = &successors[block.index()];

        // This block is done when we've visited all successors, we can then push it so it's in post
        // order
        if frame.next >= succs.len() {
            post.push(block);
            continue;
        }

        // Otherwise we need to visit the next successor
        let succ = succs[frame.next];
        frame.next += 1;
        stack.push(frame);

        // Recorded before the visited check so back edges and cross edges count
        blocks[succ.index()].inbound_edges.push(block);

        // New blocks need to be pushed onto the stack and marked as visited
        if !visited[succ.index()] {
            visited[succ.index()] = true;
            stack.push(Frame::new(succ));
        }
    }

    for (idx, distinct) in successors.into_iter().enumerate() {
        blocks[idx].outbound_edges = distinct;
    }

    // Reverse it for RPO
    post.reverse();
    post
}

/// A depth first search frame: the block being visited and the index of the next
/// successor to descend into.
struct Frame {
    block: BlockId,
    next: usize,
}

impl Frame {
    fn new(block: BlockId) -> Self {
        Self { block, next: 0 }
    }
}

/// The second phase of the Method CFG production will delineate block boundaries and provide a map
/// of instructions to blocks
struct MethodBlockSpecification<'a> {
    regs: MethodRegSpace,
    labels: LabelCollection,
    method: Method<'a>,
    catch_ranges: Vec<CatchRange>,
    blocks: Vec<BlockBounds>,
    inst_to_block: Vec<BlockId>,
}

impl<'a> MethodBlockSpecification<'a> {
    fn empty(collected: MethodMapCreator<'a>) -> Self {
        Self {
            regs: collected.regs,
            method: collected.method,
            labels: collected.labels,
            blocks: Vec::new(),
            inst_to_block: Vec::new(),
            catch_ranges: Vec::new(),
        }
    }

    /// This type can finally be converted to a [MethodCfg]
    fn into_cfg(self) -> BuildResult<MethodCfg<'a>> {
        let mut method_blocks = Vec::new();

        for (bid, block) in self.blocks.iter().enumerate() {
            let mut successors = Vec::new();

            let ins_id = block.last_instruction();

            let MethodLine::Instruction(inv) = self.method.line_for_instruction(ins_id) else {
                unreachable!("instructions vec contained a non-instruction line")
            };

            if inv.is_jump() {
                let InvArgs::Label(raw_label) = &inv.args() else {
                    return Err(BuildError::invalid_input(
                        "jump instruction with invalid args",
                    ));
                };

                let label_at = self.get_label_instruction(raw_label)?;
                let block_id = self.inst_to_block[label_at.index()];

                successors.push(Successor {
                    id: block_id,
                    kind: EdgeKind::Goto,
                });
            } else if inv.is_forking_cond() {
                let raw_label = match inv.args() {
                    InvArgs::OneRegLabel(_, label) | InvArgs::TwoRegLabel(_, _, label) => label,
                    _ => {
                        return Err(BuildError::invalid_input(
                            "forking condition instruction with invalid args",
                        ));
                    }
                };

                let label_at = self.get_label_instruction(raw_label)?;
                let block_id = self.inst_to_block[label_at.index()];

                successors.push(Successor {
                    id: block_id,
                    kind: EdgeKind::Conditional,
                });

                successors.push(Successor {
                    id: self.required_next_block_id(bid)?,
                    kind: EdgeKind::FallThrough,
                });
            } else if inv.is_switch() {
                let InvArgs::OneRegLabel(_, raw) = inv.args() else {
                    return Err(BuildError::invalid_input("switch with invalid args"));
                };
                let data_label = raw.to_label().ok_or_else(|| {
                    BuildError::invalid_input(format!("bad switch data label {raw}"))
                })?;

                let data = match data_label {
                    Label::PackedSwitchData(id) => self
                        .method
                        .packed_switch_data()
                        .iter()
                        .find(|d| d.label_id == id),
                    Label::SparseSwitchData(id) => self
                        .method
                        .sparse_switch_data()
                        .iter()
                        .find(|d| d.label_id == id),
                    _ => {
                        return Err(BuildError::invalid_input(format!(
                            "switch operand is not a data label {raw}"
                        )))
                    }
                }
                .ok_or_else(|| {
                    BuildError::invalid_input(format!("no switch data for label: {raw}"))
                })?;

                for case in data.cases() {
                    let at = self.labels.find_label(case.label)?;
                    successors.push(Successor {
                        id: self.inst_to_block[at.index()],
                        kind: EdgeKind::Switch(case.key),
                    });
                }

                successors.push(Successor {
                    id: self.required_next_block_id(bid)?,
                    kind: EdgeKind::SwitchDefault,
                });
            } else if !(inv.is_return() || inv.instruction() == INS_THROW) {
                // No next block means this is trailing padding after a return
                if let Some(next) = self.next_block_id(bid) {
                    successors.push(Successor {
                        id: next,
                        kind: EdgeKind::FallThrough,
                    });
                }
            }

            method_blocks.push(MethodBlock {
                first: block.start,
                end: block.end,
                successors,
                inbound_edges: Vec::new(),
                outbound_edges: Vec::new(),
            });
        }

        // Finally add exception edges
        for (ins_idx, line_id) in self.method.instructions.iter().enumerate() {
            if ins_idx == 0 {
                continue;
            }

            let id = InstructionId::new(ins_idx);
            let MethodLine::Instruction(inv) = self.method.get_line(*line_id) else {
                unreachable!("instructions vec contained a non-instruction line")
            };

            if !inv.can_throw() {
                continue;
            }

            let from = self.inst_to_block[ins_idx - 1];
            for range in self.catch_ranges.iter().filter(|r| r.contains(id)) {
                let method_block = &mut method_blocks[from.index()];
                let id = self.inst_to_block[range.handler.index()];
                method_block.successors.push(Successor {
                    id,
                    kind: EdgeKind::Exception { line: range.line },
                });
            }
        }

        let blocks_rpo = reverse_post_order(&mut method_blocks);

        Ok(MethodCfg {
            regs: self.regs,
            method: self.method,
            blocks: method_blocks,
            blocks_rpo,
        })
    }

    /// The block that falls through from `id`, if there is one.
    ///
    /// The last block of a method has none. That is normal rather than
    /// malformed: baksmali emits padding `nop`s after a `return` to align the
    /// switch payloads that follow, and those land in a trailing block. Such a
    /// block is unreachable, so it never enters the reverse post order and is
    /// never analyzed.
    fn next_block_id(&self, id: usize) -> Option<BlockId> {
        let next = id + 1;
        (next < self.blocks.len()).then(|| BlockId::new(next))
    }

    /// Like [Self::next_block_id] but for edges that only a malformed method
    /// could leave dangling: a conditional or a switch cannot be the last
    /// instruction, since something has to follow for it to fall through to.
    fn required_next_block_id(&self, id: usize) -> BuildResult<BlockId> {
        self.next_block_id(id)
            .ok_or_else(|| BuildError::invalid_input("method ends with a conditional or switch"))
    }

    fn get_label_instruction(&self, raw: &RawLabel) -> BuildResult<InstructionId> {
        let label = raw
            .to_label()
            .ok_or_else(|| BuildError::invalid_input(format!("invalid raw label: {raw}")))?;

        self.labels.find_label(label)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{parse_class, Arena, Lexer, Parser};

    /// Wrap `body` in a class and build the CFG for its only method.
    fn build<'a>(arena: &'a Arena, body: &str) -> BuildResult<MethodCfg<'a>> {
        let smali = format!(
            ".class public Lcom/example/T;\n.super Ljava/lang/Object;\n\n{}\n",
            body
        );
        let lexer = Lexer::new(smali.as_bytes(), arena);
        let mut parser = Parser::new(lexer);
        let mut class = parse_class(&mut parser).expect("smali failed to parse");
        assert_eq!(class.methods.len(), 1, "expected exactly one method");
        MethodCfg::from_method(class.methods.remove(0))
    }

    fn built<'a>(arena: &'a Arena, body: &str) -> MethodCfg<'a> {
        build(arena, body).expect("cfg build failed")
    }

    /// (first, end) of every block, in order.
    fn bounds(cfg: &MethodCfg) -> Vec<(usize, usize)> {
        cfg.blocks
            .iter()
            .map(|b| (b.first.index(), b.end.index()))
            .collect()
    }

    /// Inbound edges of every block, sorted. They are recorded in dfs discovery
    /// order, which carries no meaning: phi operands are keyed by block.
    fn inbound(cfg: &MethodCfg) -> Vec<Vec<usize>> {
        cfg.blocks
            .iter()
            .map(|b| {
                let mut edges: Vec<usize> = b.inbound_edges.iter().map(|id| id.index()).collect();
                edges.sort_unstable();
                edges
            })
            .collect()
    }

    fn rpo(cfg: &MethodCfg) -> Vec<usize> {
        cfg.blocks_rpo.iter().map(|id| id.index()).collect()
    }

    /// Successors of one block as (target, kind).
    fn edges(cfg: &MethodCfg, block: usize) -> Vec<(usize, EdgeKind)> {
        cfg.blocks[block]
            .successors
            .iter()
            .map(|s| (s.id.index(), s.kind))
            .collect()
    }

    #[test]
    fn straight_line_is_one_block() {
        let arena = Arena::new();
        let cfg = built(
            &arena,
            r#".method public m()V
    .registers 2
    const/4 v0, 0x0
    const/4 v1, 0x1
    return-void
.end method"#,
        );
        assert_eq!(bounds(&cfg), vec![(0, 3)]);
        assert_eq!(edges(&cfg, 0), vec![]);
    }

    #[test]
    fn register_space_counts_both_namespaces() {
        let arena = Arena::new();
        let cfg = built(
            &arena,
            r#".method public m(II)V
    .registers 5
    const/4 v0, 0x0
    add-int/2addr v0, p2
    move v2, v0
    return-void
.end method"#,
        );
        // v0..v2 used, p2 is the highest param
        assert_eq!(
            cfg.regs,
            MethodRegSpace {
                locals: 3,
                params: 3
            }
        );
    }

    #[test]
    fn if_else_join() {
        let arena = Arena::new();
        let cfg = built(
            &arena,
            r#".method public m(I)I
    .registers 2
    if-lez p1, :cond_0
    const/4 v0, 0x1
    goto :goto_0
    :cond_0
    const/4 v0, 0x2
    :goto_0
    return v0
.end method"#,
        );
        assert_eq!(bounds(&cfg), vec![(0, 1), (1, 3), (3, 4), (4, 5)]);
        assert_eq!(
            edges(&cfg, 0),
            vec![(2, EdgeKind::Conditional), (1, EdgeKind::FallThrough)]
        );
        assert_eq!(edges(&cfg, 1), vec![(3, EdgeKind::Goto)]);
        assert_eq!(edges(&cfg, 2), vec![(3, EdgeKind::FallThrough)]);
        assert_eq!(edges(&cfg, 3), vec![]);
    }

    #[test]
    fn loop_with_nested_branches() {
        let arena = Arena::new();
        let cfg = built(
            &arena,
            r#".method public process([II)I
    .registers 8
    const/4 v0, 0x0
    const/4 v1, 0x0
    array-length v2, p1
    :goto_0
    if-ge v1, v2, :cond_0
    aget v3, p1, v1
    if-lez v3, :cond_2
    if-le v3, p2, :cond_1
    add-int/2addr v0, p2
    goto :goto_1
    :cond_1
    add-int/2addr v0, v3
    goto :goto_1
    :cond_2
    sub-int/2addr v0, v3
    :goto_1
    add-int/lit8 v1, v1, 0x1
    goto :goto_0
    :cond_0
    if-gez v0, :cond_3
    neg-int v0, v0
    goto :goto_2
    :cond_3
    const/4 v4, 0x1
    :goto_2
    return v0
.end method"#,
        );

        assert_eq!(
            bounds(&cfg),
            vec![
                (0, 3),
                (3, 4),
                (4, 6),
                (6, 7),
                (7, 9),
                (9, 11),
                (11, 12),
                (12, 14),
                (14, 15),
                (15, 17),
                (17, 18),
                (18, 19),
            ]
        );

        assert_eq!(edges(&cfg, 0), vec![(1, EdgeKind::FallThrough)]);
        assert_eq!(
            edges(&cfg, 1),
            vec![(8, EdgeKind::Conditional), (2, EdgeKind::FallThrough)]
        );
        assert_eq!(
            edges(&cfg, 2),
            vec![(6, EdgeKind::Conditional), (3, EdgeKind::FallThrough)]
        );
        assert_eq!(
            edges(&cfg, 3),
            vec![(5, EdgeKind::Conditional), (4, EdgeKind::FallThrough)]
        );
        assert_eq!(edges(&cfg, 4), vec![(7, EdgeKind::Goto)]);
        assert_eq!(edges(&cfg, 5), vec![(7, EdgeKind::Goto)]);
        assert_eq!(edges(&cfg, 6), vec![(7, EdgeKind::FallThrough)]);
        // back edge
        assert_eq!(edges(&cfg, 7), vec![(1, EdgeKind::Goto)]);
        assert_eq!(
            edges(&cfg, 8),
            vec![(10, EdgeKind::Conditional), (9, EdgeKind::FallThrough)]
        );
        assert_eq!(edges(&cfg, 9), vec![(11, EdgeKind::Goto)]);
        assert_eq!(edges(&cfg, 10), vec![(11, EdgeKind::FallThrough)]);
        assert_eq!(edges(&cfg, 11), vec![]);
    }

    #[test]
    fn rpo_visits_predecessors_first_except_across_back_edges() {
        let arena = Arena::new();
        let cfg = built(
            &arena,
            r#".method public process([II)I
    .registers 8
    const/4 v0, 0x0
    const/4 v1, 0x0
    array-length v2, p1
    :goto_0
    if-ge v1, v2, :cond_0
    aget v3, p1, v1
    if-lez v3, :cond_2
    if-le v3, p2, :cond_1
    add-int/2addr v0, p2
    goto :goto_1
    :cond_1
    add-int/2addr v0, v3
    goto :goto_1
    :cond_2
    sub-int/2addr v0, v3
    :goto_1
    add-int/lit8 v1, v1, 0x1
    goto :goto_0
    :cond_0
    if-gez v0, :cond_3
    neg-int v0, v0
    goto :goto_2
    :cond_3
    const/4 v4, 0x1
    :goto_2
    return v0
.end method"#,
        );

        let order = rpo(&cfg);
        assert_eq!(order.len(), cfg.blocks.len(), "every block is reachable");

        let position: Vec<usize> = {
            let mut pos = vec![usize::MAX; cfg.blocks.len()];
            for (at, block) in order.iter().enumerate() {
                pos[*block] = at;
            }
            pos
        };

        // B7 -> B1 is the only back edge; every other edge goes forwards.
        for (from, block) in cfg.blocks.iter().enumerate() {
            for succ in &block.successors {
                let to = succ.id.index();
                if (from, to) == (7, 1) {
                    assert!(position[to] < position[from], "B7 -> B1 is the back edge");
                } else {
                    assert!(
                        position[from] < position[to],
                        "B{from} -> B{to} should go forwards in rpo"
                    );
                }
            }
        }

        assert_eq!(
            inbound(&cfg)[1],
            vec![0, 7],
            "loop header, entry then latch"
        );
        assert_eq!(inbound(&cfg)[7], vec![4, 5, 6], "three way join");
        assert_eq!(inbound(&cfg)[11], vec![9, 10]);
    }

    #[test]
    fn handler_blocks_are_reachable_and_have_inbound_edges() {
        let arena = Arena::new();
        let cfg = built(
            &arena,
            r#".method public m()V
    .registers 2
    const/4 v0, 0x0
    :try_start_0
    invoke-static {}, Lfoo/Bar;->baz()V
    invoke-static {}, Lfoo/Bar;->qux()V
    :try_end_0
    .catch Ljava/lang/Exception; {:try_start_0 .. :try_end_0} :catch_0
    return-void
    :catch_0
    move-exception v1
    return-void
.end method"#,
        );

        // The handler is only reachable by exception edges, from the two blocks
        // preceding each covered throwing instruction.
        assert_eq!(inbound(&cfg)[4], vec![0, 1]);
        assert_eq!(rpo(&cfg).len(), cfg.blocks.len());
    }

    #[test]
    fn duplicate_edges_count_once_as_inbound() {
        let arena = Arena::new();
        let cfg = built(
            &arena,
            r#".method public m(I)I
    .registers 3
    sparse-switch p1, :sswitch_data_0
    const/4 v0, 0x0
    return v0
    :sswitch_0
    const/4 v0, 0x1
    return v0
    :sswitch_data_0
    .sparse-switch
        0x0 -> :sswitch_0
        0x64 -> :sswitch_0
    .end sparse-switch
.end method"#,
        );

        // Two switch keys target block 2, but it has one inbound edge.
        assert_eq!(cfg.blocks[0].successors.len(), 3);
        assert_eq!(inbound(&cfg)[2], vec![0]);
    }

    #[test]
    fn unreachable_blocks_are_left_out_of_rpo() {
        let arena = Arena::new();
        let cfg = built(
            &arena,
            r#".method public m()V
    .registers 1
    goto :goto_0
    const/4 v0, 0x0
    :goto_0
    return-void
.end method"#,
        );

        assert_eq!(cfg.blocks.len(), 3);
        assert_eq!(rpo(&cfg), vec![0, 2]);
        assert!(inbound(&cfg)[1].is_empty());
    }

    #[test]
    fn packed_switch_targets_come_from_the_payload() {
        let arena = Arena::new();
        let cfg = built(
            &arena,
            r#".method public m(I)I
    .registers 3
    packed-switch p1, :pswitch_data_0
    const/4 v0, 0x0
    return v0
    :pswitch_0
    const/4 v0, 0x1
    return v0
    :pswitch_1
    const/4 v0, 0x2
    return v0
    :pswitch_data_0
    .packed-switch 0x1
        :pswitch_0
        :pswitch_1
    .end packed-switch
.end method"#,
        );
        assert_eq!(bounds(&cfg), vec![(0, 1), (1, 3), (3, 5), (5, 7)]);
        assert_eq!(
            edges(&cfg, 0),
            vec![
                (2, EdgeKind::Switch(1)),
                (3, EdgeKind::Switch(2)),
                (1, EdgeKind::SwitchDefault),
            ]
        );
    }

    #[test]
    fn sparse_switch_keeps_keys() {
        let arena = Arena::new();
        let cfg = built(
            &arena,
            r#".method public m(I)I
    .registers 3
    sparse-switch p1, :sswitch_data_0
    const/4 v0, 0x0
    return v0
    :sswitch_0
    const/4 v0, 0x1
    return v0
    :sswitch_data_0
    .sparse-switch
        0x0 -> :sswitch_0
        0x64 -> :sswitch_0
    .end sparse-switch
.end method"#,
        );
        // Two keys, same target: both edges are kept, since the keys differ.
        assert_eq!(
            edges(&cfg, 0),
            vec![
                (2, EdgeKind::Switch(0)),
                (2, EdgeKind::Switch(100)),
                (1, EdgeKind::SwitchDefault),
            ]
        );
    }

    #[test]
    fn covered_throwing_instructions_split_blocks_and_add_handler_edges() {
        let arena = Arena::new();
        let cfg = built(
            &arena,
            r#".method public m()V
    .registers 2
    const/4 v0, 0x0
    :try_start_0
    invoke-static {}, Lfoo/Bar;->baz()V
    invoke-static {}, Lfoo/Bar;->qux()V
    :try_end_0
    .catch Ljava/lang/Exception; {:try_start_0 .. :try_end_0} :catch_0
    return-void
    :catch_0
    move-exception v1
    return-void
.end method"#,
        );

        // Each covered throwing instruction is a leader.
        assert_eq!(bounds(&cfg), vec![(0, 1), (1, 2), (2, 3), (3, 4), (4, 6)]);

        let handler_line = match edges(&cfg, 0)[1].1 {
            EdgeKind::Exception { line } => line,
            other => panic!("expected an exception edge, got {other:?}"),
        };

        // The edge leaves the block *before* the throwing instruction, so it
        // carries the state from before the throw.
        assert_eq!(
            edges(&cfg, 0),
            vec![
                (1, EdgeKind::FallThrough),
                (4, EdgeKind::Exception { line: handler_line }),
            ]
        );
        assert_eq!(
            edges(&cfg, 1),
            vec![
                (2, EdgeKind::FallThrough),
                (4, EdgeKind::Exception { line: handler_line }),
            ]
        );
        // The block containing the last covered instruction has no handler edge
        // of its own.
        assert_eq!(edges(&cfg, 2), vec![(3, EdgeKind::FallThrough)]);
        assert_eq!(edges(&cfg, 3), vec![]);
        assert_eq!(edges(&cfg, 4), vec![]);
    }

    #[test]
    fn try_end_label_is_exclusive() {
        let arena = Arena::new();
        let cfg = built(
            &arena,
            r#".method public m()V
    .registers 1
    const/4 v0, 0x0
    :try_start_0
    invoke-static {}, Lfoo/Bar;->baz()V
    :try_end_0
    .catch Ljava/lang/Exception; {:try_start_0 .. :try_end_0} :catch_0
    invoke-static {}, Lfoo/Bar;->qux()V
    return-void
    :catch_0
    move-exception v0
    return-void
.end method"#,
        );
        assert_eq!(bounds(&cfg), vec![(0, 1), (1, 2), (2, 4), (4, 6)]);

        // Instruction 1 is inside the range, so block 0 gets a handler edge.
        assert_eq!(edges(&cfg, 0).len(), 2);
        assert!(matches!(edges(&cfg, 0)[1].1, EdgeKind::Exception { .. }));

        // Instruction 2 sits exactly at :try_end_0 and is therefore *not*
        // covered, so block 1 has only its fallthrough.
        assert_eq!(edges(&cfg, 1), vec![(2, EdgeKind::FallThrough)]);
    }

    #[test]
    fn uncovered_throwing_instructions_do_not_split() {
        let arena = Arena::new();
        let cfg = built(
            &arena,
            r#".method public m(Lfoo/Bar;)V
    .registers 2
    invoke-static {}, Lfoo/Bar;->baz()V
    invoke-static {}, Lfoo/Bar;->qux()V
    return-void
.end method"#,
        );
        assert_eq!(bounds(&cfg), vec![(0, 3)]);
        assert_eq!(edges(&cfg, 0), vec![]);
    }

    #[test]
    fn nested_ranges_produce_one_edge_per_handler_in_source_order() {
        let arena = Arena::new();
        let cfg = built(
            &arena,
            r#".method public m()V
    .registers 2
    const/4 v0, 0x0
    :try_start_0
    :try_start_1
    invoke-static {}, Lfoo/Bar;->baz()V
    :try_end_1
    .catch Ljava/lang/RuntimeException; {:try_start_1 .. :try_end_1} :catch_0
    :try_end_0
    .catchall {:try_start_0 .. :try_end_0} :catchall_0
    return-void
    :catch_0
    move-exception v1
    return-void
    :catchall_0
    move-exception v1
    throw v1
.end method"#,
        );

        let out = edges(&cfg, 0);
        assert_eq!(out.len(), 3, "fallthrough plus two handlers: {out:?}");
        assert_eq!(out[0].1, EdgeKind::FallThrough);
        assert!(matches!(out[1].1, EdgeKind::Exception { .. }));
        assert!(matches!(out[2].1, EdgeKind::Exception { .. }));
        // .catch is declared before .catchall, and handler order is preserved.
        assert_ne!(out[1].0, out[2].0);
    }

    #[test]
    fn abstract_method_has_no_blocks() {
        let arena = Arena::new();
        let cfg = built(
            &arena,
            r#".method public abstract m()V
.end method"#,
        );
        assert!(cfg.blocks.is_empty());
        assert!(cfg.method.instructions.is_empty());
    }

    #[test]
    fn missing_branch_target_is_an_error() {
        let arena = Arena::new();
        let err = build(
            &arena,
            r#".method public m()V
    .registers 1
    goto :goto_9
    return-void
.end method"#,
        );
        assert!(err.is_err(), "expected a missing-label error");
    }

    #[test]
    fn reversed_catch_range_is_an_error() {
        let arena = Arena::new();
        let err = build(
            &arena,
            r#".method public m()V
    .registers 2
    :try_end_0
    const/4 v0, 0x0
    :try_start_0
    return-void
    .catch Ljava/lang/Exception; {:try_start_0 .. :try_end_0} :catch_0
    :catch_0
    move-exception v1
    return-void
.end method"#,
        );
        assert!(err.is_err(), "expected a reversed-range error");
    }

    /// baksmali pads with a `nop` after a `return` so the switch payload that
    /// follows is aligned. That nop is unreachable and falls off the end of the
    /// method, which is normal rather than malformed.
    #[test]
    fn trailing_padding_after_a_return_has_no_successor() {
        let arena = Arena::new();
        let cfg = built(
            &arena,
            r#".method public m(I)I
    .registers 3
    packed-switch p1, :pswitch_data_0
    const/4 v0, 0x0
    return v0
    :pswitch_0
    const/4 v0, 0x1
    return v0
    nop
    :pswitch_data_0
    .packed-switch 0x1
        :pswitch_0
    .end packed-switch
.end method"#,
        );

        let last = cfg.blocks.len() - 1;
        assert_eq!(
            bounds(&cfg)[last],
            (5, 6),
            "the padding nop is its own block"
        );
        assert_eq!(edges(&cfg, last), vec![], "and it falls off the end");
        assert!(
            !rpo(&cfg).contains(&last),
            "it is unreachable so it is never analyzed"
        );
    }

    #[test]
    fn a_trailing_conditional_is_an_error() {
        let arena = Arena::new();
        let err = build(
            &arena,
            r#".method public m(I)V
    .registers 2
    if-lez p1, :cond_0
    :cond_0
    if-lez p1, :cond_0
.end method"#,
        );
        assert!(err.is_err(), "a conditional has nothing to fall through to");
    }
}
