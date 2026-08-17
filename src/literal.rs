use std::error::Error;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::mem::discriminant;

use crate::extra::parse_numeric;
use crate::{SmaliClassName, Token, Type};

/// Wrapper for numeric literal values
///
/// Note that this type doesn't have a standard [PartialEq]/[Eq] implementation: floats compare via
/// `to_bits` calls. This is _not_ how equality on floats normally works!
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum NumericLiteral {
    Float(f32),
    Double(f64),
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
}

impl PartialEq for NumericLiteral {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Float(lhs), Self::Float(rhs)) => lhs.to_bits() == rhs.to_bits(),
            (Self::Double(lhs), Self::Double(rhs)) => lhs.to_bits() == rhs.to_bits(),
            (Self::Byte(lhs), Self::Byte(rhs)) => lhs == rhs,
            (Self::Short(lhs), Self::Short(rhs)) => lhs == rhs,
            (Self::Int(lhs), Self::Int(rhs)) => lhs == rhs,
            (Self::Long(lhs), Self::Long(rhs)) => lhs == rhs,
            _ => false,
        }
    }
}

impl Eq for NumericLiteral {}

impl Hash for NumericLiteral {
    fn hash<H: Hasher>(&self, state: &mut H) {
        discriminant(self).hash(state);
        match self {
            Self::Float(num) => num.to_bits().hash(state),
            Self::Double(num) => num.to_bits().hash(state),
            Self::Byte(num) => num.hash(state),
            Self::Short(num) => num.hash(state),
            Self::Int(num) => num.hash(state),
            Self::Long(num) => num.hash(state),
        }
    }
}

impl<'a> From<NumericLiteral> for Literal<'a> {
    fn from(nl: NumericLiteral) -> Self {
        match nl {
            NumericLiteral::Float(n) => Self::Float(n),
            NumericLiteral::Double(n) => Self::Double(n),
            NumericLiteral::Byte(n) => Self::Byte(n),
            NumericLiteral::Short(n) => Self::Short(n),
            NumericLiteral::Int(n) => Self::Int(n),
            NumericLiteral::Long(n) => Self::Long(n),
        }
    }
}

/// Wrapper for literal values
///
/// Note that this type doesn't have a standard [PartialEq]/[Eq] implementation: floats compare via
/// `to_bits` calls. This is _not_ how equality on floats normally works!
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "yoke", derive(yoke::Yokeable))]
pub enum Literal<'a> {
    Null,
    Char(char),
    Bool(bool),
    Float(f32),
    Double(f64),
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    String(&'a str),
    Type(#[cfg_attr(feature = "serde", serde(borrow))] Type<'a>),
}

impl<'a> PartialEq for Literal<'a> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Null, Self::Null) => true,
            (Self::Char(lhs), Self::Char(rhs)) => lhs == rhs,
            (Self::Bool(lhs), Self::Bool(rhs)) => lhs == rhs,
            (Self::Float(lhs), Self::Float(rhs)) => lhs.to_bits() == rhs.to_bits(),
            (Self::Double(lhs), Self::Double(rhs)) => lhs.to_bits() == rhs.to_bits(),
            (Self::Byte(lhs), Self::Byte(rhs)) => lhs == rhs,
            (Self::Short(lhs), Self::Short(rhs)) => lhs == rhs,
            (Self::Int(lhs), Self::Int(rhs)) => lhs == rhs,
            (Self::Long(lhs), Self::Long(rhs)) => lhs == rhs,
            (Self::String(lhs), Self::String(rhs)) => lhs == rhs,
            (Self::Type(lhs), Self::Type(rhs)) => lhs == rhs,
            _ => false,
        }
    }
}

impl<'a> Eq for Literal<'a> {}

impl<'a> Hash for Literal<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        discriminant(self).hash(state);
        match self {
            Self::Null => {}
            Self::Char(chr) => chr.hash(state),
            Self::Bool(val) => val.hash(state),
            Self::Float(num) => num.to_bits().hash(state),
            Self::Double(num) => num.to_bits().hash(state),
            Self::Byte(num) => num.hash(state),
            Self::Short(num) => num.hash(state),
            Self::Int(num) => num.hash(state),
            Self::Long(num) => num.hash(state),
            Self::String(string) => string.hash(state),
            Self::Type(ty) => ty.hash(state),
        }
    }
}

#[derive(Debug)]
pub struct LiteralIntoError<'a> {
    expected: &'static str,
    got: Literal<'a>,
}

impl<'a> fmt::Display for LiteralIntoError<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "expected Literal::{} but had Literal::{}",
            self.expected, self.got
        )
    }
}

impl<'a> Error for LiteralIntoError<'a> {}

macro_rules! lit_try_into {
    ($target:ty, $name:ident) => {
        impl<'a> TryInto<$target> for Literal<'a> {
            type Error = LiteralIntoError<'a>;
            fn try_into(self) -> Result<$target, Self::Error> {
                if let Self::$name(inner) = self {
                    Ok(inner)
                } else {
                    Err(Self::Error {
                        expected: stringify!(name),
                        got: self,
                    })
                }
            }
        }
    };
}

lit_try_into!(i32, Int);
lit_try_into!(char, Char);
lit_try_into!(bool, Bool);
lit_try_into!(f32, Float);
lit_try_into!(f64, Double);
lit_try_into!(i8, Byte);
lit_try_into!(i16, Short);
lit_try_into!(&'a str, String);

impl<'a> TryInto<String> for Literal<'a> {
    type Error = LiteralIntoError<'a>;
    fn try_into(self) -> Result<String, Self::Error> {
        if let Self::String(inner) = self {
            Ok(inner.into())
        } else {
            Err(Self::Error {
                expected: "String",
                got: self,
            })
        }
    }
}

impl<'a> Literal<'a> {
    pub fn is_str(&self) -> bool {
        matches!(self, Self::String(_))
    }

    pub fn is_numeric(&self) -> bool {
        matches!(
            self,
            Self::Float(_)
                | Self::Double(_)
                | Self::Byte(_)
                | Self::Short(_)
                | Self::Int(_)
                | Self::Long(_)
        )
    }

    pub fn is_bool(&self) -> bool {
        matches!(self, Self::Bool(_))
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    pub fn is_char(&self) -> bool {
        matches!(self, Self::Char(_))
    }

    pub fn is_type(&self) -> bool {
        matches!(self, Self::Type(_))
    }

    pub fn is_class(&self) -> bool {
        matches!(self, Self::Type(Type::Class(_, _)))
    }
}

impl<'a> fmt::Display for Literal<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Literal::Null => write!(f, "null"),
            Literal::Char(c) => write!(f, "{}", c),
            Literal::Bool(b) => write!(f, "{}", b),
            Literal::Float(n) => write!(f, "{}", n),
            Literal::Double(d) => write!(f, "{}", d),
            Literal::Byte(b) => write!(f, "{}", b),
            Literal::Short(s) => write!(f, "{}", s),
            Literal::Int(i) => write!(f, "{}", i),
            Literal::Long(l) => write!(f, "{}", l),
            Literal::String(s) => write!(f, "{}", s),
            Literal::Type(ty) => write!(f, "{}", ty),
        }
    }
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "yoke", derive(yoke::Yokeable))]
pub enum RawLiteral<'a> {
    Unset,
    Null,
    Char(&'a str),
    Bool(bool),
    Numeric(&'a str),
    String(&'a str),
    Type(#[cfg_attr(feature = "serde", serde(borrow))] Type<'a>),
}

macro_rules! raw_lit_try_into {
    ($target:ty) => {
        impl<'a> TryInto<$target> for RawLiteral<'a> {
            type Error = RawLiteralTryIntoError<'a>;
            fn try_into(self) -> Result<$target, Self::Error> {
                match self.to_literal() {
                    Some(v) => v
                        .try_into()
                        .map_err(|e| RawLiteralTryIntoError::LiteralError(self, e)),
                    None => Err(RawLiteralTryIntoError::ParseErrror(self)),
                }
            }
        }
    };
}

raw_lit_try_into!(i32);
raw_lit_try_into!(char);
raw_lit_try_into!(bool);
raw_lit_try_into!(f32);
raw_lit_try_into!(f64);
raw_lit_try_into!(i8);
raw_lit_try_into!(i16);
raw_lit_try_into!(&'a str);
raw_lit_try_into!(String);

#[derive(Debug)]
pub enum RawLiteralTryIntoError<'a> {
    ParseErrror(RawLiteral<'a>),
    LiteralError(RawLiteral<'a>, LiteralIntoError<'a>),
}

impl<'a> fmt::Display for RawLiteralTryIntoError<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ParseErrror(raw) => write!(f, "failed to parse raw literal: {:?}", raw),
            Self::LiteralError(raw, err) => write!(f, "failed to convert {:?}: {}", raw, err),
        }
    }
}

impl<'a> Error for RawLiteralTryIntoError<'a> {}

impl<'a> Default for RawLiteral<'a> {
    #[inline(always)]
    fn default() -> Self {
        Self::Unset
    }
}

impl<'a> RawLiteral<'a> {
    /// Reads a RawLiteral out of a Token. Returns None if the Token wasn't a
    /// known literal type.
    pub fn from_token(tok: &Token<'a>) -> Option<Self> {
        match tok {
            Token::NumericLiteral(s) => Some(RawLiteral::Numeric(s)),
            Token::StringLiteral(s) => Some(RawLiteral::String(s)),
            Token::BoolLiteral(b) => Some(RawLiteral::Bool(*b)),
            Token::NullLiteral => Some(RawLiteral::Null),
            Token::CharLiteral(s) => Some(RawLiteral::Char(s)),
            Token::ClassDescriptor(c) => {
                Some(RawLiteral::Type(Type::new_class(SmaliClassName::new(c))))
            }
            _ => None,
        }
    }

    /// Converts a raw literal to a literal. This will perform any necessary
    /// parsing operations. Instead of returning a parsing error, this function
    /// just returns None.
    pub fn to_literal(&self) -> Option<Literal<'a>> {
        match self {
            RawLiteral::Null => Some(Literal::Null),
            RawLiteral::Numeric(num) => {
                if let Some(v) = parse_numeric(num) {
                    Some(v.into())
                } else {
                    None
                }
            }
            RawLiteral::Bool(b) => Some(Literal::Bool(*b)),
            RawLiteral::Char(s) => {
                let b = s.as_bytes();
                if s.len() == 1 {
                    Some(Literal::Char(b[0] as char))
                } else if s.is_empty() {
                    None
                } else {
                    if s.len() != 6 {
                        None
                    } else {
                        let (_, hex) = b.split_at(2);
                        let as_str = if let Ok(s) = std::str::from_utf8(&hex) {
                            s
                        } else {
                            return None;
                        };
                        let decoded = if let Ok(v) = u32::from_str_radix(as_str, 16) {
                            v
                        } else {
                            return None;
                        };
                        let c = if let Some(v) = std::char::from_u32(decoded) {
                            v
                        } else {
                            return None;
                        };
                        Some(Literal::Char(c))
                    }
                }
            }
            RawLiteral::String(s) => Some(Literal::String(s)),
            RawLiteral::Type(ty) => Some(Literal::Type(*ty)),
            _ => None,
        }
    }

    pub fn is_str(&self) -> bool {
        matches!(self, Self::String(_))
    }

    pub fn is_numeric(&self) -> bool {
        matches!(self, Self::Numeric(_))
    }

    pub fn is_bool(&self) -> bool {
        matches!(self, Self::Bool(_))
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    pub fn is_char(&self) -> bool {
        matches!(self, Self::Char(_))
    }

    pub fn is_type(&self) -> bool {
        matches!(self, Self::Type(_))
    }

    pub fn is_class(&self) -> bool {
        matches!(self, Self::Type(Type::Class(_, _)))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn convert_char() {
        let raw = RawLiteral::Char("b");
        assert_eq!(raw.to_literal(), Some(Literal::Char('b')));
        let raw = RawLiteral::Char("\\u2764");
        assert_eq!(raw.to_literal(), Some(Literal::Char('❤')));

        let raw = RawLiteral::Char("\\u276");
        assert!(raw.to_literal().is_none());
    }
}
