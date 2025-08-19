use crate::NumericLiteral;
use std::borrow::Cow;

#[inline]
pub fn i8_to_u8(i: i8) -> u8 {
    if i >= 0 {
        i as u8
    } else {
        ((i as i32) + 256) as u8
    }
}

pub fn parse_numeric(s: &str) -> Option<NumericLiteral> {
    if s.is_empty() {
        return None;
    }
    let mut len = s.len();

    let bytes = s.as_bytes();

    // b[0] == b'I' checking for Infinity
    // b[1] == b'I' checking for -Infinity
    // b[-] == b'N' checking for NaNf
    let is_float = bytes.contains(&b'.')
        || bytes[0] == b'I'
        || (bytes.len() > 2 && bytes[1] == b'I')
        || bytes[0] == b'N';

    // Both unwraps are ok -- the only issue is that they might look at the same thing
    let last = bytes.last().unwrap();
    let is_neg = *bytes.first().unwrap() == b'-';

    let is_hex = if is_neg {
        s.starts_with("-0x")
    } else {
        s.starts_with("0x")
    };

    // If it is hex, we need to remove the 0x becuase Rust from_str_radix doesn't
    // expect that.
    let s: Cow<'_, str> = if is_hex {
        len -= 2;
        Cow::Owned(s.replace("0x", ""))
    } else {
        Cow::Borrowed(s)
    };

    macro_rules! parse_num_hex {
        ($ty:ty, $tok:ident, $s:ident) => {
            let res = <$ty>::from_str_radix(&$s, 16);
            return if let Ok(val) = res {
                Some(NumericLiteral::$tok(val))
            } else {
                None
            }
        };
    }

    macro_rules! parse_num_non_hex {
        ($ty:ty, $tok:ident, $s:ident) => {
            let res = $s.parse::<$ty>();
            return if let Ok(val) = res {
                Some(NumericLiteral::$tok(val))
            } else {
                None
            }
        };
    }

    macro_rules! parse_num_split {
        ($ty:ty, $tok:ident) => {
            let (num, _) = s.split_at(len - 1);
            if is_hex {
                parse_num_hex!($ty, $tok, num);
            } else {
                parse_num_non_hex!($ty, $tok, num);
            }
        };
    }

    match last {
        // byte lit
        b't' => {
            parse_num_split!(i8, Byte);
        }
        // long lit
        b'L' => {
            parse_num_split!(i64, Long);
        }
        // short lit
        b's' => {
            parse_num_split!(i16, Short);
        }
        _ => {
            if is_float {
                if bytes[0] == b'F' || bytes[0] == b'N' {
                    return Some(NumericLiteral::Float(f32::NAN));
                } else if bytes[0] == b'I' {
                    return Some(NumericLiteral::Float(f32::INFINITY));
                } else if is_neg && bytes.len() > 2 && bytes[1] == b'I' {
                    return Some(NumericLiteral::Float(f32::NEG_INFINITY));
                } else {
                    let (num, _) = s.split_at(len - 1);
                    parse_num_non_hex!(f32, Float, num);
                }
            } else if is_hex {
                parse_num_hex!(i32, Int, s);
            } else {
                parse_num_non_hex!(i32, Int, s);
            }
        }
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn parse_numeric_success() {
        macro_rules! test_parse {
            ($s:literal, $ty:ident, $val:literal) => {
                let parsed = parse_numeric($s);
                assert!(parsed.is_some(), concat!("failed to parse ", $s));
                assert_eq!(parsed.unwrap(), NumericLiteral::$ty($val));
            };
        }

        test_parse!("-10", Int, -10);

        test_parse!("10", Int, 10);

        test_parse!("0xf", Int, 0xf);

        test_parse!("-0xf", Int, -0xf);

        test_parse!("-10t", Byte, -10);

        test_parse!("10t", Byte, 10);

        test_parse!("-0xft", Byte, -0xf);

        test_parse!("-0x80t", Byte, -128);

        test_parse!("0x7ft", Byte, 127);

        test_parse!("0xft", Byte, 0xf);

        test_parse!("-1000000000L", Long, -1000000000);

        test_parse!("1000000000L", Long, 1000000000);

        test_parse!("2.0f", Float, 2.0);

        test_parse!("-1.0f", Float, -1.0);

        let parsed = parse_numeric("Float.NaN");
        assert!(parsed.is_some(), "failed to parse Float.NaN");
        let un = parsed.unwrap();
        if let NumericLiteral::Float(f) = un {
            assert!(f.is_nan(), "{:?} should have been nan", f);
        } else {
            panic!("expected NaN but got {:?}", un);
        }

        let parsed = parse_numeric("NaNf");
        assert!(parsed.is_some(), "failed to parse NaNf");
        let un = parsed.unwrap();
        if let NumericLiteral::Float(f) = un {
            assert!(f.is_nan(), "{:?} should have been nan", f);
        } else {
            panic!("expected NaN but got {:?}", un);
        }

        let parsed = parse_numeric("Infinity");
        assert!(parsed.is_some(), "failed to parse Infinity");
        let un = parsed.unwrap();
        if let NumericLiteral::Float(f) = un {
            assert!(f.is_infinite(), "{:?} should have been Infinity", f);
        } else {
            panic!("expected Infinity but got {:?}", un);
        }

        let parsed = parse_numeric("-Infinity");
        assert!(parsed.is_some(), "failed to parse -Infinity");
        let un = parsed.unwrap();
        if let NumericLiteral::Float(f) = un {
            assert!(
                f.is_infinite() && f.is_sign_negative(),
                "{:?} should have been -Infinity",
                f
            );
        } else {
            panic!("expected -Infinity but got {:?}", un);
        }
    }

    #[test]
    fn test_i8_to_u8() {
        let mut i8val: i8 = 1;
        assert_eq!(1, i8_to_u8(i8val));
        i8val = 2;
        assert_eq!(2, i8_to_u8(i8val));
        i8val = -1;
        assert_eq!(255, i8_to_u8(i8val));
        i8val = -2;
        assert_eq!(254, i8_to_u8(i8val));
        i8val = 0;
        assert_eq!(0, i8_to_u8(i8val));
    }
}
