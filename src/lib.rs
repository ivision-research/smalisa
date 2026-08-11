#[macro_use]
extern crate bitflags;
mod token;
pub use token::Token;

mod lexer;
pub use lexer::{Lex, LexError, LexResult, Lexer};

mod parser;
pub use parser::{parse_class, parse_method, LineParse, ParseError, ParseResult, Parser};

mod line;
pub use line::Line;

mod register;
pub use register::{
    Register, RegisterArray, RegisterCollection, RegisterCollectionIter, RegisterNumber,
    RegisterRange, VarRegister, MAX_FIXED_REGISTERS,
};

mod access;
pub use access::AccessFlag;

mod directive;
pub use directive::Directive;

#[macro_use]
pub(crate) mod utils;

mod arena;
pub use arena::Arena;

mod class;
pub use class::{Class, ClassLineBuilder, PackageClass};

mod method;
pub use method::{
    parse_method_args, parse_method_args_into, Method, MethodHeader, MethodLine, MethodLineBuilder,
    MethodRef,
};

mod field;
pub use field::{Field, FieldRef};

pub mod instructions;

mod primitive;
pub use primitive::Primitive;

mod annotation;
pub use annotation::{Annotation, AnnotationValue, AnnotationVisibility, ParamAnnotations};

mod types;
pub use types::Type;

mod literal;
pub use literal::{Literal, NumericLiteral, RawLiteral};

mod label;
pub use label::{Label, RawLabel};

pub mod extra;

mod enum_type;
pub use enum_type::Enum;

mod catch;
pub use catch::{Catch, CatchAll, NamedCatch, RawCatchAll, RawNamedCatch};

mod switch;
pub use switch::{RawPackedSwitchData, RawSparseSwitchData, RawSwitchPair, SwitchCase, SwitchData};

mod array;
pub use array::{ArrayData, RawArrayData};
