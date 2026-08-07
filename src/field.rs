use std::hash::Hash;

use crate::{AccessFlag, Annotation, Literal, RawLiteral, Type};

#[derive(Debug, Default, Clone, PartialEq, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
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
pub struct Field<'a> {
    pub name: &'a str,
    pub access: AccessFlag,
    pub ty: Type<'a>,
    pub annotations: Vec<Annotation<'a>>,
    pub raw_value: RawLiteral<'a>,

    value: Option<Literal<'a>>,
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
            value: None,
        }
    }
}

impl<'a> Field<'a> {
    #[inline]
    pub fn has_literal_value(&self) -> bool {
        self.value.is_some() || self.raw_value != RawLiteral::Unset
    }

    /// Gets the literal value but doesn't cache it. Useful for when you
    /// are only able to work with an immutable referecnce, otherwise use
    /// [get_literal_value_mut].
    ///
    /// Note that you can also just use the RawLiteral value in raw_value.
    /// For nonnumeric types, this should be easy to deal with.
    pub fn get_literal_value(&self) -> Option<Literal<'a>> {
        if let Some(v) = self.value {
            return Some(v);
        }
        self.raw_value.to_literal()
    }

    /// Gets the literal value and caches it speed up retrieval next time.
    pub fn get_literal_value_mut(&mut self) -> Option<Literal<'a>> {
        if let Some(v) = self.value {
            return Some(v);
        }
        // TODO we don't ensure that type and value match but that is fine
        // since we're assuming valid smali I guess
        self.value = self.raw_value.to_literal();
        // It's never gonna work
        if self.value.is_none() {
            self.raw_value = RawLiteral::Unset;
        }
        self.value
    }
}
