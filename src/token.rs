use crate::instructions::Instruction;
use crate::{AccessFlag, AnnotationVisibility, Directive, Primitive, Register};

/// Token is a single smali token.
#[derive(PartialEq, Debug, Clone, Copy)]
pub enum Token<'a> {
    Unknown,
    AccessSpec(AccessFlag),
    Directive(Directive),
    Instruction(Instruction),
    ArrayTypePrefix,
    Arrow,
    NullLiteral,
    BoolLiteral(bool),
    /// Contains the unparsed char with single quotes removed
    CharLiteral(&'a str),
    // Contains the unparsed numeric value
    NumericLiteral(&'a str),
    StringLiteral(&'a str),
    ClassDescriptor(&'a str),
    MethodArgs(&'a str),
    CloseBrace,
    CloseParen,
    Colon,
    Comma,
    DotDot,
    Equal,
    OpenBrace,
    OpenParen,
    PrimitiveType(Primitive),
    Register(Register),
    SimpleName(&'a str),
    AnnotationVisibility(AnnotationVisibility),
    // These are used by odex and the like and we don't handle them currently.

    /*
    MethodHandleTypeField,
    MethodHandleTypeMethod,
    At,
    InlineIndex,
    FieldOffset,
    VtableIndex,
    VerificationErrorType,
    */
}

impl<'a> Default for Token<'a> {
    #[inline(always)]
    fn default() -> Token<'a> {
        Token::Unknown
    }
}
