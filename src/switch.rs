use crate::{Label, RawLabel};

/// A single `key -> label` entry of a packed or sparse switch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct SwitchCase {
    pub key: i32,
    pub label: Label,
}

/// All switch data parses into the same type: the ordered list of cases.
///
/// Several keys may target the same [Label] Cases are kept in source order, which for a packed
/// switch means ascending key order.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct SwitchData {
    pub label_id: u32,
    cases: Vec<SwitchCase>,
}

impl std::ops::Deref for SwitchData {
    type Target = [SwitchCase];

    fn deref(&self) -> &Self::Target {
        &self.cases
    }
}

impl SwitchData {
    /// The cases in source order.
    #[inline]
    pub fn cases(&self) -> &[SwitchCase] {
        &self.cases
    }

    /// The label a given key branches to, if the key is present.
    pub fn label_for(&self, key: i32) -> Option<Label> {
        self.cases
            .iter()
            .find(|case| case.key == key)
            .map(|case| case.label)
    }

    /// Every key that branches to `label`.
    pub fn keys_for(&self, label: Label) -> impl Iterator<Item = i32> + '_ {
        self.cases
            .iter()
            .filter(move |case| case.label == label)
            .map(|case| case.key)
    }

    /// Every branch target in source order. A label appears once per case, so
    /// callers building a CFG should deduplicate.
    pub fn targets(&self) -> impl Iterator<Item = Label> + '_ {
        self.cases.iter().map(|case| case.label)
    }
}

#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct RawSwitchPair<'a> {
    pub num: &'a str,
    #[cfg_attr(feature = "serde", serde(borrow))]
    pub label: RawLabel<'a>,
}

#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct RawSparseSwitchData<'a> {
    #[cfg_attr(feature = "serde", serde(borrow))]
    pub label: RawLabel<'a>,
    pub data: Vec<RawSwitchPair<'a>>,
}

impl<'a> RawSparseSwitchData<'a> {
    #[inline]
    pub fn new(label: RawLabel<'a>) -> Self {
        Self {
            label,
            data: Vec::new(),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct RawPackedSwitchData<'a> {
    #[cfg_attr(feature = "serde", serde(borrow))]
    pub label: RawLabel<'a>,
    pub start: &'a str,
    pub labels: Vec<RawLabel<'a>>,
}

impl<'a> RawPackedSwitchData<'a> {
    #[inline]
    pub fn new(label: RawLabel<'a>, start: &'a str) -> Self {
        Self {
            label,
            start,
            labels: Vec::new(),
        }
    }
}

/// Parse a switch key. Dex switch keys are 32 bit signed, so anything that
/// doesn't fit is rejected rather than truncated.
fn parse_num(snum: &str) -> Option<i32> {
    let (is_neg, rest) = match snum.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, snum),
    };
    let digits = rest.strip_prefix("0x").unwrap_or(rest);
    let magnitude = i64::from(u32::from_str_radix(digits, 16).ok()?);
    let value = if is_neg { -magnitude } else { magnitude };
    i32::try_from(value).ok()
}

/// The numeric suffix of a `:pswitch_data_N` or `:sswitch_data_N` label.
fn parse_label_id(label: &RawLabel) -> Option<u32> {
    u32::from_str_radix(label.split('_').nth(2)?, 16).ok()
}

impl<'a> RawPackedSwitchData<'a> {
    pub fn to_parsed(&self) -> Option<SwitchData> {
        let label_id = parse_label_id(&self.label)?;
        let start = parse_num(self.start)?;

        let mut cases = Vec::with_capacity(self.labels.len());
        for (offset, label) in self.labels.iter().enumerate() {
            let key = start.checked_add(i32::try_from(offset).ok()?)?;
            cases.push(SwitchCase {
                key,
                label: label.to_label()?,
            });
        }

        Some(SwitchData { label_id, cases })
    }

    #[inline]
    pub fn to_parsed_unchecked(&self) -> SwitchData {
        self.to_parsed().unwrap_or_else(|| {
            panic!("bad raw packed switch data {:?}", self);
        })
    }
}

impl<'a> RawSparseSwitchData<'a> {
    pub fn to_parsed(&self) -> Option<SwitchData> {
        let label_id = parse_label_id(&self.label)?;

        let mut cases = Vec::with_capacity(self.data.len());
        for pair in &self.data {
            cases.push(SwitchCase {
                key: parse_num(pair.num)?,
                label: pair.label.to_label()?,
            });
        }

        Some(SwitchData { label_id, cases })
    }

    #[inline]
    pub fn to_parsed_unchecked(&self) -> SwitchData {
        self.to_parsed().unwrap_or_else(|| {
            panic!("bad raw sparse switch data {:?}", self);
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn packed(start: &'static str, labels: &[&'static str]) -> SwitchData {
        let mut raw = RawPackedSwitchData::new(RawLabel::new("pswitch_data_0"), start);
        raw.labels = labels.iter().map(|l| RawLabel::new(l)).collect();
        raw.to_parsed().expect("failed to parse packed switch")
    }

    fn sparse(pairs: &[(&'static str, &'static str)]) -> SwitchData {
        let mut raw = RawSparseSwitchData::new(RawLabel::new("sswitch_data_0"));
        raw.data = pairs
            .iter()
            .map(|(num, label)| RawSwitchPair {
                num,
                label: RawLabel::new(label),
            })
            .collect();
        raw.to_parsed().expect("failed to parse sparse switch")
    }

    #[test]
    fn packed_keys_ascend_from_start() {
        let data = packed("0x1", &["pswitch_0", "pswitch_1", "pswitch_2"]);
        assert_eq!(data.label_id, 0);
        let keys: Vec<i32> = data.cases().iter().map(|c| c.key).collect();
        assert_eq!(keys, vec![1, 2, 3]);
        assert_eq!(data.label_for(2), Some(Label::PackedSwitch(1)));
        assert_eq!(data.label_for(9), None);
    }

    #[test]
    fn packed_negative_start() {
        let data = packed("-0x2", &["pswitch_0", "pswitch_1"]);
        let keys: Vec<i32> = data.cases().iter().map(|c| c.key).collect();
        assert_eq!(keys, vec![-2, -1]);
    }

    #[test]
    fn duplicate_targets_keep_every_key() {
        // The map-keyed-by-label representation used to drop keys 1 and 2 here.
        let data = packed("0x1", &["pswitch_0", "pswitch_1", "pswitch_0"]);
        assert_eq!(data.cases().len(), 3);
        let keys: Vec<i32> = data.keys_for(Label::PackedSwitch(0)).collect();
        assert_eq!(keys, vec![1, 3]);
        let targets: Vec<Label> = data.targets().collect();
        assert_eq!(
            targets,
            vec![
                Label::PackedSwitch(0),
                Label::PackedSwitch(1),
                Label::PackedSwitch(0)
            ]
        );
    }

    #[test]
    fn sparse_preserves_source_order() {
        let data = sparse(&[("0x0", "sswitch_0"), ("0x64", "sswitch_1")]);
        let keys: Vec<i32> = data.cases().iter().map(|c| c.key).collect();
        assert_eq!(keys, vec![0, 100]);
        assert_eq!(data.label_for(100), Some(Label::SparseSwitch(1)));
    }

    #[test]
    fn keys_outside_i32_are_rejected() {
        assert_eq!(parse_num("0x7fffffff"), Some(i32::MAX));
        assert_eq!(parse_num("-0x80000000"), Some(i32::MIN));
        assert_eq!(parse_num("0x80000000"), None);
        assert_eq!(parse_num("-0x80000001"), None);
        assert_eq!(parse_num(""), None);
    }
}
