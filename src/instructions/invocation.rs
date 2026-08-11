use crate::instructions::Instruction;
use crate::{
    FieldRef, MethodRef, RawLabel, Register, RegisterCollection, Type, VarRegister,
    MAX_FIXED_REGISTERS,
};

#[derive(PartialEq, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Invocation<'a> {
    ins: Instruction,
    #[cfg_attr(feature = "serde", serde(borrow))]
    args: InvArgs<'a>,
}

impl<'a> Default for Invocation<'a> {
    fn default() -> Self {
        Self {
            ins: Default::default(),
            args: InvArgs::Bare,
        }
    }
}

simple_deref!(Invocation<'a>, ins, Instruction, 'a);

impl<'a> Invocation<'a> {
    pub fn new(ins: Instruction, args: InvArgs<'a>) -> Self {
        Self { ins, args }
    }

    #[inline]
    pub fn args(&self) -> &InvArgs<'a> {
        &self.args
    }

    #[inline]
    pub fn instruction(&self) -> Instruction {
        self.ins
    }

    /// Return a [RegisterCollection] containing all registers written by this instruction
    pub fn written_registers(&self) -> RegisterCollection {
        let mut regs = [Register::default(); MAX_FIXED_REGISTERS];
        let mut len = 0;
        if self.ins.sets_register() {
            if let Some(reg) = self.slots().get(0) {
                push(&mut regs, &mut len, reg, self.ins.pair_first());
            }
        }
        RegisterCollection::from_slots(regs, len)
    }

    /// Return a [RegisterCollection] containing all registers read by this instruction
    pub fn read_registers(&self) -> RegisterCollection {
        // All current instructions using a variable list of registers read them all
        if let Some(list) = self.register_list() {
            return RegisterCollection::Var(*list);
        }

        // The first register is purely a destination unless the instruction
        // reads it first
        let write_only_dest = self.ins.sets_register() && !self.ins.is_inout();
        let pairs = [
            self.ins.pair_first(),
            self.ins.pair_second(),
            self.ins.pair_third(),
        ];

        let mut regs = [Register::default(); MAX_FIXED_REGISTERS];
        let mut len = 0;
        for (slot, reg) in self.slots().into_iter().enumerate() {
            if slot == 0 && write_only_dest {
                continue;
            }
            push(&mut regs, &mut len, reg, pairs[slot]);
        }
        RegisterCollection::from_slots(regs, len)
    }

    fn slots(&self) -> InvRegisters {
        InvRegisters::from_args(&self.args)
    }

    fn register_list(&self) -> Option<&VarRegister> {
        match &self.args {
            InvArgs::VarRegMethod(regs, _)
            | InvArgs::VarRegArray(regs, _)
            | InvArgs::Polymorphic(regs, _, _, _) => Some(regs),
            _ => None,
        }
    }
}

#[derive(Clone, Copy)]
struct InvRegisters {
    regs: [Register; 3],
    len: usize,
}

#[derive(Clone, Copy)]
struct InvRegistersIter {
    regs: [Register; 3],
    len: usize,
    at: usize,
}

impl Iterator for InvRegistersIter {
    type Item = Register;

    fn next(&mut self) -> Option<Self::Item> {
        if self.at >= self.len {
            return None;
        }
        let reg = self.regs[self.at];
        self.at += 1;
        Some(reg)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let rem = self.len.saturating_sub(self.at);
        (rem, Some(rem))
    }
}

impl ExactSizeIterator for InvRegistersIter {}

impl IntoIterator for InvRegisters {
    type Item = Register;
    type IntoIter = InvRegistersIter;
    fn into_iter(self) -> Self::IntoIter {
        InvRegistersIter {
            regs: self.regs,
            len: self.len,
            at: 0,
        }
    }
}

impl InvRegisters {
    fn get(&self, idx: usize) -> Option<Register> {
        if idx >= self.len {
            None
        } else {
            Some(self.regs[idx])
        }
    }

    fn from_args(args: &InvArgs<'_>) -> Self {
        let (regs, len) = match args {
            InvArgs::OneReg(a)
            | InvArgs::RegStr(a, _)
            | InvArgs::OneRegLabel(a, _)
            | InvArgs::OneRegNum(a, _)
            | InvArgs::OneRegField(a, _)
            | InvArgs::OneRegClass(a, _) => ([*a, Default::default(), Default::default()], 1),

            InvArgs::TwoReg(a, b)
            | InvArgs::TwoRegLabel(a, b, _)
            | InvArgs::TwoRegNum(a, b, _)
            | InvArgs::TwoRegField(a, b, _)
            | InvArgs::TwoRegClass(a, b, _)
            | InvArgs::TwoRegArray(a, b, _) => ([*a, *b, Default::default()], 2),

            InvArgs::ThreeReg(a, b, c) => ([*a, *b, *c], 3),
            InvArgs::Bare
            | InvArgs::Label(_)
            | InvArgs::VarRegMethod(_, _)
            | InvArgs::VarRegArray(_, _)
            | InvArgs::Polymorphic(_, _, _, _) => ([Default::default(); 3], 0),
        };

        Self { regs, len }
    }
}

fn push(regs: &mut [Register; MAX_FIXED_REGISTERS], len: &mut usize, reg: Register, pair: bool) {
    debug_assert!(*len < MAX_FIXED_REGISTERS);
    regs[*len] = reg;
    *len += 1;
    if pair {
        regs[*len] = Register::from_raw(reg.is_param(), reg.num() + 1);
        *len += 1;
    }
}

#[derive(PartialEq, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum InvArgs<'a> {
    Bare,

    OneReg(Register),
    TwoReg(Register, Register),
    ThreeReg(Register, Register, Register),

    RegStr(Register, &'a str),

    VarRegMethod(VarRegister, MethodRef<'a>),

    Label(RawLabel<'a>),
    OneRegLabel(Register, RawLabel<'a>),
    TwoRegLabel(Register, Register, RawLabel<'a>),

    // RegNum uses the string representation of the literal so users only parse
    // numerics that they care about.
    OneRegNum(Register, &'a str),
    TwoRegNum(Register, Register, &'a str),

    OneRegField(Register, FieldRef<'a>),
    TwoRegField(Register, Register, FieldRef<'a>),

    OneRegClass(Register, Type<'a>),
    TwoRegClass(Register, Register, Type<'a>),

    VarRegArray(VarRegister, Type<'a>),
    TwoRegArray(Register, Register, Type<'a>),

    /// Used only for invoke-polymorphic and invoke-polymorphic/range
    Polymorphic(VarRegister, MethodRef<'a>, &'a str, Type<'a>),
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::instructions::*;
    use crate::{Primitive, RegisterArray, RegisterRange};

    macro_rules! reg {
        (p $lit:literal) => {
            crate::Register::new(true, u16::try_from($lit).unwrap()).unwrap()
        };
        (v $lit:literal) => {
            crate::Register::new(false, u16::try_from($lit).unwrap()).unwrap()
        };
    }

    fn check(ins: Instruction, args: InvArgs<'static>, defs: &[Register], uses: &[Register]) {
        let inv = Invocation::new(ins, args);
        let got_defs: Vec<Register> = inv.written_registers().into_iter().collect();
        let got_uses: Vec<Register> = inv.read_registers().into_iter().collect();
        assert_eq!(got_defs, defs, "{} defs", ins);
        assert_eq!(got_uses, uses, "{} uses", ins);
        assert_eq!(
            inv.written_registers().len(),
            defs.len(),
            "{} defs len",
            ins
        );
        assert_eq!(inv.read_registers().len(), uses.len(), "{} uses len", ins);
    }

    #[test]
    fn pairs_expand_on_the_tagged_slot() {
        check(
            INS_ADD_LONG,
            InvArgs::ThreeReg(reg!(v 0), reg!(v 2), reg!(v 4)),
            &[reg!(v 0), reg!(v 1)],
            &[reg!(v 2), reg!(v 3), reg!(v 4), reg!(v 5)],
        );
        // The shift amount is an int, so slot C is not a pair.
        check(
            INS_SHL_LONG,
            InvArgs::ThreeReg(reg!(v 0), reg!(v 2), reg!(v 4)),
            &[reg!(v 0), reg!(v 1)],
            &[reg!(v 2), reg!(v 3), reg!(v 4)],
        );
        // Source is the pair, destination is a plain int.
        check(
            INS_LONG_TO_INT,
            InvArgs::TwoReg(reg!(v 0), reg!(v 1)),
            &[reg!(v 0)],
            &[reg!(v 1), reg!(v 2)],
        );
        // Destination is the pair, source is a plain int.
        check(
            INS_INT_TO_LONG,
            InvArgs::TwoReg(reg!(v 0), reg!(v 2)),
            &[reg!(v 0), reg!(v 1)],
            &[reg!(v 2)],
        );
        // Operands are pairs, result is an int.
        check(
            INS_CMP_LONG,
            InvArgs::ThreeReg(reg!(v 0), reg!(v 2), reg!(v 4)),
            &[reg!(v 0)],
            &[reg!(v 2), reg!(v 3), reg!(v 4), reg!(v 5)],
        );
    }

    #[test]
    fn inout_reads_its_destination() {
        check(
            INS_ADD_LONG_2ADDR,
            InvArgs::TwoReg(reg!(v 0), reg!(v 2)),
            &[reg!(v 0), reg!(v 1)],
            &[reg!(v 0), reg!(v 1), reg!(v 2), reg!(v 3)],
        );
        // Not an inout, so slot A is write only.
        check(
            INS_NEG_LONG,
            InvArgs::TwoReg(reg!(v 0), reg!(v 2)),
            &[reg!(v 0), reg!(v 1)],
            &[reg!(v 2), reg!(v 3)],
        );
    }

    #[test]
    fn arrays_and_fields() {
        check(
            INS_AGET_WIDE,
            InvArgs::ThreeReg(reg!(v 0), reg!(v 2), reg!(v 3)),
            &[reg!(v 0), reg!(v 1)],
            &[reg!(v 2), reg!(v 3)],
        );
        check(
            INS_APUT_WIDE,
            InvArgs::ThreeReg(reg!(v 0), reg!(v 2), reg!(v 3)),
            &[],
            &[reg!(v 0), reg!(v 1), reg!(v 2), reg!(v 3)],
        );
        let field = FieldRef {
            class: "LFoo;",
            name: "f",
            ty: Type::new_prim(Primitive::Long),
        };
        check(
            INS_IPUT_WIDE,
            InvArgs::TwoRegField(reg!(v 0), reg!(p 0), field.clone()),
            &[],
            &[reg!(v 0), reg!(v 1), reg!(p 0)],
        );
        check(
            INS_IGET_WIDE,
            InvArgs::TwoRegField(reg!(v 0), reg!(p 0), field.clone()),
            &[reg!(v 0), reg!(v 1)],
            &[reg!(p 0)],
        );
        check(
            INS_SGET_WIDE,
            InvArgs::OneRegField(reg!(v 0), field),
            &[reg!(v 0), reg!(v 1)],
            &[],
        );
    }

    #[test]
    fn results_and_returns() {
        check(
            INS_MOVE_RESULT_WIDE,
            InvArgs::OneReg(reg!(v 0)),
            &[reg!(v 0), reg!(v 1)],
            &[],
        );
        check(
            INS_RETURN_WIDE,
            InvArgs::OneReg(reg!(v 0)),
            &[],
            &[reg!(v 0), reg!(v 1)],
        );
        check(INS_RETURN_VOID, InvArgs::Bare, &[], &[]);
    }

    #[test]
    fn register_lists_are_not_expanded() {
        // Both halves of the long are already named.
        let mut array = RegisterArray::new_empty();
        array.push(reg!(v 0));
        array.push(reg!(v 1));
        let mref = MethodRef::new("LFoo;", "bar", "J", Type::new_prim(Primitive::Void));
        check(
            INS_INVOKE_STATIC,
            InvArgs::VarRegMethod(VarRegister::Array(array), mref),
            &[],
            &[reg!(v 0), reg!(v 1)],
        );

        let mref = MethodRef::new("LFoo;", "baz", "III", Type::new_prim(Primitive::Void));
        let range = RegisterRange::new(reg!(v 3), reg!(v 5));
        check(
            INS_INVOKE_STATIC_RANGE,
            InvArgs::VarRegMethod(VarRegister::Range(range), mref),
            &[],
            &[reg!(v 3), reg!(v 4), reg!(v 5)],
        );
    }

    #[test]
    fn single_width_instructions_are_unaffected() {
        check(
            INS_ADD_INT,
            InvArgs::ThreeReg(reg!(v 0), reg!(v 1), reg!(v 2)),
            &[reg!(v 0)],
            &[reg!(v 1), reg!(v 2)],
        );
        check(
            INS_CHECK_CAST,
            InvArgs::OneRegClass(reg!(v 0), Type::new_class("LFoo;")),
            &[],
            &[reg!(v 0)],
        );
        // The array reference is read, nothing is written.
        check(
            INS_FILL_ARRAY_DATA,
            InvArgs::OneRegLabel(reg!(v 0), RawLabel::new("array_0")),
            &[],
            &[reg!(v 0)],
        );
    }
}
