use std::fmt;
use std::ops::Deref;

/// A raw label with the : removed.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RawLabel<'a>(&'a str);

impl<'a> From<&'a str> for RawLabel<'a> {
    fn from(s: &'a str) -> Self {
        Self(s)
    }
}

impl<'a> Deref for RawLabel<'a> {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> fmt::Display for RawLabel<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, ":{}", self.0)
    }
}

impl<'a> RawLabel<'a> {
    #[inline(always)]
    pub fn new(s: &'a str) -> Self {
        Self(s)
    }

    #[inline(always)]
    pub fn to_label(&self) -> Option<Label> {
        Label::from_raw(self)
    }

    #[inline(always)]
    pub fn to_label_unchecked(&self) -> Label {
        Label::from_raw_unchecked(self)
    }

    #[inline]
    pub fn is_packed_switch_data(&self) -> bool {
        if self.0.as_bytes()[0] != b'p' {
            return false;
        }
        let idx = self.0.rfind('_').unwrap_or(0);
        idx > 7
    }

    #[inline]
    pub fn is_sparse_switch_data(&self) -> bool {
        if self.0.as_bytes()[0] != b's' {
            return false;
        }
        let idx = self.0.rfind('_').unwrap_or(0);
        idx > 7
    }

    #[inline]
    pub fn is_array(&self) -> bool {
        self.0.as_bytes()[0] == b'a'
    }
}

/// Represents a parsed label. Labels of the form:
///
/// :label_name_hex
///
/// Such as :goto_2f.
///
/// This enum simply removes the string part and parses the hex digit.
#[derive(PartialEq, Eq, Debug, Clone, Copy, Hash)]
pub enum Label {
    Unset,
    Cond(u32),
    Goto(u32),
    PackedSwitch(u32),
    PackedSwitchData(u32),
    SparseSwitch(u32),
    SparseSwitchData(u32),
    Catch(u32),
    CatchAll(u32),
    TryStart(u32),
    TryEnd(u32),
    Array(u32),
}

impl fmt::Display for Label {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, ":")?;
        let num = match *self {
            Self::Unset => return write!(f, "?"),
            Self::Cond(v) => {
                write!(f, "cond_")?;
                v
            }
            Self::Goto(v) => {
                write!(f, "goto_")?;
                v
            }
            Self::PackedSwitch(v) => {
                write!(f, "pswitch_")?;
                v
            }
            Self::PackedSwitchData(v) => {
                write!(f, "pswitch_data_")?;
                v
            }
            Self::SparseSwitch(v) => {
                write!(f, "sswitch_")?;
                v
            }
            Self::SparseSwitchData(v) => {
                write!(f, "sswitch_data_")?;
                v
            }
            Self::Catch(v) => {
                write!(f, "catch_")?;
                v
            }
            Self::CatchAll(v) => {
                write!(f, "catchall_")?;
                v
            }
            Self::TryStart(v) => {
                write!(f, "try_start_")?;
                v
            }
            Self::TryEnd(v) => {
                write!(f, "try_end_")?;
                v
            }
            Self::Array(v) => {
                write!(f, "array_")?;
                v
            }
        };
        write!(f, "{:x}", num)
    }
}

impl Default for Label {
    #[inline(always)]
    fn default() -> Self {
        Self::Unset
    }
}

impl Label {
    #[inline]
    pub fn from_raw_unchecked(raw: &RawLabel) -> Self {
        Self::from_simple_name(raw).unwrap_or_else(|| {
            panic!("bad label: {}", raw);
        })
    }

    #[inline]
    pub fn from_raw(raw: &RawLabel) -> Option<Self> {
        Self::from_simple_name(raw)
    }

    pub(crate) fn from_simple_name(s: &str) -> Option<Self> {
        let idx = s.rfind('_')?;
        let (name, val) = s.split_at(idx);
        // Label numbers will always be positive hex digits.
        let num = u32::from_str_radix(val.get(1..)?, 16).ok()?;
        let bytes = name.as_bytes();
        // This is another example of us assuming we've been given correct
        // smali. No reason to compare the whole thing.
        match bytes[0] {
            b'c' => match bytes[1] {
                b'o' => Some(Label::Cond(num)),
                b'a' => Some(if name.len() > 5 {
                    Label::CatchAll(num)
                } else {
                    Label::Catch(num)
                }),
                _ => None,
            },
            b'g' => Some(Label::Goto(num)),
            b's' => {
                // :sswitch_data_
                //              ^
                // :sswitch_
                //         ^
                if idx > 7 {
                    Some(Label::SparseSwitchData(num))
                } else {
                    Some(Label::SparseSwitch(num))
                }
            }
            b'p' => {
                // :pswitch_data_
                //              ^
                // :pswitch_
                //         ^
                if idx > 7 {
                    Some(Label::PackedSwitchData(num))
                } else {
                    Some(Label::PackedSwitch(num))
                }
            }
            b't' => match bytes[4] {
                b'e' => Some(Label::TryEnd(num)),
                b's' => Some(Label::TryStart(num)),
                _ => None,
            },
            b'a' => Some(Label::Array(num)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn label_from_simple_names() {
        macro_rules! test_label {
            ($s:literal, $ty:ident, $num:literal) => {
                let lbl = Label::from_simple_name($s);
                assert!(lbl.is_some(), concat!("failed to parse ", $s));
                assert_eq!(lbl.unwrap(), Label::$ty($num));
            };
        }

        test_label!("goto_2e", Goto, 0x2e);
        test_label!("cond_0", Cond, 0);
        test_label!("catch_1234", Catch, 0x1234);
        test_label!("catchall_12fe", CatchAll, 0x12fe);
        test_label!("try_start_0", TryStart, 0x0);
        test_label!("try_end_f", TryEnd, 0xf);
    }
}
