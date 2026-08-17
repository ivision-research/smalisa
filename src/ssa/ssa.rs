use std::mem;
use std::ops::Deref;

use std::collections::HashMap;

use crate::{
    instructions::{InvArgs, Invocation},
    AccessFlag, Field, Literal, MethodLine, Register, SmaliClassName,
};

use crate::cfg::{
    BlockId, BuildResult, InstructionId, Method, MethodCfg, MethodRegSpace, VariableId,
};
use crate::index::index_type;

index_type!(
    ValueId,
    "An opaque reference to an SSA value, used as an index into a value vector"
);

index_type!(
    PhiId,
    "An opaque reference to a phi, used as an index into a phi vector"
);

index_type!(
    ConstId,
    "An opaque reference to a constant value, used as an index into a const vector"
);

// TODO: I've done no optimization pass on this file, just a "get it working" pass.

/// A class with all of its methods in SSA form.
#[cfg_attr(debug_assertions, derive(Debug))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "yoke", derive(yoke::Yokeable))]
pub struct SsaClass<'a> {
    pub access: AccessFlag,
    #[cfg_attr(feature = "serde", serde(borrow))]
    pub name: &'a SmaliClassName,
    #[cfg_attr(feature = "serde", serde(borrow))]
    pub parent: &'a SmaliClassName,
    pub interfaces: Vec<&'a SmaliClassName>,
    pub fields: Vec<Field<'a>>,

    pub methods: Vec<SsaMethod<'a>>,
}

impl<'a> SsaClass<'a> {
    /// Build the SSA form for every method in `class`.
    pub fn from_class(class: crate::Class<'a>) -> BuildResult<Self> {
        let crate::Class {
            methods,
            access,
            name,
            parent,
            interfaces,
            fields,
            ..
        } = class;
        let methods = methods
            .into_iter()
            .map(|method| MethodCfg::from_method(method).and_then(SsaMethod::build))
            .collect::<BuildResult<Vec<_>>>()?;

        Ok(Self {
            access,
            name,
            parent,
            interfaces,
            fields,
            methods,
        })
    }
}

/// A single method in SSA form
///
/// Note that all fields are public so you can do whatever you please but they do not present an
/// intuitive interface. This type can deref to a [crate::Method]
#[cfg_attr(debug_assertions, derive(Debug))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "yoke", derive(yoke::Yokeable))]
pub struct SsaMethod<'a> {
    #[cfg_attr(feature = "serde", serde(borrow))]
    pub method: Method<'a>,
    pub regs: MethodRegSpace,
    pub blocks: Vec<SsaBlock>,
    pub values: Vec<Value>,
    /// All [Phi] values. Note that this may contain removed [Phi]s, see the comment on
    /// [Phi::replaced_by] for more
    pub phis: Vec<Phi>,
    /// All constant values in the method, deduplicated
    pub consts: Vec<Literal<'a>>,
    /// Indexed by [InstructionId]. The value each use resolved to, parallel to the instruction's
    /// `read_registers()` in most cases with a potential extra value at the end for the result
    /// register
    pub inst_uses: Vec<Vec<ValueId>>,
    /// Indexed by [InstructionId]. The values the instruction defines, parallel to its
    /// `written_registers()` in most cases with a potential extra value at the end for the result
    /// register
    pub inst_defs: Vec<Vec<ValueId>>,
    /// Reachable blocks in reverse post order
    pub blocks_rpo: Vec<BlockId>,
    /// Indexed by [ValueId]. Everything that consumes the value, deduplicated.
    /// Removed phis are not listed.
    pub value_users: Vec<Vec<ValueUser>>,
}

impl<'a> Deref for SsaMethod<'a> {
    type Target = crate::Method<'a>;
    fn deref(&self) -> &Self::Target {
        &self.method.method
    }
}

impl<'a> SsaMethod<'a> {
    fn empty(method: MethodCfg<'a>) -> Self {
        let MethodCfg { method, regs, .. } = method;
        Self {
            method,
            regs,
            blocks: Vec::new(),
            values: Vec::new(),
            phis: Vec::new(),
            consts: Vec::new(),
            inst_uses: Vec::new(),
            inst_defs: Vec::new(),
            blocks_rpo: Vec::new(),
            value_users: Vec::new(),
        }
    }

    fn build(cfg: MethodCfg<'a>) -> BuildResult<Self> {
        SsaBuilder::new().run(cfg)
    }

    /// Retrieve the [BlockId] for the entry block
    pub fn entry_block(&self) -> BlockId {
        BlockId::new(0)
    }

    /// Retrieve the [SsaBlock] corresponding to the [BlockId]
    pub fn block(&self, id: BlockId) -> &SsaBlock {
        &self.blocks[id.index()]
    }

    /// Return an [Iterator] over all [SsaBlock]s with their associated [BlockId]
    pub fn blocks(&self) -> impl Iterator<Item = (BlockId, &SsaBlock)> {
        self.blocks
            .iter()
            .enumerate()
            .map(|(idx, block)| (BlockId::new(idx), block))
    }

    /// Reachable blocks in the order the algorithm filled them. Blocks that are
    /// unreachable from the entry are not listed and were never analyzed.
    pub fn reverse_post_order(&self) -> impl Iterator<Item = BlockId> {
        self.blocks_rpo.iter().copied()
    }

    /// Return an [Iterator] for all [InstructionId]s in the provided block
    pub fn block_instructions(&self, id: BlockId) -> impl Iterator<Item = InstructionId> + use<> {
        let block = self.block(id);
        (block.first.index()..block.end.index()).map(InstructionId::new)
    }

    /// Convert an [InstructionId] to the [Invocation] associated with it
    ///
    /// Note that this throws on invalid [InstructionId]s
    pub fn instruction(&self, id: InstructionId) -> &Invocation<'a> {
        match self.method.line_for_instruction(id) {
            MethodLine::Instruction(inv) => inv,
            other => unreachable!("instruction {id:?} is not an instruction line: {other:?}"),
        }
    }

    /// The values this instruction reads, in `read_registers()` order, followed
    /// by the implicit result register if the instruction consumes one.
    pub fn uses(&self, id: InstructionId) -> &[ValueId] {
        &self.inst_uses[id.index()]
    }

    /// The values this instruction defines, in `written_registers()` order,
    /// followed by the implicit result register if the instruction writes one.
    pub fn defs(&self, id: InstructionId) -> &[ValueId] {
        &self.inst_defs[id.index()]
    }

    /// Retrieve the [Value] for the provided [ValueId]
    pub fn value(&self, id: ValueId) -> &Value {
        &self.values[id.index()]
    }

    /// Retrieve the [Phi] for the given [PhiId]
    pub fn phi(&self, id: PhiId) -> &Phi {
        &self.phis[id.index()]
    }

    /// Retrieve the [Literal] for the given [ConstId]
    pub fn constant(&self, id: ConstId) -> &Literal<'a> {
        &self.consts[id.index()]
    }

    /// Retrieve an iterator over all [Literal]s
    pub fn constants(&self) -> impl Iterator<Item = &Literal<'a>> {
        self.consts.iter()
    }

    /// Every phi that survived trivial phi removal. Prefer this over iterating
    /// [Self::phis], which also holds the removed ones so ids stay stable.
    pub fn live_phis(&self) -> impl Iterator<Item = (PhiId, &Phi)> {
        self.phis
            .iter()
            .enumerate()
            .filter(|(_, phi)| phi.is_live())
            .map(|(idx, phi)| (PhiId::new(idx), phi))
    }

    /// Everything that consumes `value`. This is the direction dataflow travels:
    /// from a definition towards the instructions that read it.
    pub fn users(&self, value: ValueId) -> &[ValueUser] {
        &self.value_users[value.index()]
    }

    /// The variable a smali register maps to in this method
    pub fn variable(&self, reg: Register) -> VariableId {
        self.regs.variable(reg)
    }
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SsaBlock {
    /// Instruction index where this block starts
    pub first: InstructionId,
    /// Instruction index where this block ends, exclusive
    pub end: InstructionId,
    /// Blocks with an outgoing edge into this one, deduplicated
    pub inbound_edges: Vec<BlockId>,
    /// Phis at the head of this block
    pub phis: Vec<PhiId>,
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Value {
    Phi(PhiId),
    /// Result of a real instruction
    Instruction(InstructionId),
    /// An incoming method parameter, defined on entry
    Param(VariableId),
    Const(ConstId),
    /// Read of a variable with no definition on some path
    Undef,
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PhiOperand {
    pub block: BlockId,
    pub value: ValueId,
}

impl PhiOperand {
    fn new(block: BlockId, value: ValueId) -> Self {
        Self { block, value }
    }
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Phi {
    pub block: BlockId,
    pub variable: VariableId,
    pub value: ValueId,
    pub operands: Vec<PhiOperand>,
    /// This is Some(..) when the phi was trivial and replaced. This survives the creation of the
    /// SSA form just to keep [PhiId]s stable. [SsaMethod] provides helpers for iterating [Phi]s if
    /// needed
    pub replaced_by: Option<ValueId>,
}

impl Phi {
    pub fn is_live(&self) -> bool {
        self.replaced_by.is_none()
    }
}

/// Something that consumes a [ValueId].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ValueUser {
    Instruction(InstructionId),
    Phi(PhiId),
}

/// The 64 bit pattern a wide constant puts in a register pair. Returns None for literals that
/// can't occupy a pair, which a `const-wide*` should never carry.
fn wide_bits(literal: Literal<'_>) -> Option<u64> {
    let bits = match literal {
        Literal::Long(num) => num as u64,
        Literal::Int(num) => i64::from(num) as u64,
        Literal::Short(num) => i64::from(num) as u64,
        Literal::Byte(num) => i64::from(num) as u64,
        Literal::Double(num) => num.to_bits(),
        Literal::Float(num) => f64::from(num).to_bits(),
        _ => return None,
    };
    Some(bits)
}

/// The two 32 bit words of a wide constant, low word first
fn wide_words(bits: u64) -> [i32; 2] {
    let bytes = bits.to_le_bytes();
    let (low, high) = bytes.split_at(4);
    [
        i32::from_le_bytes(low.try_into().expect("4 bytes")),
        i32::from_le_bytes(high.try_into().expect("4 bytes")),
    ]
}

/// Whether a block has seen all of its inbound edges.
///
/// The pending phis only exist while unsealed, so they live in the variant
/// rather than beside a boolean.
enum Sealed {
    No { incomplete_phis: Vec<PhiId> },
    Yes,
}

impl Sealed {
    fn is_sealed(&self) -> bool {
        matches!(self, Self::Yes)
    }

    fn unsealed() -> Self {
        Self::No {
            incomplete_phis: Vec::new(),
        }
    }
}

impl Default for Sealed {
    fn default() -> Self {
        Self::unsealed()
    }
}

struct BlockState {
    sealed: Sealed,
    filled: bool,
    /// Counts down as inbound blocks are filled; the block seals at zero
    unfilled_edges: u32,
}

/// Drives the Braun algorithm over a single [MethodCfg].
struct SsaBuilder<'a> {
    blocks: Vec<BlockState>,
    /// current_def[variable][block]
    current_def: Vec<Vec<Option<ValueId>>>,

    values: Vec<Value>,
    phis: Vec<Phi>,
    consts: Vec<Literal<'a>>,
    /// Maps a literal to the value holding it so identical constants share one value
    const_values: HashMap<Literal<'a>, ValueId>,
    /// phi_users[phi] is every phi taking that value as an operand
    phi_users: Vec<Vec<PhiId>>,

    /// See [SsaMethod::inst_uses]
    inst_uses: Vec<Vec<ValueId>>,
    /// See [SsaMethod::inst_defs]
    inst_defs: Vec<Vec<ValueId>>,
}

impl<'a> SsaBuilder<'a> {
    fn new() -> Self {
        Self {
            blocks: Vec::new(),
            values: Vec::new(),
            consts: Vec::new(),
            const_values: HashMap::new(),
            current_def: Vec::new(),
            phis: Vec::new(),
            phi_users: Vec::new(),
            inst_uses: Vec::new(),
            inst_defs: Vec::new(),
        }
    }

    fn get_block_state(&self, id: BlockId) -> &BlockState {
        &self.blocks[id.index()]
    }

    fn get_block_state_mut(&mut self, id: BlockId) -> &mut BlockState {
        &mut self.blocks[id.index()]
    }

    fn run(mut self, cfg: MethodCfg<'a>) -> BuildResult<SsaMethod<'a>> {
        if cfg.blocks.len() == 0 {
            return Ok(SsaMethod::empty(cfg));
        }

        let mut block_id = 0;
        self.blocks.resize_with(cfg.blocks.len(), || {
            let state = BlockState {
                sealed: Sealed::unsealed(),
                filled: false,
                unfilled_edges: cfg.get_block(BlockId::new(block_id)).inbound_edges.len() as u32,
            };
            block_id += 1;
            state
        });

        // Make sure all vectors are sized correctly for the method since we index into them.
        let n_ins = cfg.method.instructions.len();
        self.inst_defs.resize(n_ins, Vec::new());
        self.inst_uses.resize(n_ins, Vec::new());
        self.current_def
            .resize(cfg.regs.count(), vec![None; cfg.blocks.len()]);

        // Seal the first block if we can, all other blocks will (I think?) by definition have at
        // least one unfilled edge
        if self.blocks[0].unfilled_edges == 0 {
            self.seal_block(&cfg, BlockId::new(0));
        }

        // Incoming parameters are defined on entry, otherwise reading one finds no definition
        // anywhere and resolves to Undef.
        let entry = BlockId::new(0);
        for var in cfg.regs.param_variables() {
            let value = self.new_value(Value::Param(var));
            self.write_variable(var, entry, value);
        }

        for i in 0..cfg.blocks_rpo.len() {
            let block_id = cfg.blocks_rpo[i];
            self.fill_block(block_id, &cfg)?;
            self.on_filled(&cfg, block_id);
        }

        Ok(self.finish(cfg))
    }

    fn finish(mut self, cfg: MethodCfg<'a>) -> SsaMethod<'a> {
        // Do a last resolve path so everything we hand back is resolved
        for use_idx in 0..self.inst_uses.len() {
            let values = &self.inst_uses[use_idx];
            for value_idx in 0..values.len() {
                let value_id = self.inst_uses[use_idx][value_idx];
                let value = self.resolve(value_id);
                self.inst_uses[use_idx][value_idx] = value;
            }
        }

        // Resolve phi operands as well
        for phi_idx in 0..self.phis.len() {
            let phi = &self.phis[phi_idx];
            if phi.replaced_by.is_some() {
                continue;
            }
            for op_idx in 0..phi.operands.len() {
                let value = self.resolve(self.phis[phi_idx].operands[op_idx].value);
                self.phis[phi_idx].operands[op_idx].value = value;
            }
        }

        let MethodCfg {
            method,
            regs,
            blocks: method_blocks,
            blocks_rpo,
        } = cfg;

        let mut blocks = Vec::with_capacity(self.blocks.len());
        for block in method_blocks {
            blocks.push(SsaBlock {
                first: block.first,
                end: block.end,
                inbound_edges: block.inbound_edges,
                phis: Vec::new(),
            })
        }

        for (idx, phi) in self.phis.iter().enumerate() {
            if phi.replaced_by.is_some() {
                continue;
            }
            blocks[phi.block.index()].phis.push(PhiId::new(idx));
        }

        let Self {
            phis,
            consts,
            inst_uses,
            inst_defs,
            values,
            ..
        } = self;

        // Reverse index from a value to everything consuming it. Built here because
        // it has to skip removed phis, which callers can't easily do themselves.
        let mut value_users: Vec<Vec<ValueUser>> = vec![Vec::new(); values.len()];
        let mut add_user = |value: ValueId, user: ValueUser| {
            let users = &mut value_users[value.index()];
            if !users.contains(&user) {
                users.push(user);
            }
        };

        for (idx, uses) in inst_uses.iter().enumerate() {
            let user = ValueUser::Instruction(InstructionId::new(idx));
            for value in uses {
                add_user(*value, user);
            }
        }
        for (idx, phi) in phis.iter().enumerate() {
            if !phi.is_live() {
                continue;
            }
            let user = ValueUser::Phi(PhiId::new(idx));
            for operand in &phi.operands {
                add_user(operand.value, user);
            }
        }

        SsaMethod {
            method,
            regs,
            blocks,
            values,
            consts,
            phis,
            inst_uses,
            inst_defs,
            blocks_rpo,
            value_users,
        }
    }

    fn do_simple_move_propagation(
        &mut self,
        cfg: &MethodCfg<'a>,
        block_id: BlockId,
        inv: &Invocation,
    ) -> bool {
        // For simple move instructions, we still perform the read, but we don't push anything to
        // `inst_uses` or `inst_defs` and we simply write the value we read instead of a new value.
        // This should almost always return true, but as a defensive measure for me potentially not
        // knowing as much about dex as I should we perform the length check.

        let uses_regs = inv.read_registers();
        let defs_regs = inv.written_registers();

        if uses_regs.len() != defs_regs.len() || uses_regs.len() > 2 || uses_regs.len() == 0 {
            return false;
        }

        let mut read_vars = [ValueId::default(); 2];
        let mut idx = 0;

        // We do this in two separate loops because it is possible to have a case like:
        //
        // move-wide v1, v0 which has an overlapping register, v1. I'm pretty sure the reads are all
        // supposed to happen before the writes for an instruction like this.
        for reg in uses_regs {
            let var = cfg.regs.variable(reg);
            read_vars[idx] = self.read_variable(cfg, var, block_id);
            idx += 1;
        }

        idx = 0;

        for reg in defs_regs {
            let var = cfg.regs.variable(reg);
            self.write_variable(var, block_id, read_vars[idx]);
            idx += 1;
        }

        true
    }

    fn do_result_move_propagation(
        &mut self,
        cfg: &MethodCfg<'a>,
        block_id: BlockId,
        inv: &Invocation,
    ) -> bool {
        // Very similar to the normal move propagation, but we use the pseudo result registers
        // instead.

        let vars = [VariableId::RESULT_FIRST, VariableId::RESULT_SECOND];
        let nused = if inv.pair_first() { 2 } else { 1 };

        let defs_regs = inv.written_registers();

        if defs_regs.len() != nused {
            return false;
        }

        // We don't need the two loops here because we're using the psuedo registers
        for (idx, reg) in defs_regs.iter().enumerate() {
            let var = cfg.regs.variable(reg);
            let val = self.read_variable(cfg, vars[idx], block_id);
            self.write_variable(var, block_id, val);
        }

        true
    }

    /// Fold a const instruction into the value it puts in its register(s). Returns false when the
    /// literal can't be parsed, leaving the instruction to be treated as an ordinary definition.
    fn do_const_instruction(
        &mut self,
        cfg: &MethodCfg,
        block_id: BlockId,
        inv: &Invocation<'a>,
    ) -> bool {
        let InvArgs::OneRegLiteral(_, raw) = inv.args() else {
            return false;
        };
        let Some(literal) = raw.to_literal() else {
            return false;
        };

        if inv.pair_first() {
            return self.write_wide_const(cfg, block_id, inv, literal);
        }

        for reg in inv.written_registers() {
            let var = cfg.regs.variable(reg);
            let value_id = self.new_const(literal);
            self.write_variable(var, block_id, value_id);
        }
        true
    }

    /// Fold a `const-wide*` into the two 32 bit words its register pair holds. Wide values are
    /// stored little endian, so the lower numbered register takes the low word.
    fn write_wide_const(
        &mut self,
        cfg: &MethodCfg,
        block_id: BlockId,
        inv: &Invocation<'a>,
        literal: Literal<'a>,
    ) -> bool {
        let Some(bits) = wide_bits(literal) else {
            return false;
        };

        let regs = inv.written_registers();
        if regs.len() != 2 {
            return false;
        }

        for (reg, word) in regs.into_iter().zip(wide_words(bits)) {
            let var = cfg.regs.variable(reg);
            let value_id = self.new_const(Literal::Int(word));
            self.write_variable(var, block_id, value_id);
        }
        true
    }

    /// Used to determine if a call is a wide return, this changes how we set the psuedo result
    /// registers on calls. The only way to figure this out is to look at the return type of the
    /// call.
    fn is_wide_return(inv: &Invocation) -> bool {
        let ret_type = match inv.args() {
            InvArgs::VarRegArray(_, ty) => ty,
            InvArgs::VarRegMethod(_, mref) => &mref.return_type,
            InvArgs::Polymorphic(_, _, _, ty) => ty,
            _ => return false,
        };

        matches!(
            ret_type,
            crate::Type::Primitive(crate::Primitive::Long | crate::Primitive::Double, 0)
        )
    }

    fn fill_block(&mut self, block_id: BlockId, cfg: &MethodCfg<'a>) -> BuildResult<()> {
        // To fill a block we loop through every instruction and find what it uses and defs.
        for (ins_id, inv) in cfg.instructions_for_block(block_id) {
            // Collapse `move*` variants of interest during construction, essentially doing the copy
            // propagation optimization here.
            if inv.is_move() && self.do_simple_move_propagation(cfg, block_id, inv) {
                continue;
            } else if inv.is_move_result() && self.do_result_move_propagation(cfg, block_id, inv) {
                continue;
            }

            // Collapse const instructions to their value directly instead of as an instruction
            // reference
            if inv.uses_const() && self.do_const_instruction(cfg, block_id, inv) {
                continue;
            }

            // All registers that are used by the instruction need to be read and stored in the
            // inst_uses container
            for used in inv.read_registers() {
                let var = cfg.regs.variable(used);
                let value = self.read_variable(cfg, var, block_id);
                self.get_uses_vec_mut(ins_id).push(value);
            }

            if inv.reads_result() {
                if inv.pair_first() {
                    let value = self.read_variable(cfg, VariableId::RESULT_FIRST, block_id);
                    self.get_uses_vec_mut(ins_id).push(value);
                    let value = self.read_variable(cfg, VariableId::RESULT_SECOND, block_id);
                    self.get_uses_vec_mut(ins_id).push(value);
                } else {
                    let value = self.read_variable(cfg, VariableId::RESULT, block_id);
                    self.get_uses_vec_mut(ins_id).push(value);
                }
            }

            // All registers that are def by the instruction need to be written and stored in the
            // inst_defs container
            for def in inv.written_registers() {
                let var = cfg.regs.variable(def);
                let value = Value::Instruction(ins_id);
                let value_id = self.new_value(value);
                self.write_variable(var, block_id, value_id);
                self.get_defs_vec_mut(ins_id).push(value_id);
            }

            if inv.writes_result() {
                if Self::is_wide_return(inv) {
                    let value = Value::Instruction(ins_id);
                    let value_id = self.new_value(value);
                    self.write_variable(VariableId::RESULT_FIRST, block_id, value_id);
                    self.get_defs_vec_mut(ins_id).push(value_id);
                    let value = Value::Instruction(ins_id);
                    let value_id = self.new_value(value);
                    self.write_variable(VariableId::RESULT_SECOND, block_id, value_id);
                    self.get_defs_vec_mut(ins_id).push(value_id);
                } else {
                    let value = Value::Instruction(ins_id);
                    let value_id = self.new_value(value);
                    self.write_variable(VariableId::RESULT, block_id, value_id);
                    self.get_defs_vec_mut(ins_id).push(value_id);
                }
            }
        }

        self.get_block_state_mut(block_id).filled = true;
        Ok(())
    }

    fn get_defs_vec_mut(&mut self, ins_id: InstructionId) -> &mut Vec<ValueId> {
        &mut self.inst_defs[ins_id.index()]
    }

    fn get_uses_vec_mut(&mut self, ins_id: InstructionId) -> &mut Vec<ValueId> {
        &mut self.inst_uses[ins_id.index()]
    }

    fn write_variable(&mut self, var: VariableId, block: BlockId, value: ValueId) {
        self.current_def[var.index()][block.index()] = Some(value);
    }

    /// Wrapper for indexing into current_def with type safe indices
    fn get_current_def(&self, var: VariableId, block: BlockId) -> Option<ValueId> {
        self.current_def[var.index()][block.index()]
    }

    fn read_variable(&mut self, cfg: &MethodCfg<'a>, var: VariableId, block: BlockId) -> ValueId {
        // If a local value exists return that
        if let Some(val) = self.get_current_def(var, block) {
            return self.resolve(val);
        }
        self.read_variable_recursive(cfg, var, block)
    }

    fn read_variable_recursive(
        &mut self,
        cfg: &MethodCfg<'a>,
        var: VariableId,
        block_id: BlockId,
    ) -> ValueId {
        if let Some((_, value)) = self.maybe_add_incomplete_phi(block_id, var) {
            self.write_variable(var, block_id, value);
            return value;
        }

        let block = cfg.get_block(block_id);
        if block.inbound_edges.len() == 1 {
            let value = self.read_variable(cfg, var, block.inbound_edges[0]);
            self.write_variable(var, block_id, value);
            return value;
        }

        // Place an operandless Phi to break potential cycles
        let (phi, value) = self.new_phi(block_id, var);
        self.write_variable(var, block_id, value);
        let value = self.add_phi_operands(cfg, var, block_id, phi);
        self.write_variable(var, block_id, value);
        value
    }

    fn block_is_sealed(&self, block_id: BlockId) -> bool {
        self.get_block_state(block_id).sealed.is_sealed()
    }

    /// Add an incomplete phi to an unsealed block
    ///
    /// This function will return None if the block is sealed and Some if it isn't. The return value
    /// should be used as the sentinel for "sealed or not" in read_variable_recursive.
    ///
    /// This function was needed to remove an awkward mutable borrow issue.
    fn maybe_add_incomplete_phi(
        &mut self,
        block_id: BlockId,
        var: VariableId,
    ) -> Option<(PhiId, ValueId)> {
        if self.block_is_sealed(block_id) {
            return None;
        }
        let (phi, value) = self.new_phi(block_id, var);
        match &mut self.get_block_state_mut(block_id).sealed {
            Sealed::No { incomplete_phis } => incomplete_phis.push(phi),
            Sealed::Yes => unreachable!("block can't become sealed during this call"),
        }

        Some((phi, value))
    }

    fn add_phi_operands(
        &mut self,
        cfg: &MethodCfg<'a>,
        var: VariableId,
        block_id: BlockId,
        phi_id: PhiId,
    ) -> ValueId {
        let block = cfg.get_block(block_id);

        for &inbound in &block.inbound_edges {
            let value = self.read_variable(cfg, var, inbound);
            if let Value::Phi(phi) = self.values[value.index()] {
                self.phi_users[phi.index()].push(phi_id);
            }
            let phi = self.get_phi_mut(phi_id);
            phi.operands.push(PhiOperand::new(inbound, value));
        }

        self.try_remove_trivial_phi(phi_id)
    }

    fn try_remove_trivial_phi(&mut self, phi_id: PhiId) -> ValueId {
        let mut same: Option<ValueId> = None;
        let phi = self.get_phi(phi_id);

        for op in &phi.operands {
            // Make sure to resolve the operand's value because there could be a chain here
            let op_val = self.resolve(op.value);

            // Is it unique or a self reference?
            if same == Some(op_val) || op_val == phi.value {
                continue;
            }
            // Merges at least two values so non-trivial
            if same.is_some() {
                return phi.value;
            }
            same = Some(op_val);
        }

        let value = match same {
            None => self.new_value(Value::Undef),
            Some(v) => v,
        };

        self.get_phi_mut(phi_id).replaced_by = Some(value);

        // Take here to deal with borrowing issues
        let users = mem::take(&mut self.phi_users[phi_id.index()]);

        for user in users {
            if user == phi_id {
                // skip self
                continue;
            }
            if self.get_phi(user).replaced_by.is_some() {
                // already replaced
                continue;
            }
            self.try_remove_trivial_phi(user);
        }
        value
    }

    fn get_phi(&self, phi_id: PhiId) -> &Phi {
        &self.phis[phi_id.index()]
    }

    fn get_phi_mut(&mut self, phi_id: PhiId) -> &mut Phi {
        &mut self.phis[phi_id.index()]
    }

    fn get_value(&self, value_id: ValueId) -> &Value {
        &self.values[value_id.index()]
    }

    fn on_filled(&mut self, cfg: &MethodCfg<'a>, block_id: BlockId) {
        let block = cfg.get_block(block_id);
        for outbound in block.unique_outbound_block_ids() {
            let state = self.get_block_state_mut(outbound);
            debug_assert!(state.unfilled_edges > 0);
            state.unfilled_edges -= 1;
            if state.unfilled_edges == 0 {
                self.seal_block(cfg, outbound);
            }
        }
    }

    fn seal_block(&mut self, cfg: &MethodCfg<'a>, block_id: BlockId) {
        let block = self.get_block_state_mut(block_id);
        let Sealed::No { incomplete_phis } = mem::replace(&mut block.sealed, Sealed::Yes) else {
            return;
        };

        for phi_id in incomplete_phis {
            let phi = self.get_phi(phi_id);
            let variable = phi.variable;
            let value = self.add_phi_operands(cfg, variable, block_id, phi_id);
            self.write_variable(variable, block_id, value);
        }
    }

    fn new_value(&mut self, def: Value) -> ValueId {
        let value_id = ValueId::new(self.values.len());
        self.values.push(def);
        value_id
    }

    /// Values for constants are shared, so a literal already seen in this method resolves to the
    /// value that was created for it rather than a new one.
    fn new_const(&mut self, literal: Literal<'a>) -> ValueId {
        if let Some(value_id) = self.const_values.get(&literal) {
            return *value_id;
        }
        let const_id = ConstId::new(self.consts.len());
        let value_id = self.new_value(Value::Const(const_id));
        self.consts.push(literal);
        self.const_values.insert(literal, value_id);
        value_id
    }

    fn new_phi(&mut self, block: BlockId, variable: VariableId) -> (PhiId, ValueId) {
        let phi_id = PhiId::new(self.phis.len());
        let value = self.new_value(Value::Phi(phi_id));
        let phi = Phi {
            block,
            value,
            variable,
            operands: Vec::new(),
            replaced_by: None,
        };
        self.phis.push(phi);
        self.phi_users.push(Vec::new());
        (phi_id, value)
    }

    /// Resolve is our solution to stale trivial phis after removal. It should be used any time a
    /// value is retrieved since at any point a trivial phi could have been removed.
    fn resolve(&self, mut value_id: ValueId) -> ValueId {
        loop {
            match self.get_value(value_id) {
                Value::Phi(phi_id) => {
                    let phi = self.get_phi(*phi_id);
                    match phi.replaced_by {
                        None => return value_id,
                        Some(v) => {
                            value_id = v;
                        }
                    }
                }
                _ => return value_id,
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{parse_class, Arena, Lexer, Parser};

    fn build<'a>(arena: &'a Arena, body: &str) -> BuildResult<SsaMethod<'a>> {
        let smali = format!(
            ".class public Lcom/example/T;\n.super Ljava/lang/Object;\n\n{}\n",
            body
        );
        let lexer = Lexer::new(smali.as_bytes(), arena);
        let mut parser = Parser::new(lexer);
        let mut class = parse_class(&mut parser).expect("smali failed to parse");
        assert_eq!(class.methods.len(), 1, "expected exactly one method");
        MethodCfg::from_method(class.methods.remove(0)).and_then(SsaMethod::build)
    }

    fn built<'a>(arena: &'a Arena, body: &str) -> SsaMethod<'a> {
        build(arena, body).expect("ssa build failed")
    }

    fn reg(m: &SsaMethod, name: &str) -> VariableId {
        m.variable(Register::parse(name).expect("bad register"))
    }

    /// The literal a collapsed const value carries. Const instructions are folded
    /// into [Value::Const] during construction, so they have no defs to look at.
    fn const_lit<'a>(m: &SsaMethod<'a>, value: ValueId) -> Literal<'a> {
        match m.value(value) {
            Value::Const(id) => *m.constant(*id),
            other => panic!("{value:?} is not a const: {other:?}"),
        }
    }

    fn const_int(m: &SsaMethod, value: ValueId) -> i32 {
        const_lit(m, value)
            .try_into()
            .expect("const is not an integer")
    }

    /// Live phis as (block, variable), sorted, so tests don't depend on the
    /// order phis happen to be created in.
    fn live_phis(m: &SsaMethod) -> Vec<(usize, usize)> {
        let mut out: Vec<(usize, usize)> = m
            .live_phis()
            .map(|(_, phi)| (phi.block.index(), phi.variable.index()))
            .collect();
        out.sort_unstable();
        out
    }

    /// Operands of the phi merging `var` at `block`, as (from block, value).
    fn phi_operands(m: &SsaMethod, block: usize, var: VariableId) -> Vec<(usize, ValueId)> {
        let (_, phi) = m
            .live_phis()
            .find(|(_, phi)| phi.block.index() == block && phi.variable == var)
            .unwrap_or_else(|| panic!("no live phi for {var:?} at B{block}"));
        let mut out: Vec<(usize, ValueId)> = phi
            .operands
            .iter()
            .map(|op| (op.block.index(), op.value))
            .collect();
        out.sort_unstable_by_key(|(block, _)| *block);
        out
    }

    #[test]
    fn straight_line_uses_point_at_the_preceding_def() {
        let arena = Arena::new();
        let m = built(
            &arena,
            r#".method public m()V
    .registers 2
    const/4 v0, 0x1
    move v1, v0
    invoke-static {v1}, Lfoo/Bar;->take(I)V
    return-void
.end method"#,
        );

        assert!(live_phis(&m).is_empty(), "no joins, so no phis");

        // Both the const and the move are collapsed, so v1 is the constant itself
        let taken = m.uses(InstructionId::new(2))[0];
        assert_eq!(const_int(&m, taken), 1);
        assert!(
            m.defs(InstructionId::new(0)).is_empty(),
            "the const has no def of its own"
        );
    }

    #[test]
    fn a_collapsed_move_is_absent_from_uses_and_defs() {
        let arena = Arena::new();
        let m = built(
            &arena,
            r#".method public m()V
    .registers 2
    const/4 v0, 0x1
    move v1, v0
    invoke-static {v1}, Lfoo/Bar;->take(I)V
    move-result v0
    return-void
.end method"#,
        );

        // Anything that indexes defs()[0] blindly would panic instead, which is
        // the whole reason both have to be empty rather than just defs
        for block in m.reverse_post_order() {
            for ins in m.block_instructions(block) {
                let inv = m.instruction(ins);
                if !inv.is_move() && !inv.is_move_result() {
                    continue;
                }
                assert!(
                    m.uses(ins).is_empty() && m.defs(ins).is_empty(),
                    "{} at {ins:?} was only half collapsed: uses={:?} defs={:?}",
                    inv.instruction(),
                    m.uses(ins),
                    m.defs(ins)
                );
            }
        }
    }

    #[test]
    fn an_overlapping_wide_move_reads_before_it_writes() {
        let arena = Arena::new();
        let m = built(
            &arena,
            r#".method public m()V
    .registers 4
    const-wide/16 v0, 0x1
    move-wide v1, v0
    invoke-static {v1, v2}, Lfoo/Bar;->take(J)V
    return-void
.end method"#,
        );

        // const-wide/16 0x1 puts the low word in v0 and the zeroed high word in v1
        let taken = m.uses(InstructionId::new(2));
        assert_eq!(taken.len(), 2);

        // move-wide v1, v0 overlaps on v1: the destination pair is v1,v2 and the
        // source pair is v0,v1, so every read has to happen before any write. A
        // write landing first would carry the low word into both halves.
        assert_eq!(
            const_lit(&m, taken[0]),
            Literal::Int(1),
            "v1 takes the old v0"
        );
        assert_eq!(
            const_lit(&m, taken[1]),
            Literal::Int(0),
            "v2 takes the old v1"
        );
    }

    #[test]
    fn a_wide_const_splits_into_the_words_of_its_pair() {
        let arena = Arena::new();
        let m = built(
            &arena,
            r#".method public m()V
    .registers 3
    const-wide v0, 0x3ff0000000000000L
    invoke-static {v0, v1}, Lfoo/Bar;->take(D)V
    return-void
.end method"#,
        );

        // 1.0 as a double, little endian, so v0 holds the zeroed low word
        let taken = m.uses(InstructionId::new(1));
        assert_eq!(const_lit(&m, taken[0]), Literal::Int(0));
        assert_eq!(const_lit(&m, taken[1]), Literal::Int(0x3ff0_0000));
    }

    #[test]
    fn identical_constants_share_one_value() {
        let arena = Arena::new();
        let m = built(
            &arena,
            r#".method public m(I)I
    .registers 2
    if-lez p1, :cond_0
    const/4 v0, 0x1
    goto :goto_0
    :cond_0
    const/4 v0, 0x1
    :goto_0
    return v0
.end method"#,
        );

        // Both arms assign the same constant, so the join has nothing to merge
        assert!(
            live_phis(&m).is_empty(),
            "the join phi is trivial once both arms share the constant"
        );
        assert_eq!(m.consts.len(), 1, "the literal is stored once");

        let returned = m.uses(InstructionId::new(4))[0];
        assert_eq!(const_lit(&m, returned), Literal::Int(1));
    }

    #[test]
    fn constants_of_different_widths_stay_apart() {
        let arena = Arena::new();
        let m = built(
            &arena,
            r#".method public m()V
    .registers 4
    const/4 v0, 0x1
    const-wide/16 v1, 0x1
    invoke-static {v0}, Lfoo/Bar;->take(I)V
    invoke-static {v1, v2}, Lfoo/Bar;->takeWide(J)V
    return-void
.end method"#,
        );

        // The low word of the wide const is Int(1) as well, so it shares the
        // narrow const's value. The high word is a separate zero.
        let narrow = m.uses(InstructionId::new(2))[0];
        let wide = m.uses(InstructionId::new(3));
        assert_eq!(wide[0], narrow, "the low word is the same value as the 1");
        assert_eq!(const_lit(&m, wide[1]), Literal::Int(0));
        assert_eq!(m.consts.len(), 2, "Int(1) and Int(0)");
    }

    #[test]
    fn parameters_are_defined_on_entry() {
        let arena = Arena::new();
        let m = built(
            &arena,
            r#".method public m(I)I
    .registers 2
    return p1
.end method"#,
        );

        let value = m.uses(InstructionId::new(0))[0];
        let p1 = reg(&m, "p1");
        assert!(
            matches!(m.value(value), Value::Param(var) if *var == p1),
            "p1 should be a Param, got {:?}",
            m.value(value)
        );
    }

    #[test]
    fn if_else_join_gets_one_phi() {
        let arena = Arena::new();
        let m = built(
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

        let v0 = reg(&m, "v0");
        assert_eq!(live_phis(&m), vec![(3, v0.index())]);

        // One operand per branch, each the const folded there
        let operands = phi_operands(&m, 3, v0);
        assert_eq!(operands.len(), 2);
        assert_eq!(operands[0].0, 1);
        assert_eq!(const_int(&m, operands[0].1), 1);
        assert_eq!(operands[1].0, 2);
        assert_eq!(const_int(&m, operands[1].1), 2);

        // The return reads the phi
        let returned = m.uses(InstructionId::new(4))[0];
        assert!(matches!(m.value(returned), Value::Phi(_)));
    }

    #[test]
    fn a_variable_defined_on_only_one_side_still_merges() {
        let arena = Arena::new();
        let m = built(
            &arena,
            r#".method public m(I)I
    .registers 2
    const/4 v0, 0x1
    if-lez p1, :cond_0
    const/4 v0, 0x2
    :cond_0
    return v0
.end method"#,
        );

        let v0 = reg(&m, "v0");
        assert_eq!(live_phis(&m), vec![(2, v0.index())]);

        let operands = phi_operands(&m, 2, v0);
        // Fallthrough carries the first const, the taken branch the second
        assert_eq!(operands[0].0, 0);
        assert_eq!(const_int(&m, operands[0].1), 1);
        assert_eq!(operands[1].0, 1);
        assert_eq!(const_int(&m, operands[1].1), 2);
    }

    #[test]
    fn loop_keeps_only_the_necessary_phis() {
        let arena = Arena::new();
        let m = built(
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

        let v0 = reg(&m, "v0").index();
        let v1 = reg(&m, "v1").index();

        assert_eq!(
            live_phis(&m),
            vec![(1, v0), (1, v1), (7, v0), (11, v0)],
            "sum merges at the header, the three way join and the tail; \
             the counter merges only at the header"
        );

        // The loop invariant array-length and both parameters collapse
        let v2 = reg(&m, "v2");
        let p1 = reg(&m, "p1");
        let p2 = reg(&m, "p2");
        for var in [v2, p1, p2] {
            assert!(
                !m.live_phis().any(|(_, phi)| phi.variable == var),
                "{var:?} should have no surviving phi"
            );
        }

        // v4 is written and never read, so it gets no phi at the B11 join
        let v4 = reg(&m, "v4");
        assert!(!m.live_phis().any(|(_, phi)| phi.variable == v4));

        // The header phi for the counter merges the entry and the latch
        assert_eq!(
            phi_operands(&m, 1, reg(&m, "v1"))
                .iter()
                .map(|(b, _)| *b)
                .collect::<Vec<_>>(),
            vec![0, 7]
        );
    }

    #[test]
    fn removed_phis_are_kept_but_not_live() {
        let arena = Arena::new();
        let m = built(
            &arena,
            r#".method public m(I)I
    .registers 3
    const/4 v0, 0x0
    :goto_0
    if-lez p1, :cond_0
    goto :goto_0
    :cond_0
    return v0
.end method"#,
        );

        // v0 is loop invariant, so its header phi is created and then removed
        assert!(live_phis(&m).is_empty());
        assert!(!m.phis.is_empty(), "the removed phi is still stored");
        assert!(m.phis.iter().all(|phi| !phi.is_live()));

        // Nothing reachable refers to it
        let const_value = m.uses(InstructionId::new(3))[0];
        assert_eq!(const_int(&m, const_value), 0);

        // The removed phi took the const as an operand, but it must not be
        // reported as a consumer of it
        assert!(
            !m.users(const_value)
                .iter()
                .any(|user| matches!(user, ValueUser::Phi(_))),
            "a removed phi is listed as a user: {:?}",
            m.users(const_value)
        );
    }

    /// The loop header phi for `v0` is created, memoized into `current_def`, and
    /// only removed when the latch seals the header. The tail block is filled
    /// after that, so its read finds a `current_def` entry pointing at a phi that
    /// no longer exists.
    #[test]
    fn a_read_through_a_removed_phi_resolves_to_the_real_definition() {
        let arena = Arena::new();
        let m = built(
            &arena,
            r#".method public m(I)I
    .registers 2
    const/4 v0, 0x5
    :goto_0
    if-lez p1, :cond_0
    add-int/2addr p1, v0
    goto :goto_0
    :cond_0
    return v0
.end method"#,
        );

        assert!(
            !m.live_phis().any(|(_, phi)| phi.variable == reg(&m, "v0")),
            "v0 is loop invariant so its phi is trivial"
        );

        let returned = m.uses(InstructionId::new(4))[0];
        assert_eq!(
            const_int(&m, returned),
            5,
            "expected the const, got {:?}",
            m.value(returned)
        );
    }

    #[test]
    fn move_result_takes_the_calls_value() {
        let arena = Arena::new();
        let m = built(
            &arena,
            r#".method public m()V
    .registers 2
    invoke-static {}, Lfoo/Bar;->baz()I
    move-result v0
    invoke-static {v0}, Lfoo/Bar;->take(I)V
    return-void
.end method"#,
        );

        let invoke = InstructionId::new(0);

        // The invoke writes the implicit result register
        let produced = m.defs(invoke)[0];
        assert!(matches!(
            m.value(produced),
            Value::Instruction(id) if *id == invoke
        ));

        // The move-result is collapsed, so v0 *is* the call's value rather than
        // being one hop from it
        assert_eq!(m.uses(InstructionId::new(2)), &[produced]);
    }

    #[test]
    fn users_is_the_reverse_of_uses() {
        let arena = Arena::new();
        let m = built(
            &arena,
            r#".method public m()V
    .registers 2
    const/4 v0, 0x1
    invoke-static {v0}, Lfoo/Bar;->a(I)V
    invoke-static {v0}, Lfoo/Bar;->b(I)V
    invoke-static {}, Lfoo/Bar;->c()I
    return-void
.end method"#,
        );

        // Collapsed moves are not users of anything, so the consumers here are
        // calls rather than the moves this used to be written against
        let const_value = m.uses(InstructionId::new(1))[0];
        assert_eq!(const_int(&m, const_value), 1);
        assert_eq!(
            m.users(const_value),
            &[
                ValueUser::Instruction(InstructionId::new(1)),
                ValueUser::Instruction(InstructionId::new(2)),
            ]
        );

        // Nothing consumes the last call's result
        assert!(m.users(m.defs(InstructionId::new(3))[0]).is_empty());
    }

    #[test]
    fn phis_appear_as_users_of_their_operands() {
        let arena = Arena::new();
        let m = built(
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

        let (phi_id, phi) = m.live_phis().next().expect("expected a phi");
        let branch_value = phi
            .operands
            .iter()
            .find(|op| op.block == BlockId::new(1))
            .expect("no operand from B1")
            .value;
        assert_eq!(const_int(&m, branch_value), 1);
        assert_eq!(m.users(branch_value), &[ValueUser::Phi(phi_id)]);
    }

    #[test]
    fn block_phis_list_only_survivors() {
        let arena = Arena::new();
        let m = built(
            &arena,
            r#".method public m(I)I
    .registers 3
    const/4 v0, 0x0
    if-lez p1, :cond_0
    const/4 v0, 0x1
    :cond_0
    return v0
.end method"#,
        );

        let listed: Vec<PhiId> = m
            .blocks()
            .flat_map(|(_, block)| block.phis.iter().copied())
            .collect();
        let live: Vec<PhiId> = m.live_phis().map(|(id, _)| id).collect();
        assert_eq!(listed, live);

        // and they are listed against the block they actually merge in
        for (id, phi) in m.live_phis() {
            assert!(
                m.block(phi.block).phis.contains(&id),
                "{id:?} merges in B{} but is not listed there",
                phi.block.index()
            );
        }
        assert_eq!(
            m.block(BlockId::new(2)).phis.len(),
            1,
            "the join has the phi"
        );
        assert!(m.block(BlockId::new(0)).phis.is_empty());
        assert!(m.block(BlockId::new(1)).phis.is_empty());
    }

    #[test]
    fn unreachable_blocks_are_not_in_the_order() {
        let arena = Arena::new();
        let m = built(
            &arena,
            r#".method public m()V
    .registers 1
    goto :goto_0
    const/4 v0, 0x0
    :goto_0
    return-void
.end method"#,
        );

        let order: Vec<usize> = m.reverse_post_order().map(|id| id.index()).collect();
        assert_eq!(order, vec![0, 2]);
        assert_eq!(m.blocks.len(), 3);
    }
}
