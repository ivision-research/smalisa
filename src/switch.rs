use crate::utils::ptr_eq;
use crate::{Label, RawLabel};
use std::collections::HashMap;

/// All switch data parses into the same type which is just a map of
/// label -> data.
#[derive(Debug, Clone)]
pub struct SwitchData {
    pub label_id: u32,
    map: HashMap<Label, isize>,
}

impl std::ops::Deref for SwitchData {
    type Target = HashMap<Label, isize>;

    fn deref(&self) -> &Self::Target {
        &self.map
    }
}

impl std::ops::DerefMut for SwitchData {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.map
    }
}

impl PartialEq for SwitchData {
    fn eq(&self, oth: &SwitchData) -> bool {
        if ptr_eq(self, oth) || ptr_eq(&self.map, &oth.map) {
            return true;
        }
        self.label_id == oth.label_id && self.map == oth.map
    }
}

impl SwitchData {
    #[inline(always)]
    pub const fn get_data(&self) -> &HashMap<Label, isize> {
        &self.map
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct RawSwitchPair<'a> {
    pub num: &'a str,
    pub label: RawLabel<'a>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct RawSparseSwitchData<'a> {
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
pub struct RawPackedSwitchData<'a> {
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

fn parse_num(snum: &str) -> Option<isize> {
    let is_neg = snum.as_bytes()[0] == b'-';
    if is_neg {
        let raw = snum.trim_start_matches("-0x");
        isize::from_str_radix(raw, 16).map(|s| -s).ok()
    } else {
        let raw = snum.trim_start_matches("0x");
        isize::from_str_radix(raw, 16).ok()
    }
}

impl<'a> RawPackedSwitchData<'a> {
    pub fn to_parsed(&self) -> Option<SwitchData> {
        let num = self.label.split('_').nth(2)?;
        let lbl_val = u32::from_str_radix(num, 16).ok()?;
        let num = parse_num(self.start)?;
        let mut data = SwitchData {
            label_id: lbl_val,
            map: HashMap::new(),
        };
        for (i, lab) in self.labels.iter().enumerate() {
            data.insert(lab.to_label()?, i as isize + num);
        }
        Some(data)
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
        let num = self.label.split('_').nth(2)?;
        let lbl_val = u32::from_str_radix(num, 16).ok()?;
        let mut data = SwitchData {
            label_id: lbl_val,
            map: HashMap::new(),
        };
        for d in &self.data {
            let num = parse_num(d.num)?;
            let lab = d.label.to_label()?;
            data.insert(lab, num);
        }
        Some(data)
    }

    #[inline]
    pub fn to_parsed_unchecked(&self) -> SwitchData {
        self.to_parsed().unwrap_or_else(|| {
            panic!("bad raw sparse switch data {:?}", self);
        })
    }
}

/*
impl<'a> RawSwitchData<'a> {
    pub fn to_parsed(&self) -> Option<SwitchData> {
        todo!();
    }

    fn to_parsed_sparsed(pairs: &Vec<RawSwitchPair<'a>>) -> Option<SwitchData> {
        todo!();
    }

    fn to_parsed_packed(num: &'a str, labels: &Vec<RawLabel<'a>>) -> Option<SwitchData> {
        todo!();
        //let is_neg = self.num.as_bytes()[0] == b'-';
        //let num = if is_neg {
        //    let raw = self.num.trim_start_matches("-0x");
        //    -isize::from_str_radix(self.num, 16).ok()?
        //} else {
        //    let raw = self.num.trim_start_matches("0x");
        //    isize::from_str_radix(self.num, 16).ok()
        //};
        //Some(SwitchData {
        //    num,
        //    label: self.label.to_label(),
        //})
    }

    #[inline]
    pub fn to_parsed_unchecked(&self) -> SwitchData {
        self.to_parsed().expect("unchecked call failed")
    }
}
*/
