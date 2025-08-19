use crate::extra::{i8_to_u8, parse_numeric};
use crate::{NumericLiteral, RawLabel};

#[derive(PartialEq, Debug, Clone, Default)]
pub struct ArrayData {
    pub label_id: u32,
    pub data_size: usize,
    pub data: Vec<NumericLiteral>,
}

impl ArrayData {
    pub const ANNONYMOUS_ID: u32 = u32::MAX;
}

#[derive(PartialEq, Debug, Clone)]
pub struct RawArrayData<'a> {
    pub label: RawLabel<'a>,
    pub data_size: &'a str,
    pub data: Vec<&'a str>,
}

impl<'a> RawArrayData<'a> {
    pub fn as_byte_array(&self) -> Option<Vec<u8>> {
        if self.data_size != "1" {
            return None;
        }
        let mut byte_vec = Vec::with_capacity(self.data.len());
        for numeric in &self.data {
            if let Some(NumericLiteral::Byte(b)) = parse_numeric(numeric) {
                byte_vec.push(i8_to_u8(b));
            } else {
                // TODO
                return None;
            }
        }
        Some(byte_vec)
    }

    pub fn to_parsed(&self) -> Option<ArrayData> {
        let ds = usize::from_str_radix(self.data_size, 10).ok()?;
        // TODO: This is handling a weird case that I've seen. The smali source has a test for
        // "LZeroArrayPayloadWidthTest" that will trigger this path, so it seems they're aware of
        // stuff like this happening.
        let label_id = if self.label.is_empty() {
            ArrayData::ANNONYMOUS_ID
        } else {
            let num = self.label.split('_').nth(1)?;
            u32::from_str_radix(num, 16).ok()?
        };
        let mut ad = ArrayData {
            label_id,
            data_size: ds,
            data: Vec::new(),
        };
        let mut nums = Vec::new();
        let mut is_float = false;
        let mut is_double = false;
        for numeric in &self.data {
            let parsed = parse_numeric(numeric)?;
            match parsed {
                NumericLiteral::Float(_) => {
                    is_float = true;
                }
                NumericLiteral::Double(_) => {
                    is_double = true;
                }
                _ => {}
            }
            nums.push(parsed);
        }
        if is_float || is_double {
            for n in nums {
                match n {
                    NumericLiteral::Float(_) | NumericLiteral::Double(_) => {
                        ad.data.push(n);
                    }
                    NumericLiteral::Long(l) => {
                        ad.data.push(if is_float {
                            NumericLiteral::Float(l as f32)
                        } else {
                            NumericLiteral::Double(l as f64)
                        });
                    }
                    NumericLiteral::Int(i) => {
                        ad.data.push(if is_float {
                            NumericLiteral::Float(i as f32)
                        } else {
                            NumericLiteral::Double(i as f64)
                        });
                    }
                    _ => {
                        // TODO
                        return None;
                    }
                }
            }
        } else {
            ad.data = nums;
        }
        Some(ad)
    }

    #[inline]
    pub fn to_parsed_unchecked(&self) -> ArrayData {
        self.to_parsed().expect("unchecked parse failed")
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn array_to_byte_array() {
        let data = RawArrayData {
            label: "array_22".into(),
            data_size: "1",
            data: vec!["0x1ft", "0x31t", "0x27t", "0x3at", "-0x43t"],
        };
        let as_bytes = data.as_byte_array().expect("should have suceeded");
        assert_eq!(as_bytes, vec![0x1f, 0x31, 0x27, 0x3a, 0xbd]);
    }

    #[test]
    fn array_raw_to_parsed() {
        let data = RawArrayData {
            label: "array_22".into(),
            data_size: "2",
            data: vec!["0x30s", "0x31s", "0x32s", "-0x33s"],
        };
        let parsed = data.to_parsed();
        assert!(parsed.is_some(), "{:?} was not some", data);
        let parsed = parsed.unwrap();
        assert_eq!(
            parsed,
            ArrayData {
                label_id: 0x22,
                data_size: 2,
                data: vec![
                    NumericLiteral::Short(0x30),
                    NumericLiteral::Short(0x31),
                    NumericLiteral::Short(0x32),
                    NumericLiteral::Short(-0x33),
                ]
            }
        );

        let data = RawArrayData {
            label: "array_2fe".into(),
            data_size: "8",
            data: vec!["1.0", "0x0"],
        };
        let parsed = data.to_parsed();
        assert!(parsed.is_some(), "{:?} was not some", data);
        let parsed = parsed.unwrap();
        assert_eq!(
            parsed,
            ArrayData {
                label_id: 0x2fe,
                data_size: 8,
                data: vec![NumericLiteral::Float(1.0), NumericLiteral::Float(0.0)]
            }
        );
    }
}
