use std::fmt;

use crate::extra::parse_numeric;
use crate::Token;

#[derive(PartialEq, Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum NumericLiteral {
    Float(f32),
    Double(f64),
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
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

#[derive(PartialEq, Debug, Clone, Copy)]
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
        }
    }
}

#[derive(PartialEq, Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "yoke", derive(yoke::Yokeable))]
pub enum RawLiteral<'a> {
    Unset,
    Null,
    Char(&'a str),
    Bool(bool),
    Numeric(&'a str),
    String(&'a str),
}

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
            _ => None,
        }
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
