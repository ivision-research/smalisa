use std::fmt;

const PARAM_BIT: usize = 1 << (std::mem::size_of::<usize>() - 1);
const NUM_BIT_MASK: usize = !PARAM_BIT;

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Register(usize);

impl Default for Register {
    #[inline(always)]
    fn default() -> Self {
        Self(0)
    }
}

impl Register {
    #[inline]
    pub fn new(is_p: bool, val: usize) -> Register {
        Register(if is_p { PARAM_BIT | val } else { val })
    }

    #[inline]
    pub fn is_param(&self) -> bool {
        self.0 & PARAM_BIT != 0
    }

    #[inline]
    pub fn num(&self) -> usize {
        self.0 & NUM_BIT_MASK
    }

    pub fn parse(s: &str) -> Option<Register> {
        let is_p = s.bytes().next()? == b'p';
        let reg_val = s.get(1..)?.parse().ok()?;
        let num = if is_p { PARAM_BIT | reg_val } else { reg_val };
        Some(Self(num))
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
    pub fn get_first_num(&self) -> usize {
        self.first.num()
    }

    #[inline]
    pub fn get_last_num(&self) -> usize {
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
pub struct RegisterArray {
    registers: [Register; MAX_VAR_REGISTERS],
    i: usize,
}

impl PartialEq for RegisterArray {
    fn eq(&self, o: &RegisterArray) -> bool {
        if self.i != o.i {
            return false;
        }
        self.registers[..self.i] == o.registers[..self.i]
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
        self.i
    }

    #[inline]
    pub fn get_registers(&self) -> &[Register] {
        &self.registers[..self.i]
    }

    #[inline]
    pub fn get_register(&self, n: usize) -> Option<Register> {
        if n >= self.i {
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
        self.registers[self.i] = reg;
        self.i += 1;
    }
}

impl fmt::Display for RegisterArray {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{")?;
        for i in 0..self.i {
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
pub enum RegisterCollection {
    /// Fixed register list that also makes the implict ones explicit
    Fixed {
        regs: [Register; MAX_FIXED_REGISTERS],
        len: usize,
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
        Self::Fixed { regs, len }
    }

    #[inline]
    pub fn len(&self) -> usize {
        match self {
            Self::Fixed { len, .. } => *len,
            Self::Var(VarRegister::Empty) => 0,
            Self::Var(VarRegister::Array(array)) => array.count(),
            Self::Var(VarRegister::Range(range)) => {
                range.get_last_num() - range.get_first_num() + 1
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
                if idx < *len {
                    Some(regs[idx])
                } else {
                    None
                }
            }
            Self::Var(VarRegister::Empty) => None,
            Self::Var(VarRegister::Array(array)) => array.get_register(idx),
            Self::Var(VarRegister::Range(range)) => {
                let num = range.get_first_num().checked_add(idx)?;
                if num <= range.get_last_num() {
                    Some(Register::new(range.is_params(), num))
                } else {
                    None
                }
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
pub enum VarRegister {
    Empty,
    Range(RegisterRange),
    Array(RegisterArray),
}

impl fmt::Display for VarRegister {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "{{}}"),
            Self::Range(ref rr) => rr.fmt(f),
            Self::Array(ref arr) => arr.fmt(f),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    macro_rules! reg {
        (p $lit:literal) => {
            crate::Register::new(true, $lit as usize)
        };
        (v $lit:literal) => {
            crate::Register::new(false, $lit as usize)
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
