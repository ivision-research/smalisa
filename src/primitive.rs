use std::{fmt, hash::Hash};

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum Primitive {
    Int,
    Long,
    Short,
    Byte,
    Char,
    Float,
    Double,
    Bool,
    Void,
}

impl Hash for Primitive {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_smali_str().hash(state);
    }
}

impl Primitive {
    pub fn as_smali_str(&self) -> &'static str {
        match *self {
            Self::Int => "I",
            Self::Long => "J",
            Self::Short => "S",
            Self::Byte => "B",
            Self::Char => "C",
            Self::Float => "F",
            Self::Double => "D",
            Self::Bool => "Z",
            Self::Void => "V",
        }
    }

    #[deprecated(since = "0.2.0", note = "use `as_smali_str` instead")]
    #[inline(always)]
    pub fn as_str(&self) -> &'static str {
        self.as_smali_str()
    }

    pub fn as_java_str(&self) -> &'static str {
        match *self {
            Self::Int => "int",
            Self::Long => "long",
            Self::Byte => "byte",
            Self::Char => "char",
            Self::Bool => "boolean",
            Self::Void => "void",
            Self::Short => "short",
            Self::Float => "float",
            Self::Double => "double",
        }
    }
}

impl fmt::Display for Primitive {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                Self::Int => "I",
                Self::Long => "J",
                Self::Short => "S",
                Self::Byte => "B",
                Self::Char => "C",
                Self::Float => "F",
                Self::Double => "D",
                Self::Bool => "Z",
                Self::Void => "V",
            }
        )
    }
}
