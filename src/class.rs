use std::borrow::Cow;

use crate::{AccessFlag, Annotation, Field, Method};

/// Represents a fully parsed class.
#[derive(Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Class<'a> {
    pub access: AccessFlag,
    pub name: &'a str,
    pub parent: &'a str,
    pub interfaces: Vec<&'a str>,
    pub annotations: Vec<Annotation<'a>>,
    pub methods: Vec<Method<'a>>,
    pub fields: Vec<Field<'a>>,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct PackageClass<'a> {
    pub package: &'a str,
    pub name: &'a str,
}

impl<'a> PackageClass<'a> {
    pub fn from_lclass(s: &'a str) -> Option<Self> {
        if let Some(idx) = s.rfind('/') {
            let (package, name) = s.split_at(idx);
            let package = package.get(1..)?;
            let name = name.get(1..name.len() - 1)?;
            if name.is_empty() || package.is_empty() {
                None
            } else {
                Some(Self { package, name })
            }
        } else {
            let name = s.get(1..s.len() - 1)?;
            if name.is_empty() {
                None
            } else {
                Some(Self { package: "", name })
            }
        }
    }

    pub fn dot_package(&self) -> Cow<'a, str> {
        if self.package.contains('/') {
            Cow::Owned(self.package.replace('/', "."))
        } else {
            Cow::Borrowed(self.package)
        }
    }

    pub fn as_java_str(&self) -> String {
        format!("{}.{}", self.dot_package(), self.name)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn split_package_name() {
        let class = "Lfoo;";
        let split = PackageClass::from_lclass(class);
        assert!(split.is_some());
        let un = split.unwrap();
        assert_eq!(un.package, "");
        assert_eq!(un.name, "foo");

        let class = "Ljava/lang/String;";
        let split = PackageClass::from_lclass(class);
        assert!(split.is_some());
        let un = split.unwrap();
        assert_eq!(un.package, "java/lang");
        assert_eq!(un.name, "String");

        let class = "La/b;";
        let split = PackageClass::from_lclass(class);
        assert!(split.is_some());
        let un = split.unwrap();
        assert_eq!(un.package, "a");
        assert_eq!(un.name, "b");

        macro_rules! assert_none {
            ($split:ident) => {
                assert!($split.is_none(), "expected none but got {:?}", $split);
            };
        }

        let class = "a/b";
        let split = PackageClass::from_lclass(class);
        assert_none!(split);

        let class = "La/b";
        let split = PackageClass::from_lclass(class);
        assert_none!(split);

        let class = "a/b;";
        let split = PackageClass::from_lclass(class);
        assert_none!(split);

        let class = "a;";
        let split = PackageClass::from_lclass(class);
        assert_none!(split);

        let class = "La";
        let split = PackageClass::from_lclass(class);
        assert_none!(split);
    }
}
