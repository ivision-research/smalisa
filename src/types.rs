use std::fmt;
use std::{borrow::Cow, hash::Hash};

use crate::class::{ClassName, SmaliClassName};
use crate::Primitive;

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "yoke", derive(yoke::Yokeable))]
pub enum Type<'a> {
    Class(
        #[cfg_attr(feature = "serde", serde(borrow))] &'a SmaliClassName,
        u8,
    ),
    Primitive(Primitive, u8),
}

impl<'a> Default for Type<'a> {
    fn default() -> Self {
        Self::Primitive(Primitive::Void, 0)
    }
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
        }
    }
}

impl<'a> Type<'a> {
    /// Attempt to parse a single smali type, returning None on failure
    ///
    /// This will parse both classes (La/b/C;) and primitives (I, J, Z, etc) as well as arrays of
    /// either of those
    pub fn parse_smali(smali: &'a str) -> Option<Self> {
        let trimmed = smali.trim_start_matches('[');
        let dim = (smali.len() - trimmed.len()) as u8;

        Some(if trimmed.starts_with('L') {
            Self::Class(SmaliClassName::new(trimmed), dim)
        } else {
            Self::Primitive(Primitive::from_smali_str(trimmed)?, dim)
        })
    }

    /// Returns the name of the type as it would appear in smali
    ///
    /// This will only fail on
    pub fn as_smali_str(&self) -> Cow<'a, str> {
        let (smali, size) = match *self {
            Type::Primitive(prim, size) => (prim.as_smali_str(), size),
            Type::Class(cd, size) => (cd.as_str(), size),
        };

        if size == 0 {
            Cow::Borrowed(smali)
        } else {
            let mut s = String::with_capacity(smali.len() + (size as usize));
            for _ in 0..size {
                s.push('[');
            }
            s.push_str(smali);
            Cow::Owned(s)
        }
    }

    /// Returns the name of the type as it would appear in Java source code.
    pub fn as_java_str(&self, fully_qualified: bool) -> Cow<'a, str> {
        let (pkg, java, size) = match *self {
            Type::Primitive(prim, size) => (None, prim.as_java_str(), size),
            Type::Class(cd, size) => (
                if fully_qualified {
                    cd.get_java_package()
                } else {
                    None
                },
                cd.get_simple_class(),
                size,
            ),
        };

        if size == 0 {
            return match pkg {
                None => Cow::Borrowed(java),
                Some(v) => {
                    let mut s = String::with_capacity(v.len() + 1 + java.len());
                    s.push_str(&v);
                    s.push('.');
                    s.push_str(java);
                    Cow::Owned(s)
                }
            };
        }

        let needed = java.len() + 2 * (size as usize);
        let mut s = if let Some(pkg) = &pkg {
            let mut s = String::with_capacity(needed + pkg.len() + 1);
            s.push_str(pkg);
            s.push('.');
            s
        } else {
            String::with_capacity(needed)
        };

        s.push_str(java);
        for _ in 0..size {
            s.push_str("[]");
        }

        Cow::Owned(s)
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
    pub fn new_class(clazz: &'a SmaliClassName) -> Self {
        Self::Class(clazz, 0)
    }

    #[inline]
    pub fn new_class_array(clazz: &'a SmaliClassName, dim: u8) -> Self {
        Self::Class(clazz, dim)
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
    use crate::SmaliClassName;

    #[test]
    fn parse_smali() {
        macro_rules! test_parse {
            ($smali:literal, $kind:ident, $expected:expr) => {
                test_parse!($smali, $kind, $expected, 0);
            };
            ($smali:literal, $kind:ident, $expected:expr, $dim:literal) => {
                let parsed = Type::parse_smali($smali).expect(concat!("should parse:", $smali));
                assert_eq!(parsed, Type::$kind($expected, $dim));
            };
        }

        test_parse!("I", Primitive, Primitive::Int);
        test_parse!("J", Primitive, Primitive::Long);
        test_parse!("S", Primitive, Primitive::Short);
        test_parse!("B", Primitive, Primitive::Byte);
        test_parse!("C", Primitive, Primitive::Char);
        test_parse!("F", Primitive, Primitive::Float);
        test_parse!("D", Primitive, Primitive::Double);
        test_parse!("Z", Primitive, Primitive::Bool);
        test_parse!("V", Primitive, Primitive::Void);

        test_parse!("[I", Primitive, Primitive::Int, 1);
        test_parse!("[[J", Primitive, Primitive::Long, 2);

        test_parse!("La;", Class, SmaliClassName::new("La;"));
        test_parse!("La/b/C;", Class, SmaliClassName::new("La/b/C;"));
        test_parse!("[La;", Class, SmaliClassName::new("La;"), 1);
        test_parse!("[[La;", Class, SmaliClassName::new("La;"), 2);
    }

    #[test]
    fn as_java_str() {
        macro_rules! test_java_str {
            (owned $ty:expr, $expected:literal, $qual:literal) => {{
                let as_str: Cow<'_, str> = $ty.as_java_str($qual);
                let expected: Cow<'_, str> = Cow::Owned(String::from($expected));
                assert_eq!(as_str, expected);
            }};

            (borrowed $ty:expr, $expected:literal, $qual:literal) => {{
                let as_str = $ty.as_java_str($qual);
                assert_eq!(as_str, Cow::Borrowed($expected));
            }};
        }

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

        test_java_str!(borrowed Type::Class(SmaliClassName::new("Lfoo/bar/Baz;"), 0), "Baz", false);
        test_java_str!(owned Type::Class(SmaliClassName::new("Lfoo/bar/Baz;"), 1), "Baz[]", false);
        test_java_str!(owned Type::Class(SmaliClassName::new("Lfoo/bar/Baz;"), 2), "Baz[][]", false);
        test_java_str!(owned Type::Class(SmaliClassName::new("Lfoo/bar/Baz;"), 0), "foo.bar.Baz", true);
        test_java_str!(owned Type::Class(SmaliClassName::new("Lfoo/bar/Baz;"), 1), "foo.bar.Baz[]", true);
        test_java_str!(owned Type::Class(SmaliClassName::new("Lfoo/bar/Baz;"), 2), "foo.bar.Baz[][]", true);
    }
    #[test]
    fn as_smali_str() {
        macro_rules! test_smali_str {
            ($ty:expr) => {
                let as_str = $ty.as_smali_str();
                assert!(as_str.is_none());
            };

            (owned $ty:expr, $expected:literal) => {{
                let as_str: Cow<'_, str> = $ty.as_smali_str();
                let expected: Cow<'_, str> = Cow::Owned(String::from($expected));
                assert_eq!(as_str, expected);
            }};

            (borrowed $ty:expr, $expected:literal) => {{
                let as_str = $ty.as_smali_str();
                assert_eq!(as_str, Cow::Borrowed($expected));
            }};
        }

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

        test_smali_str!(borrowed Type::Class(SmaliClassName::new("Lfoo/bar/Baz;"), 0), "Lfoo/bar/Baz;");
        test_smali_str!(owned Type::Class(SmaliClassName::new("Lfoo/bar/Baz;"), 1), "[Lfoo/bar/Baz;");
        test_smali_str!(owned Type::Class(SmaliClassName::new("Lfoo/bar/Baz;"), 2), "[[Lfoo/bar/Baz;");
    }
}
