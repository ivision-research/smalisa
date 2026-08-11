use std::fmt;

#[cfg(feature = "big-registers")]
pub type RegisterNumber = u32;

#[cfg(not(feature = "big-registers"))]
pub type RegisterNumber = u16;

const PARAM_BIT: RegisterNumber = 1 << (std::mem::size_of::<RegisterNumber>() * 8 - 1);
const NUM_BIT_MASK: RegisterNumber = !PARAM_BIT;

#[cfg(feature = "big-registers")]
pub type RegisterError = core::convert::Infallible;

#[cfg(not(feature = "big-registers"))]
#[derive(Debug, thiserror::Error)]
#[error("register value 0x{0:x} too large without the `big-registers` feature enabled")]
pub struct RegisterError(pub u16);

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Register(RegisterNumber);

impl Default for Register {
    #[inline(always)]
    fn default() -> Self {
        Self(0)
    }
}

impl Register {
    /// Create a new Register for the provided value
    ///
    /// With `big-registers` this call can't fail, but it might fail without it.
    pub fn new(is_p: bool, val: u16) -> Result<Self, RegisterError> {
        #[cfg(not(feature = "big-registers"))]
        if val > NUM_BIT_MASK as u16 {
            return Err(RegisterError(val));
        }

        let rval = val as RegisterNumber;
        Ok(Self::from_raw(is_p, rval))
    }

    #[inline]
    pub fn is_param(&self) -> bool {
        self.0 & PARAM_BIT != 0
    }

    #[inline]
    pub fn num(&self) -> RegisterNumber {
        self.0 & NUM_BIT_MASK
    }

    #[inline]
    pub(crate) fn from_raw(is_p: bool, val: RegisterNumber) -> Self {
        Self(if is_p { PARAM_BIT | val } else { val })
    }

    pub fn parse(s: &str) -> Option<Self> {
        let is_p = s.bytes().next()? == b'p';
        let val: u16 = s.get(1..)?.parse().ok()?;
        Self::new(is_p, val).ok()
    }
}

impl fmt::Debug for Register {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_param() {
            write!(f, "Register(p{})", self.num())
        } else {
            write!(f, "Register(v{})", self.num())
        }
    }
}

impl fmt::Display for Register {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_param() {
            write!(f, "p{}", self.num())
        } else {
            write!(f, "v{}", self.num())
        }
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct RegisterRange {
    pub(crate) first: Register,
    pub(crate) last: Register,
}

impl RegisterRange {
    #[inline]
    pub fn new(first: Register, last: Register) -> Self {
        Self { first, last }
    }
    #[inline]
    pub fn get_first(&self) -> Register {
        self.first
    }

    #[inline]
    pub fn get_last(&self) -> Register {
        self.last
    }

    #[inline]
    pub fn get_first_num(&self) -> RegisterNumber {
        self.first.num()
    }

    #[inline]
    pub fn get_last_num(&self) -> RegisterNumber {
        self.last.num()
    }

    #[inline]
    pub fn is_params(&self) -> bool {
        self.first.is_param()
    }
}

impl fmt::Display for RegisterRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{{} .. {}}}", self.first, self.last)
    }
}

/// The max number of registers that can be used before requiring a register
/// range.
pub const MAX_VAR_REGISTERS: usize = 5;

/// RegisterArray is for a variable number of registers. This differents from
/// the register range in that they don't have to be sequential.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct RegisterArray {
    registers: [Register; MAX_VAR_REGISTERS],
    i: u8,
}

impl PartialEq for RegisterArray {
    fn eq(&self, o: &RegisterArray) -> bool {
        if self.i != o.i {
            return false;
        }
        self.registers[..(self.i as usize)] == o.registers[..(self.i as usize)]
    }
}

impl RegisterArray {
    pub fn new_empty() -> Self {
        Self {
            registers: [Default::default(); MAX_VAR_REGISTERS],
            i: 0,
        }
    }

    #[inline(always)]
    pub const fn count(&self) -> usize {
        self.i as usize
    }

    #[inline]
    pub fn get_registers(&self) -> &[Register] {
        &self.registers[..(self.i as usize)]
    }

    #[inline]
    pub fn get_register(&self, n: usize) -> Option<Register> {
        if n >= (self.i as usize) {
            None
        } else {
            Some(self.registers[n])
        }
    }

    #[inline]
    pub fn get_register_unchecked(&self, n: usize) -> Register {
        self.registers[n]
    }

    pub fn push(&mut self, reg: Register) {
        // TODO Should I make a push checked?
        self.registers[self.i as usize] = reg;
        self.i += 1;
    }
}

impl fmt::Display for RegisterArray {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{")?;
        for i in 0..(self.i as usize) {
            write!(f, "{}", self.registers[i])?;
        }
        write!(f, "}}")
    }
}

/// The registers an instruction actually uses, with 64 bit pairs expanded so that the implicit high
/// half follows its low register.
///
/// Unlike [VarRegister] this includes registers that don't appear in the smali directly, such as
/// with wide instructions naming `v0` but using `v0` and `v1.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum RegisterCollection {
    /// Fixed register list that also makes the implict ones explicit
    Fixed {
        regs: [Register; MAX_FIXED_REGISTERS],
        len: u8,
    },
    /// Just wraps VarRegister
    Var(VarRegister),
}

pub const MAX_FIXED_REGISTERS: usize = 6;

impl Default for RegisterCollection {
    #[inline]
    fn default() -> Self {
        Self::Fixed {
            regs: [Register::default(); MAX_FIXED_REGISTERS],
            len: 0,
        }
    }
}

impl RegisterCollection {
    pub(crate) fn from_slots(regs: [Register; MAX_FIXED_REGISTERS], len: usize) -> Self {
        Self::Fixed {
            regs,
            len: len as u8,
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        match self {
            Self::Fixed { len, .. } => *len as usize,
            Self::Var(VarRegister::Empty) => 0,
            Self::Var(VarRegister::Array(array)) => array.count(),
            Self::Var(VarRegister::Range(range)) => {
                (range.get_last_num().saturating_sub(range.get_first_num()) + 1) as usize
            }
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(&self, idx: usize) -> Option<Register> {
        match self {
            Self::Fixed { regs, len } => {
                if idx < *len as usize {
                    Some(regs[idx])
                } else {
                    None
                }
            }
            Self::Var(VarRegister::Empty) => None,
            Self::Var(VarRegister::Array(array)) => array.get_register(idx),
            Self::Var(VarRegister::Range(range)) => {
                let idx: RegisterNumber = idx.try_into().ok()?;
                let num = range.get_first_num().checked_add(idx)?;
                (num <= range.get_last_num()).then(|| Register::from_raw(range.is_params(), num))
            }
        }
    }

    #[inline]
    pub fn iter(&self) -> RegisterCollectionIter {
        RegisterCollectionIter {
            regs: *self,
            idx: 0,
        }
    }
}

impl IntoIterator for RegisterCollection {
    type Item = Register;
    type IntoIter = RegisterCollectionIter;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        RegisterCollectionIter { regs: self, idx: 0 }
    }
}

impl IntoIterator for &RegisterCollection {
    type Item = Register;
    type IntoIter = RegisterCollectionIter;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RegisterCollectionIter {
    regs: RegisterCollection,
    idx: usize,
}

impl Iterator for RegisterCollectionIter {
    type Item = Register;

    #[inline]
    fn next(&mut self) -> Option<Register> {
        let reg = self.regs.get(self.idx)?;
        self.idx += 1;
        Some(reg)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let rem = self.regs.len().saturating_sub(self.idx);
        (rem, Some(rem))
    }
}

impl ExactSizeIterator for RegisterCollectionIter {}

#[derive(Debug, Clone, PartialEq, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum VarRegister {
    Empty,
    Range(RegisterRange),
    Array(RegisterArray),
}

impl fmt::Display for VarRegister {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "{{}}"),
            Self::Range(rr) => rr.fmt(f),
            Self::Array(arr) => arr.fmt(f),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    macro_rules! reg {
        (p $lit:literal) => {
            crate::Register::new(true, u16::try_from($lit).unwrap()).unwrap()
        };
        (v $lit:literal) => {
            crate::Register::new(false, u16::try_from($lit).unwrap()).unwrap()
        };
    }

    #[test]
    fn parse_register_success() {
        let mut reg = "p2";
        let mut res = Register::parse(reg);
        let mut parsed = res.unwrap();
        let mut exp = reg!(p 2);
        assert_eq!(parsed, exp, "expected {:?} but got {:?}", exp, parsed);

        reg = "v28";
        res = Register::parse(reg);
        parsed = res.unwrap();
        exp = reg!(v 28);
        assert_eq!(parsed, exp, "expected {:?} but got {:?}", exp, parsed);

        reg = "v0";
        res = Register::parse(reg);
        parsed = res.unwrap();
        exp = reg!(v 0);
        assert_eq!(parsed, exp, "expected {:?} but got {:?}", exp, parsed);
    }

    #[test]
    fn parse_register_fail() {
        let mut reg = "";
        let mut res = Register::parse(reg);
        assert_eq!(res.is_none(), true, "`{}` parsing should have failed", reg);

        reg = "v-28";
        res = Register::parse(reg);
        assert_eq!(res.is_none(), true, "`{}` parsing should have failed", reg);

        reg = "vok";
        res = Register::parse(reg);
        assert_eq!(res.is_none(), true, "`{}` parsing should have failed", reg);
    }

    #[test]
    fn register_eq() {
        let mut reg_a = "p2";
        let mut parsed_a = Register::parse(reg_a).unwrap();
        let mut reg_b = "p2";
        let mut parsed_b = Register::parse(reg_b).unwrap();
        assert_eq!(parsed_a == parsed_b, true, "should have been equal");

        reg_a = "v0";
        parsed_a = Register::parse(reg_a).unwrap();
        reg_b = "v0";
        parsed_b = Register::parse(reg_b).unwrap();
        assert_eq!(parsed_a == parsed_b, true, "should have been equal");

        reg_a = "v0";
        parsed_a = Register::parse(reg_a).unwrap();
        reg_b = "v1";
        parsed_b = Register::parse(reg_b).unwrap();
        assert_eq!(parsed_a != parsed_b, true, "shouldn't have been equal");
    }
}
