use crate::literal::RawLiteral;
use crate::utils::ptr_eq;
use crate::{Enum, MethodRef, Primitive, Register, Type};

#[derive(Debug, Clone, PartialEq, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum AnnotationVisibility {
    Unset,
    Runtime,
    Build,
    System,
}

impl Default for AnnotationVisibility {
    fn default() -> Self {
        Self::Unset
    }
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "yoke", derive(yoke::Yokeable))]
pub enum AnnotationValue<'a> {
    #[cfg_attr(feature = "serde", serde(borrow))]
    Lit(RawLiteral<'a>),
    List(Vec<AnnotationValue<'a>>),
    Subannotation(Annotation<'a>),
    #[cfg_attr(feature = "serde", serde(borrow))]
    Enum(Enum<'a>),
    #[cfg_attr(feature = "serde", serde(borrow))]
    Type(Type<'a>),
    #[cfg_attr(feature = "serde", serde(borrow))]
    Method(MethodRef<'a>),
}

impl<'a> From<Primitive> for AnnotationValue<'a> {
    #[inline]
    fn from(p: Primitive) -> Self {
        Self::Type(Type::new_prim(p))
    }
}

impl<'a> From<Type<'a>> for AnnotationValue<'a> {
    #[inline(always)]
    fn from(ty: Type<'a>) -> Self {
        Self::Type(ty)
    }
}

impl<'a> From<MethodRef<'a>> for AnnotationValue<'a> {
    #[inline(always)]
    fn from(mref: MethodRef<'a>) -> Self {
        Self::Method(mref)
    }
}

impl<'a> From<RawLiteral<'a>> for AnnotationValue<'a> {
    #[inline(always)]
    fn from(raw: RawLiteral<'a>) -> Self {
        Self::Lit(raw)
    }
}

impl<'a> From<Annotation<'a>> for AnnotationValue<'a> {
    #[inline(always)]
    fn from(sub: Annotation<'a>) -> Self {
        Self::Subannotation(sub)
    }
}

impl<'a> From<Enum<'a>> for AnnotationValue<'a> {
    #[inline(always)]
    fn from(en: Enum<'a>) -> Self {
        Self::Enum(en)
    }
}

#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "yoke", derive(yoke::Yokeable))]
pub struct ParamAnnotations<'a> {
    pub register: Register,
    pub name: &'a str,
    pub annotations: Vec<Annotation<'a>>,
}

impl<'a> ParamAnnotations<'a> {
    pub fn new(register: Register, name: &'a str, annotations: Vec<Annotation<'a>>) -> Self {
        Self {
            register,
            name,
            annotations,
        }
    }
}

impl<'a> PartialEq for ParamAnnotations<'a> {
    fn eq(&self, other: &ParamAnnotations<'a>) -> bool {
        if ptr_eq(self, other) {
            return true;
        }
        self.register == other.register
            && self.name == other.name
            && self.annotations == other.annotations
    }
}

#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "yoke", derive(yoke::Yokeable))]
pub struct Annotation<'a> {
    pub class: &'a str,
    pub visibility: AnnotationVisibility,
    parameters: Vec<(&'a str, AnnotationValue<'a>)>,
}

impl<'a> Annotation<'a> {
    pub fn new(class: &'a str, visibility: AnnotationVisibility) -> Self {
        Self {
            class,
            visibility,
            parameters: Vec::new(),
        }
    }

    /// Set `key`, returning the previous value if there was one.
    pub fn insert(
        &mut self,
        key: &'a str,
        value: AnnotationValue<'a>,
    ) -> Option<AnnotationValue<'a>> {
        match self.parameters.iter_mut().find(|(k, _)| *k == key) {
            Some((_, existing)) => Some(std::mem::replace(existing, value)),
            None => {
                self.parameters.push((key, value));
                None
            }
        }
    }

    pub fn get(&self, key: &str) -> Option<&AnnotationValue<'a>> {
        self.parameters
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| v)
    }

    /// The parameters in source order.
    #[inline]
    pub fn parameters(&self) -> &[(&'a str, AnnotationValue<'a>)] {
        &self.parameters
    }
}

impl<'a> PartialEq for Annotation<'a> {
    fn eq(&self, other: &Annotation<'a>) -> bool {
        if ptr_eq(self, other) {
            return true;
        }
        if self.class != other.class
            || self.visibility != other.visibility
            || self.parameters.len() != other.parameters.len()
        {
            return false;
        }
        self.parameters
            .iter()
            .all(|(key, value)| other.get(key) == Some(value))
    }
}
