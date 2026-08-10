use std::borrow::Cow;
use std::hash::Hash;
use std::ops::{Deref, DerefMut};

use crate::instructions::Invocation;
use crate::utils::ptr_eq;
use crate::{
    AccessFlag, Annotation, ArrayData, Catch, Label, Line, ParamAnnotations, Primitive, SwitchData,
    Type,
};
use smallvec::SmallVec;
pub fn parse_method_args_into<'a>(args: &'a str, into: &mut Vec<Type<'a>>) -> Result<(), &'a str> {
    if args.is_empty() {
        return Ok(());
    }
    let mut bytes = args.bytes();
    let mut start = 0;
    let mut dim = 0;
    while let Some(b) = bytes.next() {
        match b {
            b'[' => {
                dim += 1;
                start += 1;
                continue;
            }
            b'L' => {
                let mut end = start;
                loop {
                    if let Some(b) = bytes.next() {
                        end += 1;
                        if b == b';' {
                            into.push(Type::new_class_array(&args[start..end + 1], dim));
                            start = end;
                            break;
                        }
                    } else {
                        // malformed
                        return Err(&args[start..end]);
                    }
                }
            }
            b'J' => {
                into.push(Type::new_prim_array(Primitive::Long, dim));
            }
            b'C' => {
                into.push(Type::new_prim_array(Primitive::Char, dim));
            }
            b'I' => {
                into.push(Type::new_prim_array(Primitive::Int, dim));
            }
            b'B' => {
                into.push(Type::new_prim_array(Primitive::Byte, dim));
            }
            b'S' => {
                into.push(Type::new_prim_array(Primitive::Short, dim));
            }
            b'Z' => {
                into.push(Type::new_prim_array(Primitive::Bool, dim));
            }
            b'V' => {
                into.push(Type::new_prim_array(Primitive::Void, dim));
            }
            b'F' => {
                into.push(Type::new_prim_array(Primitive::Float, dim));
            }
            b'D' => {
                into.push(Type::new_prim_array(Primitive::Double, dim));
            }
            _ => {
                return Err(&args[start..]);
            }
        }
        start += 1;
        dim = 0;
    }
    Ok(())
}

#[inline]
pub fn parse_method_args<'a>(args: &'a str) -> Result<Vec<Type<'a>>, &'a str> {
    let mut into = Vec::new();
    parse_method_args_into(args, &mut into)?;
    Ok(into)
}

#[derive(Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct MethodRef<'a> {
    pub class: &'a str,
    pub name: &'a str,
    pub args: &'a str,
    #[cfg_attr(feature = "serde", serde(borrow))]
    pub return_type: Type<'a>,

    #[cfg_attr(feature = "serde", serde(skip))]
    input_params: Option<Vec<Type<'a>>>,

    pub(crate) class_array_dim: usize,
}

impl<'a> Hash for MethodRef<'a> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.class.hash(state);
        self.name.hash(state);
        self.args.hash(state);
        self.class_array_dim.hash(state);
        // Ignore input params because they're derived from args
    }
}

impl<'a> MethodRef<'a> {
    pub fn new(class: &'a str, name: &'a str, args: &'a str, ret: Type<'a>) -> Self {
        Self {
            class,
            name,
            args,
            return_type: ret,
            input_params: None,
            class_array_dim: 0,
        }
    }

    #[inline(always)]
    pub fn class_is_array(&self) -> bool {
        self.class_array_dim > 0
    }

    /// The string representation of the class name. This is a Cow because
    /// we might actually have to add some [ in front of it is is an array.
    /// In most cases this will be a Cow::Borrowed version of the class
    /// field though.
    pub fn full_class_str(&self) -> Cow<'a, str> {
        if self.class_array_dim > 0 {
            let mut s = "[".repeat(self.class_array_dim);
            s.push_str(self.class);
            Cow::Owned(s)
        } else {
            Cow::Borrowed(self.class)
        }
    }

    /// Used to get input arguments as a vec and optionally parse them.
    pub fn get_or_parse_input_args(&mut self) -> Option<&Vec<Type<'a>>> {
        if self.args.is_empty() {
            return None;
        } else if self.input_params.is_some() {
            return self.input_params.as_ref();
        }
        self.input_params = parse_method_args(self.args).ok();
        self.input_params.as_ref()
    }

    /// Used to get input arguments when they have already been parsed.
    #[inline(always)]
    pub fn get_input_args(&self) -> Option<&Vec<Type<'a>>> {
        self.input_params.as_ref()
    }
}

#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct MethodHeader<'a> {
    pub name: &'a str,
    pub access: AccessFlag,
    pub args: &'a str,
    pub return_type: Type<'a>,

    #[cfg_attr(feature = "serde", serde(skip))]
    input_params: Option<Vec<Type<'a>>>,
}

impl<'a> PartialEq for MethodHeader<'a> {
    fn eq(&self, other: &MethodHeader<'a>) -> bool {
        if ptr_eq(self, other) {
            return true;
        }
        if self.name != other.name
            || self.access != other.access
            || self.return_type != other.return_type
        {
            return false;
        }
        self.args == other.args
    }
}

impl<'a> MethodHeader<'a> {
    pub fn new(name: &'a str, access: AccessFlag, args: &'a str, ret: Type<'a>) -> Self {
        Self {
            name,
            access,
            args,
            return_type: ret,
            input_params: None,
        }
    }

    /// Used to get input arguments as a vec and optionally parse them.
    pub fn get_or_parse_input_args(&mut self) -> Option<&Vec<Type<'a>>> {
        if self.args.is_empty() {
            return None;
        } else if self.input_params.is_some() {
            return self.input_params.as_ref();
        }
        self.input_params = parse_method_args(self.args).ok();
        self.input_params.as_ref()
    }

    /// Used to get input arguments when they have already been parsed.
    #[inline(always)]
    pub fn get_input_args(&self) -> Option<&Vec<Type<'a>>> {
        self.input_params.as_ref()
    }
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum MethodLine<'a> {
    Unset,
    #[cfg_attr(feature = "serde", serde(borrow))]
    Instruction(Invocation<'a>),
    LabelDef(Label),
    #[cfg_attr(feature = "serde", serde(borrow))]
    Catch(Catch<'a>),
}

impl<'a> Default for MethodLine<'a> {
    #[inline(always)]
    fn default() -> Self {
        Self::Unset
    }
}

/// Represents a fully parsed method.
#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Method<'a> {
    #[cfg_attr(feature = "serde", serde(borrow))]
    pub header: MethodHeader<'a>,
    pub annotations: SmallVec<[Annotation<'a>; 4]>,
    #[cfg_attr(feature = "serde", serde(borrow))]
    pub param_annotations: Option<Vec<ParamAnnotations<'a>>>,

    pub lines: Vec<MethodLine<'a>>,

    pub packed_switch_data: Vec<SwitchData>,
    pub sparse_switch_data: Vec<SwitchData>,
    pub array_data: Vec<ArrayData>,
}

pub struct MethodLineBuilder<'a> {
    method: Method<'a>,
}

impl<'a> Deref for MethodLineBuilder<'a> {
    type Target = Method<'a>;
    fn deref(&self) -> &Self::Target {
        &self.method
    }
}

impl<'a> DerefMut for MethodLineBuilder<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.method
    }
}

impl<'a> MethodLineBuilder<'a> {
    pub fn new(mh: &MethodHeader<'a>) -> Self {
        Self {
            method: Method {
                header: mh.clone(),
                ..Default::default()
            },
        }
    }

    /// Push a [Line] into the Method.
    ///
    /// This can be helpful for manually building Methods from [Line]s but note that this type is
    /// completely unaware of state: [Line::MethodEnd] and [Line::MethodHeader] are silently
    /// ignored. It is up to the driver to manage state correctly!
    pub fn push_line(&mut self, line: &Line<'a>) {
        // TODO remove the _unchecked maybe
        match line {
            Line::Annotation(ref ann) => {
                self.annotations.push(ann.clone());
            }
            Line::InstructionInvocation(ref inv) => {
                self.lines.push(MethodLine::Instruction(inv.clone()));
            }
            Line::LabelDefinition(lab) => {
                let parsed = lab.to_label_unchecked();
                self.lines.push(MethodLine::LabelDef(parsed));
            }
            Line::NamedCatch(catch) => {
                self.lines
                    .push(MethodLine::Catch(Catch::Named(catch.to_parsed_unchecked())));
            }
            Line::CatchAll(catch) => {
                self.lines.push(MethodLine::Catch(Catch::All(
                    catch.to_parsed_all_unchecked(),
                )));
            }
            Line::PackedSwitchData(ref psd) => {
                self.packed_switch_data.push(psd.to_parsed_unchecked());
            }
            Line::SparseSwitchData(ref ssd) => {
                self.sparse_switch_data.push(ssd.to_parsed_unchecked());
            }
            Line::ArrayData(ref ad) => {
                self.array_data.push(ad.to_parsed_unchecked());
            }
            Line::ParamLine(reg, name, ref annotations) => {
                if let Some(ann) = annotations {
                    let pa = ParamAnnotations::new(*reg, name, ann.clone());
                    if let Some(v) = &mut self.param_annotations {
                        v.push(pa);
                    } else {
                        self.param_annotations = Some(vec![pa]);
                    }
                }
            }

            _ => {}
        }
    }

    pub fn finish(self) -> Method<'a> {
        self.method
    }
}

impl<'a> PartialEq for Method<'a> {
    fn eq(&self, other: &Method<'a>) -> bool {
        if ptr_eq(self, other) {
            return true;
        }
        if self.header != other.header {
            return false;
        }
        self.annotations == other.annotations
            && self.sparse_switch_data == other.sparse_switch_data
            && self.packed_switch_data == other.packed_switch_data
            && self.array_data == other.array_data
            && self.lines == other.lines
    }
}

simple_deref!(Method<'a>, header, MethodHeader<'a>, 'a);

#[cfg(test)]
mod test {
    use super::*;

    macro_rules! tpma {
        ($args:literal, $($ty:expr),*) => {
            assert_eq!(
                parse_method_args($args).expect(concat!("failed to parse", $args)),
                vec![
                $(
                    $ty
                ),*
                ]
            );
        }
    }

    #[test]
    fn test_parse_method_args() {
        tpma!(
            "Labc;JCIBSZVFDLadf;Lc;",
            Type::Class("Labc;", 0),
            Type::Primitive(Primitive::Long, 0),
            Type::Primitive(Primitive::Char, 0),
            Type::Primitive(Primitive::Int, 0),
            Type::Primitive(Primitive::Byte, 0),
            Type::Primitive(Primitive::Short, 0),
            Type::Primitive(Primitive::Bool, 0),
            Type::Primitive(Primitive::Void, 0),
            Type::Primitive(Primitive::Float, 0),
            Type::Primitive(Primitive::Double, 0),
            Type::Class("Ladf;", 0),
            Type::Class("Lc;", 0)
        );

        tpma!(
            "[[Z[[Lc;[D[[[F",
            Type::Primitive(Primitive::Bool, 2),
            Type::Class("Lc;", 2),
            Type::Primitive(Primitive::Double, 1),
            Type::Primitive(Primitive::Float, 3)
        );
    }
}
