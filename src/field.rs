use std::hash::Hash;

use crate::{AccessFlag, Annotation, Literal, RawLiteral, Type};

#[derive(Debug, Default, Clone, PartialEq, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "yoke", derive(yoke::Yokeable))]
pub struct FieldRef<'a> {
    pub class: &'a str,
    pub name: &'a str,
    pub ty: Type<'a>,
}

impl<'a> Hash for FieldRef<'a> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.class.hash(state);
        self.name.hash(state);
        self.ty.hash(state);
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "yoke", derive(yoke::Yokeable))]
pub struct Field<'a> {
    pub name: &'a str,
    pub access: AccessFlag,
    pub ty: Type<'a>,
    pub annotations: Vec<Annotation<'a>>,
    pub raw_value: RawLiteral<'a>,
}

impl<'a> Field<'a> {
    pub fn new(
        name: &'a str,
        access: AccessFlag,
        ty: Type<'a>,
        raw_value: RawLiteral<'a>,
        annotations: Vec<Annotation<'a>>,
    ) -> Self {
        Self {
            name,
            access,
            ty,
            annotations,
            raw_value,
        }
    }
}

impl<'a> Field<'a> {
    #[inline]
    pub fn has_literal_value(&self) -> bool {
        self.raw_value != RawLiteral::Unset
    }

    pub fn get_literal_value(&self) -> Option<Literal<'a>> {
        self.raw_value.to_literal()
    }
}
