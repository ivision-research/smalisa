use crate::class::SmaliClassName;
use crate::instructions::Invocation;
use crate::{
    AccessFlag, Annotation, Field, MethodHeader, RawArrayData, RawCatchAll, RawLabel,
    RawNamedCatch, RawPackedSwitchData, RawSparseSwitchData, Register,
};

/// Line represents a logical line of smali. A Line may perhaps span multiple
/// file lines.
#[derive(PartialEq, Debug, Clone)]
pub enum Line<'a> {
    Empty,
    /// The .class directive line
    Class(AccessFlag, &'a SmaliClassName),
    /// The .super directive line
    Super(&'a SmaliClassName),
    /// An .implements directive line
    Interface(&'a SmaliClassName),
    /// Any instruction call
    InstructionInvocation(Invocation<'a>),
    /// A :label line
    LabelDefinition(RawLabel<'a>),
    /// An annotation, either on a class, method, or field. It is up to the
    /// user to know which it is.
    Annotation(Annotation<'a>),
    /// Array data at the end of a method
    ArrayData(RawArrayData<'a>),
    /// Packed switch data at the end of a method
    PackedSwitchData(RawPackedSwitchData<'a>),
    /// Sparse switch data at the end of a method
    SparseSwitchData(RawSparseSwitchData<'a>),
    Field(Field<'a>),
    MethodHeader(MethodHeader<'a>),
    NamedCatch(RawNamedCatch<'a>),
    CatchAll(RawCatchAll<'a>),
    ParamLine(Register, &'a str, Option<Vec<Annotation<'a>>>),
    MethodEnd,
}

impl<'a> Default for Line<'a> {
    fn default() -> Self {
        Self::Empty
    }
}
