use crate::{Label, RawLabel};

#[derive(Debug, PartialEq, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "yoke", derive(yoke::Yokeable))]
pub enum Catch<'a> {
    #[cfg_attr(feature = "serde", serde(borrow))]
    Named(NamedCatch<'a>),
    All(CatchAll),
}

#[derive(Debug, PartialEq, Clone, Default, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "yoke", derive(yoke::Yokeable))]
pub struct NamedCatch<'a> {
    pub class: &'a str,
    pub start_label: Label,
    pub end_label: Label,
    pub dest_label: Label,
}

#[derive(Debug, PartialEq, Clone, Default, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "yoke", derive(yoke::Yokeable))]
pub struct RawNamedCatch<'a> {
    pub class: &'a str,
    #[cfg_attr(feature = "serde", serde(borrow))]
    pub start_label: RawLabel<'a>,
    #[cfg_attr(feature = "serde", serde(borrow))]
    pub end_label: RawLabel<'a>,
    #[cfg_attr(feature = "serde", serde(borrow))]
    pub dest_label: RawLabel<'a>,
}

impl<'a> RawNamedCatch<'a> {
    pub fn new(
        class: &'a str,
        start_label: RawLabel<'a>,
        end_label: RawLabel<'a>,
        dest_label: RawLabel<'a>,
    ) -> Self {
        Self {
            class,
            start_label,
            end_label,
            dest_label,
        }
    }

    pub fn to_parsed(&self) -> Option<NamedCatch<'a>> {
        Some(NamedCatch {
            class: self.class,
            start_label: self.start_label.to_label()?,
            end_label: self.end_label.to_label()?,
            dest_label: self.dest_label.to_label()?,
        })
    }

    pub fn to_parsed_unchecked(&self) -> NamedCatch<'a> {
        NamedCatch {
            class: self.class,
            start_label: self.start_label.to_label_unchecked(),
            end_label: self.end_label.to_label_unchecked(),
            dest_label: self.dest_label.to_label_unchecked(),
        }
    }
}

#[derive(Debug, PartialEq, Clone, Default, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "yoke", derive(yoke::Yokeable))]
pub struct CatchAll {
    pub start_label: Label,
    pub end_label: Label,
    pub dest_label: Label,
}

#[derive(Debug, PartialEq, Clone, Default, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "yoke", derive(yoke::Yokeable))]
pub struct RawCatchAll<'a> {
    #[cfg_attr(feature = "serde", serde(borrow))]
    pub start_label: RawLabel<'a>,
    #[cfg_attr(feature = "serde", serde(borrow))]
    pub end_label: RawLabel<'a>,
    #[cfg_attr(feature = "serde", serde(borrow))]
    pub dest_label: RawLabel<'a>,
}

impl<'a> RawCatchAll<'a> {
    pub fn new(
        start_label: RawLabel<'a>,
        end_label: RawLabel<'a>,
        dest_label: RawLabel<'a>,
    ) -> Self {
        Self {
            start_label,
            end_label,
            dest_label,
        }
    }

    pub fn to_parsed_all(&self) -> Option<CatchAll> {
        Some(CatchAll {
            start_label: self.start_label.to_label()?,
            end_label: self.end_label.to_label()?,
            dest_label: self.dest_label.to_label()?,
        })
    }

    pub fn to_parsed_all_unchecked(&self) -> CatchAll {
        CatchAll {
            start_label: self.start_label.to_label_unchecked(),
            end_label: self.end_label.to_label_unchecked(),
            dest_label: self.dest_label.to_label_unchecked(),
        }
    }
}
