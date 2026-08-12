use std::fmt;
use std::{borrow::Cow, hash::Hash};

use crate::{PackageClass, Primitive};

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "yoke", derive(yoke::Yokeable))]
pub enum Type<'a> {
    Unknown,
    Class(&'a str, u8),
    Primitive(Primitive, u8),
}

impl<'a> Hash for Type<'a> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Type::Class(cd, size) => {
                cd.hash(state);
                size.hash(state);
            }
            Type::Primitive(prim, size) => {
                prim.hash(state);
                size.hash(state);
            }
            Type::Unknown => {
                state.write(&[]);
            }
        }
    }
}

impl<'a> fmt::Display for Type<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Type::Class(cd, size) => {
                write!(f, "{}{}", "[".repeat(size as usize), cd)
            }
            Type::Primitive(prim, size) => {
                write!(f, "{}{}", "[".repeat(size as usize), prim)
            }
            Type::Unknown => {
                write!(f, "?")
            }
        }
    }
}

impl<'a> Type<'a> {
    /// Returns the name of the type as it would appear in smali.
    pub fn as_smali_str(&self) -> Option<Cow<'a, str>> {
        match *self {
            Type::Primitive(prim, size) => {
                if size == 0 {
                    Some(Cow::Borrowed(prim.as_smali_str()))
                } else {
                    Some(Cow::Owned(format!(
                        "{}{}",
                        "[".repeat(size as usize),
                        prim.as_smali_str()
                    )))
                }
            }
            Type::Class(cd, size) => {
                if size == 0 {
                    Some(Cow::Borrowed(cd))
                } else {
                    Some(Cow::Owned(format!("{}{}", "[".repeat(size as usize), cd)))
                }
            }
            Type::Unknown => None,
        }
    }

    /// Returns the name of the type as it would appear in Java source code.
    pub fn as_java_str(&self, fully_qualified: bool) -> Option<Cow<'a, str>> {
        match *self {
            Type::Primitive(prim, size) => {
                if size == 0 {
                    Some(Cow::Borrowed(prim.as_java_str()))
                } else {
                    Some(Cow::Owned(format!(
                        "{}{}",
                        prim.as_java_str(),
                        "[]".repeat(size as usize)
                    )))
                }
            }
            Type::Class(cd, size) => {
                let pclass = PackageClass::from_lclass(cd)?;
                if size == 0 {
                    if fully_qualified {
                        Some(Cow::Owned(format!(
                            "{}.{}",
                            pclass.dot_package(),
                            pclass.name
                        )))
                    } else {
                        Some(Cow::Borrowed(pclass.name))
                    }
                } else {
                    if fully_qualified {
                        Some(Cow::Owned(format!(
                            "{}.{}{}",
                            pclass.dot_package(),
                            pclass.name,
                            "[]".repeat(size as usize)
                        )))
                    } else {
                        Some(Cow::Owned(format!(
                            "{}{}",
                            pclass.name,
                            "[]".repeat(size as usize)
                        )))
                    }
                }
            }
            Type::Unknown => None,
        }
    }
}

impl<'a> Type<'a> {
    #[inline]
    pub fn new_prim(p: Primitive) -> Self {
        Self::Primitive(p, 0)
    }

    #[inline]
    pub fn new_prim_array(p: Primitive, dim: u8) -> Self {
        Self::Primitive(p, dim)
    }

    #[inline]
    pub fn new_class(clazz: &'a str) -> Self {
        Self::Class(clazz, 0)
    }

    #[inline]
    pub fn new_class_array(clazz: &'a str, dim: u8) -> Self {
        Self::Class(clazz, dim)
    }
}

impl<'a> Default for Type<'a> {
    fn default() -> Self {
        Type::Unknown
    }
}

impl<'a> From<Primitive> for Type<'a> {
    #[inline(always)]
    fn from(prim: Primitive) -> Self {
        Self::Primitive(prim, 0)
    }
}

#[cfg(test)]
mod test {

    use super::*;

    macro_rules! test_java_str {
        ($ty:expr, $qual:literal) => {
            let as_str = $ty.as_java_str($qual);
            assert!(as_str.is_none());
        };

        (owned $ty:expr, $expected:literal, $qual:literal) => {{
            let as_str = $ty.as_java_str($qual);
            assert_eq!(as_str, Some(Cow::Owned($expected.to_string())));
        }};

        (borrowed $ty:expr, $expected:literal, $qual:literal) => {{
            let as_str = $ty.as_java_str($qual);
            assert_eq!(as_str, Some(Cow::Borrowed($expected)));
        }};
    }

    macro_rules! test_smali_str {
        ($ty:expr) => {
            let as_str = $ty.as_smali_str();
            assert!(as_str.is_none());
        };

        (owned $ty:expr, $expected:literal) => {{
            let as_str = $ty.as_smali_str();
            assert_eq!(as_str, Some(Cow::Owned($expected.to_string())));
        }};

        (borrowed $ty:expr, $expected:literal) => {{
            let as_str = $ty.as_smali_str();
            assert_eq!(as_str, Some(Cow::Borrowed($expected)));
        }};
    }

    #[test]
    fn as_java_str() {
        test_java_str!(Type::Unknown, false);
        test_java_str!(Type::Unknown, true);
        test_java_str!(borrowed Type::Primitive(Primitive::Int, 0), "int", false);
        test_java_str!(borrowed Type::Primitive(Primitive::Long, 0), "long", false);
        test_java_str!(borrowed Type::Primitive(Primitive::Double, 0), "double", false);
        test_java_str!(borrowed Type::Primitive(Primitive::Float, 0), "float", false);
        test_java_str!(borrowed Type::Primitive(Primitive::Char, 0), "char", false);
        test_java_str!(borrowed Type::Primitive(Primitive::Void, 0), "void", false);
        test_java_str!(borrowed Type::Primitive(Primitive::Bool, 0), "boolean", false);
        test_java_str!(borrowed Type::Primitive(Primitive::Byte, 0), "byte", false);
        test_java_str!(borrowed Type::Primitive(Primitive::Short, 0), "short", false);

        test_java_str!(owned Type::Primitive(Primitive::Int, 1), "int[]", false);
        test_java_str!(owned Type::Primitive(Primitive::Long, 2), "long[][]", false);
        test_java_str!(owned Type::Primitive(Primitive::Double, 1), "double[]", false);
        test_java_str!(owned Type::Primitive(Primitive::Float, 2), "float[][]", false);
        test_java_str!(owned Type::Primitive(Primitive::Char, 1), "char[]", false);
        test_java_str!(owned Type::Primitive(Primitive::Bool, 1), "boolean[]", false);
        test_java_str!(owned Type::Primitive(Primitive::Byte, 3), "byte[][][]", false);
        test_java_str!(owned Type::Primitive(Primitive::Short, 1), "short[]", false);

        test_java_str!(borrowed Type::Class("Lfoo/bar/Baz;", 0), "Baz", false);
        test_java_str!(owned Type::Class("Lfoo/bar/Baz;", 1), "Baz[]", false);
        test_java_str!(owned Type::Class("Lfoo/bar/Baz;", 2), "Baz[][]", false);
        test_java_str!(owned Type::Class("Lfoo/bar/Baz;", 0), "foo.bar.Baz", true);
        test_java_str!(owned Type::Class("Lfoo/bar/Baz;", 1), "foo.bar.Baz[]", true);
        test_java_str!(owned Type::Class("Lfoo/bar/Baz;", 2), "foo.bar.Baz[][]", true);
    }
    #[test]
    fn as_smali_str() {
        test_smali_str!(Type::Unknown);
        test_smali_str!(Type::Unknown);
        test_smali_str!(borrowed Type::Primitive(Primitive::Int, 0), "I");
        test_smali_str!(borrowed Type::Primitive(Primitive::Long, 0), "J");
        test_smali_str!(borrowed Type::Primitive(Primitive::Double, 0), "D");
        test_smali_str!(borrowed Type::Primitive(Primitive::Float, 0), "F");
        test_smali_str!(borrowed Type::Primitive(Primitive::Char, 0), "C");
        test_smali_str!(borrowed Type::Primitive(Primitive::Void, 0), "V");
        test_smali_str!(borrowed Type::Primitive(Primitive::Bool, 0), "Z");
        test_smali_str!(borrowed Type::Primitive(Primitive::Byte, 0), "B");
        test_smali_str!(borrowed Type::Primitive(Primitive::Short, 0), "S");

        test_smali_str!(owned Type::Primitive(Primitive::Int, 1), "[I");
        test_smali_str!(owned Type::Primitive(Primitive::Long, 2), "[[J");
        test_smali_str!(owned Type::Primitive(Primitive::Double, 1), "[D");
        test_smali_str!(owned Type::Primitive(Primitive::Float, 2), "[[F");
        test_smali_str!(owned Type::Primitive(Primitive::Char, 1), "[C");
        test_smali_str!(owned Type::Primitive(Primitive::Bool, 1), "[Z");
        test_smali_str!(owned Type::Primitive(Primitive::Byte, 3), "[[[B");
        test_smali_str!(owned Type::Primitive(Primitive::Short, 1), "[S");

        test_smali_str!(borrowed Type::Class("Lfoo/bar/Baz;", 0), "Lfoo/bar/Baz;");
        test_smali_str!(owned Type::Class("Lfoo/bar/Baz;", 1), "[Lfoo/bar/Baz;");
        test_smali_str!(owned Type::Class("Lfoo/bar/Baz;", 2), "[[Lfoo/bar/Baz;");
    }
}
