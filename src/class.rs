use std::borrow::Cow;

use crate::{method::MethodLineBuilder, AccessFlag, Annotation, Field, Line, Method};

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

pub struct ClassLineBuilder<'a> {
    pub class: Class<'a>,

    method: Option<MethodLineBuilder<'a>>,
}

impl<'a> Default for ClassLineBuilder<'a> {
    fn default() -> Self {
        Self {
            class: Class::default(),
            method: None,
        }
    }
}

impl<'a> ClassLineBuilder<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a [Line] into the builder
    ///
    /// Note that this function is not state aware, a new [Line::Class] or other class related lines
    /// will overwrite any previously set state. It is up to the caller to manage state and ensure
    /// only a single class's [Line]s are pushed into the builder
    pub fn push_line(&mut self, line: &Line<'a>) {
        if matches!(line, Line::MethodEnd) {
            if let Some(method) = self.method.take() {
                self.class.methods.push(method.finish());
            }
        } else if let Some(method) = &mut self.method {
            method.push_line(line);
        } else {
            match line {
                Line::Class(acc, cd) => {
                    self.class.access = *acc;
                    self.class.name = cd;
                }
                Line::Super(sup) => {
                    self.class.parent = sup;
                }
                Line::Interface(inf) => {
                    self.class.interfaces.push(inf);
                }
                Line::MethodHeader(ref mh) => {
                    self.method = Some(MethodLineBuilder::new(mh));
                }
                Line::Field(ref field) => {
                    self.class.fields.push(field.clone());
                }
                Line::Annotation(ref ann) => {
                    self.class.annotations.push(ann.clone());
                }
                _ => {}
            }
        }
    }

    pub fn finish(self) -> Class<'a> {
        // Note that we don't take out of `self.method` here because that would mean it is an
        // incomplete method: methods should always be taken out when a Line::MethodEnd is
        // discovered during building
        self.class
    }
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
