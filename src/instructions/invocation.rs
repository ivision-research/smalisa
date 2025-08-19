use crate::instructions::Instruction;
use crate::{simple_deref, FieldRef, MethodRef, RawLabel, Register, Type, VarRegister};

#[derive(PartialEq, Debug, Clone)]
pub struct Invocation<'a> {
    ins: Instruction,
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
}

#[derive(PartialEq, Debug, Clone)]
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
mod test {}
