use std::fmt;
use std::hash::Hash;
use std::ops::Deref;

macro_rules! mask {
    ($($vals:ident)|+) => {
        $(
            Self::$vals.bits()
        )|*
    }
}

bitflags! {
    /// Instruction bitflags used to define format and actions. The CFMT_*
    /// variants are a combination of FMT_* bits.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct InsBits: u64 {
        /// The instruction uses a :label
        const FMT_LABEL = 1 << 63;
        /// The instruction takes at least 1 register
        const FMT_REG_A = 1 << 62;
        const FMT_REG_ONE = 1 << 62;
        /// The instruction takes at least 2 registers
        const FMT_REG_B = 1 << 61;
        const FMT_REG_TWO = 1 << 61;
        /// The instruction takes 3 registesrs
        const FMT_REG_C = 1 << 60;
        const FMT_REG_THREE = 1 << 60;
        /// The instruction takes a variable number of registers
        const FMT_VAR_REG = 1 << 59;
        /// The instruction uses a field
        const FMT_FIELD = 1 << 58;
        /// The instruction uses a class
        const FMT_CLASS = 1 << 57;
        /// The instruction uses an array
        const FMT_ARR = 1 << 56;
        /// The instruction uses a method
        const FMT_METHOD = 1 << 55;
        /// The instruction uses a numeric literal
        const FMT_NUM = 1 << 54;
        /// The instruction uses a string literal
        const FMT_STR = 1 << 53;
        /// Pseudo instruction for catches
        const FMT_CATCH = 1 << 52;
        /// The instruction is a switch
        const FMT_SWITCH = 1 << 51;
        /// The instruction stands alone
        const FMT_BARE = 1 << 50;
        /// The instruction is polymorphic
        const FMT_POLYMORPHIC = 1 << 49;
        /// Special FMT for invoke-custom[/range]
        const FMT_INVOKE_CUSTOM = 1 << 48;


        /// The instruction sets a register, this is always REG_A
        const ACTION_SETS_REGISTER = 1 << 47;
        /// The instruction sets a static field
        const ACTION_SETS_STATIC_FIELD = 1 << 46;
        /// The instruction sets an instance field
        const ACTION_SETS_INSTANCE_FIELD = 1 << 45;
        /// The instruction sets a single array element
        const ACTION_SETS_ARRAY_ELEMENT = 1 << 44;
        /// The instruction sets the result
        const ACTION_SETS_RESULT = 1 << 43;
        /// After this instruction execution forks, think if/else
        const ACTION_FORKING_COND = 1 << 42;
        /// This instruction jumps execution elsewhere
        const ACTION_UNCOND_JUMP = 1 << 41;
        /// The instruction is a switch statement
        const ACTION_SWITCH = 1 << 40;
        /// The instruction returns from the method
        const ACTION_RETURN = 1 << 39;
        /// The instruction can throw an exception
        const ACTION_CAN_THROW = 1 << 38;
        /// The instruction invokes another method
        const ACTION_INVOKE = 1 << 37;
        /// The instruction retrieves a value of a static field
        const ACTION_GETS_STATIC_FIELD = 1 << 36;
        /// The instruction retrieves a value of an instance field
        const ACTION_GETS_INSTANCE_FIELD = 1 << 35;
        /// The instruction retrieves a value from an array
        const ACTION_GETS_ARRAY_ELEMENT = 1 << 34;
        /// The instruction is a move instruction variant
        const ACTION_MOVE = 1 << 33;
        /// The instruction is a move result instruction variant
        const ACTION_MOVE_RESULT = 1 << 32;
        /// The first register is an inout: read before it is written. The
        /// /2addr forms. Only ever set alongside ACTION_SETS_REGISTER.
        const ACTION_INOUT = 1 << 31;

        /// The first register is an implicit pair (v0 means v0 and v1)
        const PAIR_FIRST = 1 << 30;
        /// The second register is an implicit pair (v0 means v0 and v1)
        const PAIR_SECOND = 1 << 29;
        /// The third register is an implicit pair (v0 means v0 and v1)
        const PAIR_THIRD = 1 << 28;

        /// Combination of all action related bits
        const ACTION_MASK =
            mask!(
                ACTION_SETS_REGISTER | ACTION_SETS_STATIC_FIELD | ACTION_SETS_INSTANCE_FIELD |
                ACTION_SETS_ARRAY_ELEMENT | ACTION_SETS_RESULT | ACTION_FORKING_COND |
                ACTION_UNCOND_JUMP | ACTION_SWITCH | ACTION_RETURN | ACTION_CAN_THROW |
                ACTION_INVOKE | ACTION_GETS_STATIC_FIELD | ACTION_GETS_INSTANCE_FIELD |
                ACTION_GETS_ARRAY_ELEMENT | ACTION_MOVE | ACTION_MOVE_RESULT |
                ACTION_INOUT
            );

        /// Combination of all format related bits
        const FMT_MASK  = mask!(
            FMT_LABEL | FMT_REG_A | FMT_REG_B | FMT_REG_C | FMT_VAR_REG |
            FMT_FIELD | FMT_CLASS | FMT_ARR | FMT_METHOD | FMT_NUM | FMT_STR |
            FMT_CATCH | FMT_SWITCH | FMT_BARE | FMT_POLYMORPHIC | FMT_INVOKE_CUSTOM
        );

        /// Combination of all register pair bits. Deliberately not part of
        /// FMT_MASK so that fmt() keeps comparing equal to the CFMT_* shapes.
        const PAIR_MASK = mask!(PAIR_FIRST | PAIR_SECOND | PAIR_THIRD);

        const CFMT_BARE = Self::FMT_BARE.bits();
        const CFMT_REG = Self::FMT_REG_A.bits();
        const CFMT_REG_REG = Self::FMT_REG_A.bits() | Self::FMT_REG_B.bits();
        const CFMT_REG_REG_REG = Self::FMT_REG_A.bits() | Self::FMT_REG_B.bits() | Self::FMT_REG_C.bits();
        const CFMT_LABEL = Self::FMT_LABEL.bits();
        const CFMT_REG_LABEL = Self::FMT_REG_A.bits() | Self::FMT_LABEL.bits();
        const CFMT_REG_REG_LABEL = Self::FMT_REG_A.bits() | Self::FMT_REG_B.bits() | Self::FMT_LABEL.bits();
        const CFMT_REG_STR = Self::FMT_REG_A.bits() | Self::FMT_STR.bits();
        const CFMT_REG_FIELD = Self::FMT_REG_A.bits() | Self::FMT_FIELD.bits();
        const CFMT_REG_REG_FIELD = Self::FMT_REG_A.bits() | Self::FMT_REG_B.bits() | Self::FMT_FIELD.bits();
        const CFMT_REG_CLASS = Self::FMT_REG_A.bits() | Self::FMT_CLASS.bits();
        const CFMT_REG_REG_CLASS = Self::FMT_REG_A.bits() | Self::FMT_REG_B.bits() | Self::FMT_CLASS.bits();
        const CFMT_REG_REG_ARR = Self::FMT_REG_A.bits() | Self::FMT_REG_B.bits() | Self::FMT_ARR.bits();
        const CFMT_ARGS_METHOD = Self::FMT_VAR_REG.bits() | Self::FMT_METHOD.bits();
        const CFMT_ARGS_METHOD_POLYMORPHIC = Self::FMT_VAR_REG.bits() | Self::FMT_METHOD.bits() | Self::FMT_POLYMORPHIC.bits();
        const CFMT_ARGS_METHOD_CUSTOM = Self::FMT_VAR_REG.bits() | Self::FMT_INVOKE_CUSTOM.bits();
        const CFMT_ARGS_ARR = Self::FMT_VAR_REG.bits() | Self::FMT_ARR.bits();
        const CFMT_REG_NUM = Self::FMT_REG_A.bits() | Self::FMT_NUM.bits();
        const CFMT_REG_REG_NUM = Self::FMT_REG_A.bits() | Self::FMT_REG_B.bits() | Self::FMT_NUM.bits();

    }
}

#[derive(PartialEq, Eq, Copy, Clone)]
pub struct Instruction(pub(crate) InsBits);

// Note that these have to be manual because we stuff extra data into the low bits
#[cfg(any(feature = "serde"))]
impl<'de> serde::Deserialize<'de> for Instruction {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Self::from_raw(u64::deserialize(deserializer)?))
    }
}

#[cfg(any(feature = "serde"))]
impl serde::Serialize for Instruction {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.raw().serialize(serializer)
    }
}

macro_rules! ins {
    ($name:ident, $id:literal) => {
        pub const $name: Instruction = Instruction(
           InsBits::from_bits_retain($id)
        );
    };

    ($name:ident, $id:literal, $($bits:ident)|+) => {
        pub const $name: Instruction = Instruction (
            InsBits::from_bits_retain(
                 $id | $(
                    InsBits::$bits.bits()
                )|*
            )
        );
    }
}

impl Default for Instruction {
    fn default() -> Self {
        INS_UNKNOWN_INSTRUCTION
    }
}

ins!(INS_UNKNOWN_INSTRUCTION, 0);
ins!(INS_GOTO, 8, ACTION_UNCOND_JUMP | CFMT_LABEL);
ins!(INS_GOTO_16, 9, ACTION_UNCOND_JUMP | CFMT_LABEL);
ins!(INS_GOTO_32, 10, ACTION_UNCOND_JUMP | CFMT_LABEL);
ins!(INS_RETURN, 11, ACTION_RETURN | CFMT_REG);
ins!(INS_RETURN_VOID, 12, ACTION_RETURN | CFMT_BARE);
ins!(INS_RETURN_WIDE, 13, ACTION_RETURN | CFMT_REG | PAIR_FIRST);
ins!(INS_RETURN_OBJECT, 14, ACTION_RETURN | CFMT_REG);
ins!(INS_RETURN_VOID_BARRIER, 15, ACTION_RETURN | CFMT_BARE);
ins!(INS_RETURN_VOID_NO_BARRIER, 16, ACTION_RETURN | CFMT_BARE);
ins!(INS_NOP, 17, CFMT_BARE);
ins!(INS_CONST, 18, ACTION_SETS_REGISTER | CFMT_REG_NUM);
ins!(INS_CONST_4, 19, ACTION_SETS_REGISTER | CFMT_REG_NUM);
ins!(INS_CONST_16, 20, ACTION_SETS_REGISTER | CFMT_REG_NUM);
ins!(
    INS_CONST_WIDE,
    21,
    ACTION_SETS_REGISTER | CFMT_REG_NUM | PAIR_FIRST
);
ins!(
    INS_CONST_WIDE_16,
    22,
    ACTION_SETS_REGISTER | CFMT_REG_NUM | PAIR_FIRST
);
ins!(
    INS_CONST_WIDE_32,
    23,
    ACTION_SETS_REGISTER | CFMT_REG_NUM | PAIR_FIRST
);
ins!(INS_CONST_HIGH16, 24, ACTION_SETS_REGISTER | CFMT_REG_NUM);
ins!(
    INS_CONST_WIDE_HIGH16,
    25,
    ACTION_SETS_REGISTER | CFMT_REG_NUM | PAIR_FIRST
);
ins!(
    INS_CONST_STRING,
    26,
    ACTION_SETS_REGISTER | ACTION_CAN_THROW | CFMT_REG_STR
);
ins!(
    INS_CONST_STRING_JUMBO,
    27,
    ACTION_SETS_REGISTER | ACTION_CAN_THROW | CFMT_REG_STR
);
ins!(
    INS_CONST_METHOD_TYPE,
    28,
    ACTION_SETS_REGISTER | ACTION_CAN_THROW
);
ins!(
    INS_CONST_METHOD_HANDLE,
    29,
    ACTION_SETS_REGISTER | ACTION_CAN_THROW
);
ins!(INS_IF_NEZ, 30, ACTION_FORKING_COND | CFMT_REG_LABEL);
ins!(INS_IF_EQZ, 31, ACTION_FORKING_COND | CFMT_REG_LABEL);
ins!(INS_IF_LTZ, 32, ACTION_FORKING_COND | CFMT_REG_LABEL);
ins!(INS_IF_GEZ, 33, ACTION_FORKING_COND | CFMT_REG_LABEL);
ins!(INS_IF_GTZ, 34, ACTION_FORKING_COND | CFMT_REG_LABEL);
ins!(INS_IF_LEZ, 35, ACTION_FORKING_COND | CFMT_REG_LABEL);
ins!(INS_IF_EQ, 36, ACTION_FORKING_COND | CFMT_REG_REG_LABEL);
ins!(INS_IF_NE, 37, ACTION_FORKING_COND | CFMT_REG_REG_LABEL);
ins!(INS_IF_LT, 38, ACTION_FORKING_COND | CFMT_REG_REG_LABEL);
ins!(INS_IF_GE, 39, ACTION_FORKING_COND | CFMT_REG_REG_LABEL);
ins!(INS_IF_GT, 40, ACTION_FORKING_COND | CFMT_REG_REG_LABEL);
ins!(INS_IF_LE, 41, ACTION_FORKING_COND | CFMT_REG_REG_LABEL);
ins!(
    INS_MOVE_RESULT,
    42,
    ACTION_SETS_REGISTER | ACTION_MOVE_RESULT | CFMT_REG
);
ins!(
    INS_MOVE_RESULT_WIDE,
    43,
    ACTION_SETS_REGISTER | ACTION_MOVE_RESULT | CFMT_REG | PAIR_FIRST
);
ins!(
    INS_MOVE_RESULT_OBJECT,
    44,
    ACTION_SETS_REGISTER | ACTION_MOVE_RESULT | CFMT_REG
);
ins!(INS_MOVE_EXCEPTION, 45, ACTION_SETS_REGISTER | CFMT_REG);
ins!(INS_MONITOR_ENTER, 46, ACTION_CAN_THROW | CFMT_REG);
ins!(INS_MONITOR_EXIT, 47, ACTION_CAN_THROW | CFMT_REG);
ins!(INS_THROW, 48, ACTION_CAN_THROW | CFMT_REG);
ins!(
    INS_MOVE,
    49,
    ACTION_SETS_REGISTER | ACTION_MOVE | CFMT_REG_REG
);
ins!(
    INS_MOVE_WIDE,
    50,
    ACTION_SETS_REGISTER | ACTION_MOVE | CFMT_REG_REG | PAIR_FIRST | PAIR_SECOND
);
ins!(
    INS_MOVE_OBJECT,
    51,
    ACTION_SETS_REGISTER | ACTION_MOVE | CFMT_REG_REG
);
ins!(
    INS_ARRAY_LENGTH,
    52,
    ACTION_SETS_REGISTER | ACTION_CAN_THROW | CFMT_REG_REG
);
ins!(INS_NEG_INT, 53, ACTION_SETS_REGISTER | CFMT_REG_REG);
ins!(INS_NOT_INT, 54, ACTION_SETS_REGISTER | CFMT_REG_REG);
ins!(
    INS_NEG_LONG,
    55,
    ACTION_SETS_REGISTER | CFMT_REG_REG | PAIR_FIRST | PAIR_SECOND
);
ins!(
    INS_NOT_LONG,
    56,
    ACTION_SETS_REGISTER | CFMT_REG_REG | PAIR_FIRST | PAIR_SECOND
);
ins!(INS_NEG_FLOAT, 57, ACTION_SETS_REGISTER | CFMT_REG_REG);
ins!(
    INS_NEG_DOUBLE,
    58,
    ACTION_SETS_REGISTER | CFMT_REG_REG | PAIR_FIRST | PAIR_SECOND
);
ins!(
    INS_INT_TO_LONG,
    59,
    ACTION_SETS_REGISTER | CFMT_REG_REG | PAIR_FIRST
);
ins!(INS_INT_TO_FLOAT, 60, ACTION_SETS_REGISTER | CFMT_REG_REG);
ins!(
    INS_INT_TO_DOUBLE,
    61,
    ACTION_SETS_REGISTER | CFMT_REG_REG | PAIR_FIRST
);
ins!(
    INS_LONG_TO_INT,
    62,
    ACTION_SETS_REGISTER | CFMT_REG_REG | PAIR_SECOND
);
ins!(
    INS_LONG_TO_FLOAT,
    63,
    ACTION_SETS_REGISTER | CFMT_REG_REG | PAIR_SECOND
);
ins!(
    INS_LONG_TO_DOUBLE,
    64,
    ACTION_SETS_REGISTER | CFMT_REG_REG | PAIR_FIRST | PAIR_SECOND
);
ins!(INS_FLOAT_TO_INT, 65, ACTION_SETS_REGISTER | CFMT_REG_REG);
ins!(
    INS_FLOAT_TO_LONG,
    66,
    ACTION_SETS_REGISTER | CFMT_REG_REG | PAIR_FIRST
);
ins!(
    INS_FLOAT_TO_DOUBLE,
    67,
    ACTION_SETS_REGISTER | CFMT_REG_REG | PAIR_FIRST
);
ins!(
    INS_DOUBLE_TO_INT,
    68,
    ACTION_SETS_REGISTER | CFMT_REG_REG | PAIR_SECOND
);
ins!(
    INS_DOUBLE_TO_LONG,
    69,
    ACTION_SETS_REGISTER | CFMT_REG_REG | PAIR_FIRST | PAIR_SECOND
);
ins!(
    INS_DOUBLE_TO_FLOAT,
    70,
    ACTION_SETS_REGISTER | CFMT_REG_REG | PAIR_SECOND
);
ins!(INS_INT_TO_BYTE, 71, ACTION_SETS_REGISTER | CFMT_REG_REG);
ins!(INS_INT_TO_CHAR, 72, ACTION_SETS_REGISTER | CFMT_REG_REG);
ins!(INS_INT_TO_SHORT, 73, ACTION_SETS_REGISTER | CFMT_REG_REG);
ins!(
    INS_ADD_INT_2ADDR,
    74,
    ACTION_SETS_REGISTER | CFMT_REG_REG | ACTION_INOUT
);
ins!(
    INS_SUB_INT_2ADDR,
    75,
    ACTION_SETS_REGISTER | CFMT_REG_REG | ACTION_INOUT
);
ins!(
    INS_MUL_INT_2ADDR,
    76,
    ACTION_SETS_REGISTER | CFMT_REG_REG | ACTION_INOUT
);
ins!(
    INS_DIV_INT_2ADDR,
    77,
    ACTION_SETS_REGISTER | ACTION_CAN_THROW | CFMT_REG_REG | ACTION_INOUT
);
ins!(
    INS_REM_INT_2ADDR,
    78,
    ACTION_SETS_REGISTER | ACTION_CAN_THROW | CFMT_REG_REG | ACTION_INOUT
);
ins!(
    INS_AND_INT_2ADDR,
    79,
    ACTION_SETS_REGISTER | CFMT_REG_REG | ACTION_INOUT
);
ins!(
    INS_OR_INT_2ADDR,
    80,
    ACTION_SETS_REGISTER | CFMT_REG_REG | ACTION_INOUT
);
ins!(
    INS_XOR_INT_2ADDR,
    81,
    ACTION_SETS_REGISTER | CFMT_REG_REG | ACTION_INOUT
);
ins!(
    INS_SHL_INT_2ADDR,
    82,
    ACTION_SETS_REGISTER | CFMT_REG_REG | ACTION_INOUT
);
ins!(
    INS_SHR_INT_2ADDR,
    83,
    ACTION_SETS_REGISTER | CFMT_REG_REG | ACTION_INOUT
);
ins!(
    INS_USHR_INT_2ADDR,
    84,
    ACTION_SETS_REGISTER | CFMT_REG_REG | ACTION_INOUT
);
ins!(
    INS_ADD_LONG_2ADDR,
    85,
    ACTION_SETS_REGISTER | CFMT_REG_REG | PAIR_FIRST | PAIR_SECOND | ACTION_INOUT
);
ins!(
    INS_SUB_LONG_2ADDR,
    86,
    ACTION_SETS_REGISTER | CFMT_REG_REG | PAIR_FIRST | PAIR_SECOND | ACTION_INOUT
);
ins!(
    INS_MUL_LONG_2ADDR,
    87,
    ACTION_SETS_REGISTER | CFMT_REG_REG | PAIR_FIRST | PAIR_SECOND | ACTION_INOUT
);
ins!(
    INS_DIV_LONG_2ADDR,
    88,
    ACTION_SETS_REGISTER
        | ACTION_CAN_THROW
        | CFMT_REG_REG
        | PAIR_FIRST
        | PAIR_SECOND
        | ACTION_INOUT
);
ins!(
    INS_REM_LONG_2ADDR,
    89,
    ACTION_SETS_REGISTER
        | ACTION_CAN_THROW
        | CFMT_REG_REG
        | PAIR_FIRST
        | PAIR_SECOND
        | ACTION_INOUT
);
ins!(
    INS_AND_LONG_2ADDR,
    90,
    ACTION_SETS_REGISTER | CFMT_REG_REG | PAIR_FIRST | PAIR_SECOND | ACTION_INOUT
);
ins!(
    INS_OR_LONG_2ADDR,
    91,
    ACTION_SETS_REGISTER | CFMT_REG_REG | PAIR_FIRST | PAIR_SECOND | ACTION_INOUT
);
ins!(
    INS_XOR_LONG_2ADDR,
    92,
    ACTION_SETS_REGISTER | CFMT_REG_REG | PAIR_FIRST | PAIR_SECOND | ACTION_INOUT
);
ins!(
    INS_SHL_LONG_2ADDR,
    93,
    ACTION_SETS_REGISTER | CFMT_REG_REG | PAIR_FIRST | ACTION_INOUT
);
ins!(
    INS_SHR_LONG_2ADDR,
    94,
    ACTION_SETS_REGISTER | CFMT_REG_REG | PAIR_FIRST | ACTION_INOUT
);
ins!(
    INS_USHR_LONG_2ADDR,
    95,
    ACTION_SETS_REGISTER | CFMT_REG_REG | PAIR_FIRST | ACTION_INOUT
);
ins!(
    INS_ADD_FLOAT_2ADDR,
    96,
    ACTION_SETS_REGISTER | CFMT_REG_REG | ACTION_INOUT
);
ins!(
    INS_SUB_FLOAT_2ADDR,
    97,
    ACTION_SETS_REGISTER | CFMT_REG_REG | ACTION_INOUT
);
ins!(
    INS_MUL_FLOAT_2ADDR,
    98,
    ACTION_SETS_REGISTER | CFMT_REG_REG | ACTION_INOUT
);
ins!(
    INS_DIV_FLOAT_2ADDR,
    99,
    ACTION_SETS_REGISTER | CFMT_REG_REG | ACTION_INOUT
);
ins!(
    INS_REM_FLOAT_2ADDR,
    100,
    ACTION_SETS_REGISTER | CFMT_REG_REG | ACTION_INOUT
);
ins!(
    INS_ADD_DOUBLE_2ADDR,
    101,
    ACTION_SETS_REGISTER | CFMT_REG_REG | PAIR_FIRST | PAIR_SECOND | ACTION_INOUT
);
ins!(
    INS_SUB_DOUBLE_2ADDR,
    102,
    ACTION_SETS_REGISTER | CFMT_REG_REG | PAIR_FIRST | PAIR_SECOND | ACTION_INOUT
);
ins!(
    INS_MUL_DOUBLE_2ADDR,
    103,
    ACTION_SETS_REGISTER | CFMT_REG_REG | PAIR_FIRST | PAIR_SECOND | ACTION_INOUT
);
ins!(
    INS_DIV_DOUBLE_2ADDR,
    104,
    ACTION_SETS_REGISTER | CFMT_REG_REG | PAIR_FIRST | PAIR_SECOND | ACTION_INOUT
);
ins!(
    INS_REM_DOUBLE_2ADDR,
    105,
    ACTION_SETS_REGISTER | CFMT_REG_REG | PAIR_FIRST | PAIR_SECOND | ACTION_INOUT
);
ins!(
    INS_SGET,
    106,
    ACTION_GETS_STATIC_FIELD | ACTION_SETS_REGISTER | ACTION_CAN_THROW | CFMT_REG_FIELD
);
ins!(
    INS_SGET_WIDE,
    107,
    ACTION_GETS_STATIC_FIELD
        | ACTION_SETS_REGISTER
        | ACTION_CAN_THROW
        | CFMT_REG_FIELD
        | PAIR_FIRST
);
ins!(
    INS_SGET_OBJECT,
    108,
    ACTION_GETS_STATIC_FIELD | ACTION_SETS_REGISTER | ACTION_CAN_THROW | CFMT_REG_FIELD
);
ins!(
    INS_SGET_BOOLEAN,
    109,
    ACTION_GETS_STATIC_FIELD | ACTION_SETS_REGISTER | ACTION_CAN_THROW | CFMT_REG_FIELD
);
ins!(
    INS_SGET_BYTE,
    110,
    ACTION_GETS_STATIC_FIELD | ACTION_SETS_REGISTER | ACTION_CAN_THROW | CFMT_REG_FIELD
);
ins!(
    INS_SGET_CHAR,
    111,
    ACTION_GETS_STATIC_FIELD | ACTION_SETS_REGISTER | ACTION_CAN_THROW | CFMT_REG_FIELD
);
ins!(
    INS_SGET_SHORT,
    112,
    ACTION_GETS_STATIC_FIELD | ACTION_SETS_REGISTER | ACTION_CAN_THROW | CFMT_REG_FIELD
);
ins!(
    INS_SPUT,
    113,
    ACTION_SETS_STATIC_FIELD | ACTION_CAN_THROW | CFMT_REG_FIELD
);
ins!(
    INS_SPUT_WIDE,
    114,
    ACTION_SETS_STATIC_FIELD | ACTION_CAN_THROW | CFMT_REG_FIELD | PAIR_FIRST
);
ins!(
    INS_SPUT_OBJECT,
    115,
    ACTION_SETS_STATIC_FIELD | ACTION_CAN_THROW | CFMT_REG_FIELD
);
ins!(
    INS_SPUT_BOOLEAN,
    116,
    ACTION_SETS_STATIC_FIELD | ACTION_CAN_THROW | CFMT_REG_FIELD
);
ins!(
    INS_SPUT_BYTE,
    117,
    ACTION_SETS_STATIC_FIELD | ACTION_CAN_THROW | CFMT_REG_FIELD
);
ins!(
    INS_SPUT_CHAR,
    118,
    ACTION_SETS_STATIC_FIELD | ACTION_CAN_THROW | CFMT_REG_FIELD
);
ins!(
    INS_SPUT_SHORT,
    119,
    ACTION_SETS_STATIC_FIELD | ACTION_CAN_THROW | CFMT_REG_FIELD
);
ins!(
    INS_SGET_VOLATILE,
    120,
    ACTION_GETS_STATIC_FIELD | ACTION_SETS_REGISTER | ACTION_CAN_THROW | CFMT_REG_FIELD
);
ins!(
    INS_SGET_WIDE_VOLATILE,
    121,
    ACTION_GETS_STATIC_FIELD
        | ACTION_SETS_REGISTER
        | ACTION_CAN_THROW
        | CFMT_REG_FIELD
        | PAIR_FIRST
);
ins!(
    INS_SGET_OBJECT_VOLATILE,
    122,
    ACTION_GETS_STATIC_FIELD | ACTION_SETS_REGISTER | ACTION_CAN_THROW | CFMT_REG_FIELD
);
ins!(
    INS_SPUT_VOLATILE,
    123,
    ACTION_SETS_STATIC_FIELD | ACTION_CAN_THROW | CFMT_REG_FIELD
);
ins!(
    INS_SPUT_WIDE_VOLATILE,
    124,
    ACTION_SETS_STATIC_FIELD | ACTION_CAN_THROW | CFMT_REG_FIELD | PAIR_FIRST
);
ins!(
    INS_SPUT_OBJECT_VOLATILE,
    125,
    ACTION_SETS_STATIC_FIELD | ACTION_CAN_THROW | CFMT_REG_FIELD
);
ins!(INS_CHECK_CAST, 126, ACTION_CAN_THROW | CFMT_REG_CLASS);
ins!(
    INS_NEW_INSTANCE,
    127,
    ACTION_SETS_REGISTER | ACTION_CAN_THROW | CFMT_REG_CLASS
);
ins!(
    INS_CONST_CLASS,
    128,
    ACTION_SETS_REGISTER | ACTION_CAN_THROW | CFMT_REG_CLASS
);
ins!(
    INS_ADD_INT_LIT8,
    129,
    ACTION_SETS_REGISTER | CFMT_REG_REG_NUM
);
ins!(
    INS_RSUB_INT_LIT8,
    130,
    ACTION_SETS_REGISTER | CFMT_REG_REG_NUM
);
ins!(
    INS_MUL_INT_LIT8,
    131,
    ACTION_SETS_REGISTER | CFMT_REG_REG_NUM
);
ins!(
    INS_DIV_INT_LIT8,
    132,
    ACTION_SETS_REGISTER | ACTION_CAN_THROW | CFMT_REG_REG_NUM
);
ins!(
    INS_REM_INT_LIT8,
    133,
    ACTION_SETS_REGISTER | ACTION_CAN_THROW | CFMT_REG_REG_NUM
);
ins!(
    INS_AND_INT_LIT8,
    134,
    ACTION_SETS_REGISTER | CFMT_REG_REG_NUM
);
ins!(
    INS_OR_INT_LIT8,
    135,
    ACTION_SETS_REGISTER | CFMT_REG_REG_NUM
);
ins!(
    INS_XOR_INT_LIT8,
    136,
    ACTION_SETS_REGISTER | CFMT_REG_REG_NUM
);
ins!(
    INS_SHL_INT_LIT8,
    137,
    ACTION_SETS_REGISTER | CFMT_REG_REG_NUM
);
ins!(
    INS_SHR_INT_LIT8,
    138,
    ACTION_SETS_REGISTER | CFMT_REG_REG_NUM
);
ins!(
    INS_USHR_INT_LIT8,
    139,
    ACTION_SETS_REGISTER | CFMT_REG_REG_NUM
);
ins!(
    INS_IGET,
    140,
    ACTION_GETS_INSTANCE_FIELD | ACTION_SETS_REGISTER | ACTION_CAN_THROW | CFMT_REG_REG_FIELD
);
ins!(
    INS_IGET_WIDE,
    141,
    ACTION_GETS_INSTANCE_FIELD
        | ACTION_SETS_REGISTER
        | ACTION_CAN_THROW
        | CFMT_REG_REG_FIELD
        | PAIR_FIRST
);
ins!(
    INS_IGET_OBJECT,
    142,
    ACTION_GETS_INSTANCE_FIELD | ACTION_SETS_REGISTER | ACTION_CAN_THROW | CFMT_REG_REG_FIELD
);
ins!(
    INS_IGET_BOOLEAN,
    143,
    ACTION_GETS_INSTANCE_FIELD | ACTION_SETS_REGISTER | ACTION_CAN_THROW | CFMT_REG_REG_FIELD
);
ins!(
    INS_IGET_BYTE,
    144,
    ACTION_GETS_INSTANCE_FIELD | ACTION_SETS_REGISTER | ACTION_CAN_THROW | CFMT_REG_REG_FIELD
);
ins!(
    INS_IGET_CHAR,
    145,
    ACTION_GETS_INSTANCE_FIELD | ACTION_SETS_REGISTER | ACTION_CAN_THROW | CFMT_REG_REG_FIELD
);
ins!(
    INS_IGET_SHORT,
    146,
    ACTION_GETS_INSTANCE_FIELD | ACTION_SETS_REGISTER | ACTION_CAN_THROW | CFMT_REG_REG_FIELD
);
ins!(
    INS_IPUT,
    147,
    ACTION_SETS_INSTANCE_FIELD | ACTION_CAN_THROW | CFMT_REG_REG_FIELD
);
ins!(
    INS_IPUT_WIDE,
    148,
    ACTION_SETS_INSTANCE_FIELD | ACTION_CAN_THROW | CFMT_REG_REG_FIELD | PAIR_FIRST
);
ins!(
    INS_IPUT_OBJECT,
    149,
    ACTION_SETS_INSTANCE_FIELD | ACTION_CAN_THROW | CFMT_REG_REG_FIELD
);
ins!(
    INS_IPUT_BOOLEAN,
    150,
    ACTION_SETS_INSTANCE_FIELD | ACTION_CAN_THROW | CFMT_REG_REG_FIELD
);
ins!(
    INS_IPUT_BYTE,
    151,
    ACTION_SETS_INSTANCE_FIELD | ACTION_CAN_THROW | CFMT_REG_REG_FIELD
);
ins!(
    INS_IPUT_CHAR,
    152,
    ACTION_SETS_INSTANCE_FIELD | ACTION_CAN_THROW | CFMT_REG_REG_FIELD
);
ins!(
    INS_IPUT_SHORT,
    153,
    ACTION_SETS_INSTANCE_FIELD | ACTION_CAN_THROW | CFMT_REG_REG_FIELD
);
ins!(
    INS_IGET_VOLATILE,
    154,
    ACTION_GETS_INSTANCE_FIELD | ACTION_SETS_REGISTER | ACTION_CAN_THROW | CFMT_REG_REG_FIELD
);
ins!(
    INS_IGET_WIDE_VOLATILE,
    155,
    ACTION_GETS_INSTANCE_FIELD
        | ACTION_SETS_REGISTER
        | ACTION_CAN_THROW
        | CFMT_REG_REG_FIELD
        | PAIR_FIRST
);
ins!(
    INS_IGET_OBJECT_VOLATILE,
    156,
    ACTION_GETS_INSTANCE_FIELD | ACTION_SETS_REGISTER | ACTION_CAN_THROW | CFMT_REG_REG_FIELD
);
ins!(
    INS_IPUT_VOLATILE,
    157,
    ACTION_SETS_INSTANCE_FIELD | ACTION_CAN_THROW | CFMT_REG_REG_FIELD
);
ins!(
    INS_IPUT_WIDE_VOLATILE,
    158,
    ACTION_SETS_INSTANCE_FIELD | ACTION_CAN_THROW | CFMT_REG_REG_FIELD | PAIR_FIRST
);
ins!(
    INS_IPUT_OBJECT_VOLATILE,
    159,
    ACTION_SETS_INSTANCE_FIELD | ACTION_CAN_THROW | CFMT_REG_REG_FIELD
);
ins!(
    INS_INSTANCE_OF,
    160,
    ACTION_SETS_REGISTER | ACTION_CAN_THROW | CFMT_REG_REG_CLASS
);
ins!(
    INS_NEW_ARRAY,
    161,
    ACTION_SETS_REGISTER | ACTION_CAN_THROW | CFMT_REG_REG_ARR
);
ins!(
    INS_IGET_QUICK,
    162,
    ACTION_SETS_REGISTER | ACTION_GETS_INSTANCE_FIELD | ACTION_CAN_THROW
);
ins!(
    INS_IGET_WIDE_QUICK,
    163,
    ACTION_SETS_REGISTER | ACTION_GETS_INSTANCE_FIELD | PAIR_FIRST | ACTION_CAN_THROW
);
ins!(
    INS_IGET_OBJECT_QUICK,
    164,
    ACTION_SETS_REGISTER | ACTION_GETS_INSTANCE_FIELD | ACTION_CAN_THROW
);
ins!(
    INS_IPUT_QUICK,
    165,
    ACTION_SETS_INSTANCE_FIELD | ACTION_CAN_THROW
);
ins!(
    INS_IPUT_WIDE_QUICK,
    166,
    ACTION_SETS_INSTANCE_FIELD | PAIR_FIRST | ACTION_CAN_THROW
);
ins!(
    INS_IPUT_OBJECT_QUICK,
    167,
    ACTION_SETS_INSTANCE_FIELD | ACTION_CAN_THROW
);
ins!(
    INS_IPUT_BOOLEAN_QUICK,
    168,
    ACTION_SETS_INSTANCE_FIELD | ACTION_CAN_THROW
);
ins!(
    INS_IPUT_BYTE_QUICK,
    169,
    ACTION_SETS_INSTANCE_FIELD | ACTION_CAN_THROW
);
ins!(
    INS_IPUT_CHAR_QUICK,
    170,
    ACTION_SETS_INSTANCE_FIELD | ACTION_CAN_THROW
);
ins!(
    INS_IPUT_SHORT_QUICK,
    171,
    ACTION_SETS_INSTANCE_FIELD | ACTION_CAN_THROW
);
ins!(INS_RSUB_INT, 172, ACTION_SETS_REGISTER | CFMT_REG_REG_NUM);
ins!(
    INS_ADD_INT_LIT16,
    173,
    ACTION_SETS_REGISTER | CFMT_REG_REG_NUM
);
ins!(
    INS_MUL_INT_LIT16,
    174,
    ACTION_SETS_REGISTER | CFMT_REG_REG_NUM
);
ins!(
    INS_DIV_INT_LIT16,
    175,
    ACTION_SETS_REGISTER | ACTION_CAN_THROW | CFMT_REG_REG_NUM
);
ins!(
    INS_REM_INT_LIT16,
    176,
    ACTION_SETS_REGISTER | ACTION_CAN_THROW | CFMT_REG_REG_NUM
);
ins!(
    INS_AND_INT_LIT16,
    177,
    ACTION_SETS_REGISTER | CFMT_REG_REG_NUM
);
ins!(
    INS_OR_INT_LIT16,
    178,
    ACTION_SETS_REGISTER | CFMT_REG_REG_NUM
);
ins!(
    INS_XOR_INT_LIT16,
    179,
    ACTION_SETS_REGISTER | CFMT_REG_REG_NUM
);
ins!(
    INS_MOVE_FROM16,
    180,
    ACTION_SETS_REGISTER | ACTION_MOVE | CFMT_REG_REG
);
ins!(
    INS_MOVE_WIDE_FROM16,
    181,
    ACTION_SETS_REGISTER | ACTION_MOVE | CFMT_REG_REG | PAIR_FIRST | PAIR_SECOND
);
ins!(
    INS_MOVE_OBJECT_FROM16,
    182,
    ACTION_SETS_REGISTER | ACTION_MOVE | CFMT_REG_REG
);
ins!(INS_CMPL_FLOAT, 183, ACTION_SETS_REGISTER | CFMT_REG_REG_REG);
ins!(INS_CMPG_FLOAT, 184, ACTION_SETS_REGISTER | CFMT_REG_REG_REG);
ins!(
    INS_CMPL_DOUBLE,
    185,
    ACTION_SETS_REGISTER | CFMT_REG_REG_REG | PAIR_SECOND | PAIR_THIRD
);
ins!(
    INS_CMPG_DOUBLE,
    186,
    ACTION_SETS_REGISTER | CFMT_REG_REG_REG | PAIR_SECOND | PAIR_THIRD
);
ins!(
    INS_CMP_LONG,
    187,
    ACTION_SETS_REGISTER | CFMT_REG_REG_REG | PAIR_SECOND | PAIR_THIRD
);
ins!(
    INS_AGET,
    188,
    ACTION_SETS_REGISTER | ACTION_GETS_ARRAY_ELEMENT | ACTION_CAN_THROW | CFMT_REG_REG_REG
);
ins!(
    INS_AGET_WIDE,
    189,
    ACTION_SETS_REGISTER
        | ACTION_GETS_ARRAY_ELEMENT
        | ACTION_CAN_THROW
        | CFMT_REG_REG_REG
        | PAIR_FIRST
);
ins!(
    INS_AGET_OBJECT,
    190,
    ACTION_SETS_REGISTER | ACTION_GETS_ARRAY_ELEMENT | ACTION_CAN_THROW | CFMT_REG_REG_REG
);
ins!(
    INS_AGET_BOOLEAN,
    191,
    ACTION_SETS_REGISTER | ACTION_CAN_THROW | CFMT_REG_REG_REG
);
ins!(
    INS_AGET_BYTE,
    192,
    ACTION_SETS_REGISTER | ACTION_GETS_ARRAY_ELEMENT | ACTION_CAN_THROW | CFMT_REG_REG_REG
);
ins!(
    INS_AGET_CHAR,
    193,
    ACTION_SETS_REGISTER | ACTION_GETS_ARRAY_ELEMENT | ACTION_CAN_THROW | CFMT_REG_REG_REG
);
ins!(
    INS_AGET_SHORT,
    194,
    ACTION_SETS_REGISTER | ACTION_GETS_ARRAY_ELEMENT | ACTION_CAN_THROW | CFMT_REG_REG_REG
);
ins!(
    INS_APUT,
    195,
    ACTION_SETS_ARRAY_ELEMENT | ACTION_CAN_THROW | CFMT_REG_REG_REG
);
ins!(
    INS_APUT_WIDE,
    196,
    ACTION_SETS_ARRAY_ELEMENT | ACTION_CAN_THROW | CFMT_REG_REG_REG | PAIR_FIRST
);
ins!(
    INS_APUT_OBJECT,
    197,
    ACTION_SETS_ARRAY_ELEMENT | ACTION_CAN_THROW | CFMT_REG_REG_REG
);
ins!(
    INS_APUT_BOOLEAN,
    198,
    ACTION_SETS_ARRAY_ELEMENT | ACTION_CAN_THROW | CFMT_REG_REG_REG
);
ins!(
    INS_APUT_BYTE,
    199,
    ACTION_SETS_ARRAY_ELEMENT | ACTION_CAN_THROW | CFMT_REG_REG_REG
);
ins!(
    INS_APUT_CHAR,
    200,
    ACTION_SETS_ARRAY_ELEMENT | ACTION_CAN_THROW | CFMT_REG_REG_REG
);
ins!(
    INS_APUT_SHORT,
    201,
    ACTION_SETS_ARRAY_ELEMENT | ACTION_CAN_THROW | CFMT_REG_REG_REG
);
ins!(INS_ADD_INT, 202, ACTION_SETS_REGISTER | CFMT_REG_REG_REG);
ins!(INS_SUB_INT, 203, ACTION_SETS_REGISTER | CFMT_REG_REG_REG);
ins!(INS_MUL_INT, 204, ACTION_SETS_REGISTER | CFMT_REG_REG_REG);
ins!(
    INS_DIV_INT,
    205,
    ACTION_SETS_REGISTER | ACTION_CAN_THROW | CFMT_REG_REG_REG
);
ins!(
    INS_REM_INT,
    206,
    ACTION_SETS_REGISTER | ACTION_CAN_THROW | CFMT_REG_REG_REG
);
ins!(INS_AND_INT, 207, ACTION_SETS_REGISTER | CFMT_REG_REG_REG);
ins!(INS_OR_INT, 208, ACTION_SETS_REGISTER | CFMT_REG_REG_REG);
ins!(INS_XOR_INT, 209, ACTION_SETS_REGISTER | CFMT_REG_REG_REG);
ins!(INS_SHL_INT, 210, ACTION_SETS_REGISTER | CFMT_REG_REG_REG);
ins!(INS_SHR_INT, 211, ACTION_SETS_REGISTER | CFMT_REG_REG_REG);
ins!(INS_USHR_INT, 212, ACTION_SETS_REGISTER | CFMT_REG_REG_REG);
ins!(
    INS_ADD_LONG,
    213,
    ACTION_SETS_REGISTER | CFMT_REG_REG_REG | PAIR_FIRST | PAIR_SECOND | PAIR_THIRD
);
ins!(
    INS_SUB_LONG,
    214,
    ACTION_SETS_REGISTER | CFMT_REG_REG_REG | PAIR_FIRST | PAIR_SECOND | PAIR_THIRD
);
ins!(
    INS_MUL_LONG,
    215,
    ACTION_SETS_REGISTER | CFMT_REG_REG_REG | PAIR_FIRST | PAIR_SECOND | PAIR_THIRD
);
ins!(
    INS_DIV_LONG,
    216,
    ACTION_SETS_REGISTER
        | ACTION_CAN_THROW
        | CFMT_REG_REG_REG
        | PAIR_FIRST
        | PAIR_SECOND
        | PAIR_THIRD
);
ins!(
    INS_REM_LONG,
    217,
    ACTION_SETS_REGISTER
        | ACTION_CAN_THROW
        | CFMT_REG_REG_REG
        | PAIR_FIRST
        | PAIR_SECOND
        | PAIR_THIRD
);
ins!(
    INS_AND_LONG,
    218,
    ACTION_SETS_REGISTER | CFMT_REG_REG_REG | PAIR_FIRST | PAIR_SECOND | PAIR_THIRD
);
ins!(
    INS_OR_LONG,
    219,
    ACTION_SETS_REGISTER | CFMT_REG_REG_REG | PAIR_FIRST | PAIR_SECOND | PAIR_THIRD
);
ins!(
    INS_XOR_LONG,
    220,
    ACTION_SETS_REGISTER | CFMT_REG_REG_REG | PAIR_FIRST | PAIR_SECOND | PAIR_THIRD
);
ins!(
    INS_SHL_LONG,
    221,
    ACTION_SETS_REGISTER | CFMT_REG_REG_REG | PAIR_FIRST | PAIR_SECOND
);
ins!(
    INS_SHR_LONG,
    222,
    ACTION_SETS_REGISTER | CFMT_REG_REG_REG | PAIR_FIRST | PAIR_SECOND
);
ins!(
    INS_USHR_LONG,
    223,
    ACTION_SETS_REGISTER | CFMT_REG_REG_REG | PAIR_FIRST | PAIR_SECOND
);
ins!(INS_ADD_FLOAT, 224, ACTION_SETS_REGISTER | CFMT_REG_REG_REG);
ins!(INS_SUB_FLOAT, 225, ACTION_SETS_REGISTER | CFMT_REG_REG_REG);
ins!(INS_MUL_FLOAT, 226, ACTION_SETS_REGISTER | CFMT_REG_REG_REG);
ins!(INS_DIV_FLOAT, 227, ACTION_SETS_REGISTER | CFMT_REG_REG_REG);
ins!(INS_REM_FLOAT, 228, ACTION_SETS_REGISTER | CFMT_REG_REG_REG);
ins!(
    INS_ADD_DOUBLE,
    229,
    ACTION_SETS_REGISTER | CFMT_REG_REG_REG | PAIR_FIRST | PAIR_SECOND | PAIR_THIRD
);
ins!(
    INS_SUB_DOUBLE,
    230,
    ACTION_SETS_REGISTER | CFMT_REG_REG_REG | PAIR_FIRST | PAIR_SECOND | PAIR_THIRD
);
ins!(
    INS_MUL_DOUBLE,
    231,
    ACTION_SETS_REGISTER | CFMT_REG_REG_REG | PAIR_FIRST | PAIR_SECOND | PAIR_THIRD
);
ins!(
    INS_DIV_DOUBLE,
    232,
    ACTION_SETS_REGISTER | CFMT_REG_REG_REG | PAIR_FIRST | PAIR_SECOND | PAIR_THIRD
);
ins!(
    INS_REM_DOUBLE,
    233,
    ACTION_SETS_REGISTER | CFMT_REG_REG_REG | PAIR_FIRST | PAIR_SECOND | PAIR_THIRD
);
ins!(INS_FILL_ARRAY_DATA, 234, CFMT_REG_LABEL);
ins!(INS_SPARSE_SWITCH, 235, ACTION_SWITCH | CFMT_REG_LABEL);
ins!(INS_PACKED_SWITCH, 236, ACTION_SWITCH | CFMT_REG_LABEL);
ins!(
    INS_MOVE_16,
    237,
    ACTION_SETS_REGISTER | ACTION_MOVE | CFMT_REG_REG
);
ins!(
    INS_MOVE_WIDE_16,
    238,
    ACTION_SETS_REGISTER | ACTION_MOVE | CFMT_REG_REG | PAIR_FIRST | PAIR_SECOND
);
ins!(
    INS_MOVE_OBJECT_16,
    239,
    ACTION_SETS_REGISTER | ACTION_MOVE | CFMT_REG_REG
);
ins!(
    INS_FILLED_NEW_ARRAY,
    240,
    ACTION_SETS_RESULT | ACTION_CAN_THROW | CFMT_ARGS_ARR
);
ins!(
    INS_FILLED_NEW_ARRAY_RANGE,
    241,
    ACTION_SETS_RESULT | ACTION_CAN_THROW | CFMT_ARGS_ARR
);

ins!(
    INS_EXECUTE_INLINE,
    242,
    ACTION_SETS_RESULT | ACTION_INVOKE | ACTION_CAN_THROW | CFMT_ARGS_METHOD
);
ins!(
    INS_EXECUTE_INLINE_RANGE,
    243,
    ACTION_SETS_RESULT | ACTION_INVOKE | ACTION_CAN_THROW | CFMT_ARGS_METHOD
);
ins!(
    INS_INVOKE_CUSTOM,
    244,
    ACTION_SETS_RESULT | ACTION_INVOKE | ACTION_CAN_THROW | CFMT_ARGS_METHOD_CUSTOM
);
ins!(
    INS_INVOKE_VIRTUAL,
    245,
    ACTION_SETS_RESULT | ACTION_INVOKE | ACTION_CAN_THROW | CFMT_ARGS_METHOD
);
ins!(
    INS_INVOKE_SUPER,
    246,
    ACTION_SETS_RESULT | ACTION_INVOKE | ACTION_CAN_THROW | CFMT_ARGS_METHOD
);
ins!(
    INS_INVOKE_DIRECT,
    247,
    ACTION_SETS_RESULT | ACTION_INVOKE | ACTION_CAN_THROW | CFMT_ARGS_METHOD
);
ins!(
    INS_INVOKE_STATIC,
    248,
    ACTION_SETS_RESULT | ACTION_INVOKE | ACTION_CAN_THROW | CFMT_ARGS_METHOD
);
ins!(
    INS_INVOKE_INTERFACE,
    249,
    ACTION_SETS_RESULT | ACTION_INVOKE | ACTION_CAN_THROW | CFMT_ARGS_METHOD
);
ins!(
    INS_INVOKE_DIRECT_EMPTY,
    250,
    ACTION_SETS_RESULT | ACTION_INVOKE | ACTION_CAN_THROW | CFMT_ARGS_METHOD
);
ins!(
    INS_INVOKE_VIRTUAL_QUICK,
    251,
    ACTION_SETS_RESULT | ACTION_INVOKE | ACTION_CAN_THROW | CFMT_ARGS_METHOD
);
ins!(
    INS_INVOKE_SUPER_QUICK,
    252,
    ACTION_SETS_RESULT | ACTION_INVOKE | ACTION_CAN_THROW | CFMT_ARGS_METHOD
);
ins!(
    INS_INVOKE_POLYMORPHIC,
    253,
    ACTION_SETS_RESULT | ACTION_INVOKE | ACTION_CAN_THROW | CFMT_ARGS_METHOD_POLYMORPHIC
);
ins!(
    INS_INVOKE_CUSTOM_RANGE,
    254,
    ACTION_SETS_RESULT | ACTION_INVOKE | ACTION_CAN_THROW | CFMT_ARGS_METHOD_CUSTOM
);
ins!(
    INS_INVOKE_VIRTUAL_RANGE,
    255,
    ACTION_SETS_RESULT | ACTION_INVOKE | ACTION_CAN_THROW | CFMT_ARGS_METHOD
);
ins!(
    INS_INVOKE_SUPER_RANGE,
    256,
    ACTION_SETS_RESULT | ACTION_INVOKE | ACTION_CAN_THROW | CFMT_ARGS_METHOD
);
ins!(
    INS_INVOKE_DIRECT_RANGE,
    257,
    ACTION_SETS_RESULT | ACTION_INVOKE | ACTION_CAN_THROW | CFMT_ARGS_METHOD
);
ins!(
    INS_INVOKE_STATIC_RANGE,
    258,
    ACTION_SETS_RESULT | ACTION_INVOKE | ACTION_CAN_THROW | CFMT_ARGS_METHOD
);
ins!(
    INS_INVOKE_INTERFACE_RANGE,
    259,
    ACTION_SETS_RESULT | ACTION_INVOKE | ACTION_CAN_THROW | CFMT_ARGS_METHOD
);
ins!(
    INS_INVOKE_OBJECT_INIT_RANGE,
    260,
    ACTION_SETS_RESULT | ACTION_INVOKE | ACTION_CAN_THROW | CFMT_ARGS_METHOD
);
ins!(
    INS_INVOKE_VIRTUAL_QUICK_RANGE,
    261,
    ACTION_SETS_RESULT | ACTION_INVOKE | ACTION_CAN_THROW | CFMT_ARGS_METHOD
);
ins!(
    INS_INVOKE_SUPER_QUICK_RANGE,
    262,
    ACTION_SETS_RESULT | ACTION_INVOKE | ACTION_CAN_THROW | CFMT_ARGS_METHOD
);
ins!(
    INS_INVOKE_POLYMORPHIC_RANGE,
    263,
    ACTION_SETS_RESULT | ACTION_INVOKE | ACTION_CAN_THROW | CFMT_ARGS_METHOD_POLYMORPHIC
);

impl From<&str> for Instruction {
    fn from(s: &str) -> Self {
        match s {
            "goto" => INS_GOTO,
            "goto/16" => INS_GOTO_16,
            "goto/32" => INS_GOTO_32,
            "return" => INS_RETURN,
            "return-void" => INS_RETURN_VOID,
            "return-wide" => INS_RETURN_WIDE,
            "return-object" => INS_RETURN_OBJECT,
            "return-void-barrier" => INS_RETURN_VOID_BARRIER,
            "return-void-no-barrier" => INS_RETURN_VOID_NO_BARRIER,
            "nop" => INS_NOP,
            "const" => INS_CONST,
            "const/4" => INS_CONST_4,
            "const/16" => INS_CONST_16,
            "const-wide" => INS_CONST_WIDE,
            "const-wide/16" => INS_CONST_WIDE_16,
            "const-wide/32" => INS_CONST_WIDE_32,
            "const-high16" => INS_CONST_HIGH16,
            "const-wide-high16" => INS_CONST_WIDE_HIGH16,
            "const-string" => INS_CONST_STRING,
            "const-string/jumbo" => INS_CONST_STRING_JUMBO,
            "const-method-handle" => INS_CONST_METHOD_HANDLE,
            "const-method-type" => INS_CONST_METHOD_TYPE,
            "if-eqz" => INS_IF_EQZ,
            "if-nez" => INS_IF_NEZ,
            "if-ltz" => INS_IF_LTZ,
            "if-gez" => INS_IF_GEZ,
            "if-gtz" => INS_IF_GTZ,
            "if-lez" => INS_IF_LEZ,
            "if-eq" => INS_IF_EQ,
            "if-ne" => INS_IF_NE,
            "if-lt" => INS_IF_LT,
            "if-ge" => INS_IF_GE,
            "if-gt" => INS_IF_GT,
            "if-le" => INS_IF_LE,
            "move-result" => INS_MOVE_RESULT,
            "move-result-wide" => INS_MOVE_RESULT_WIDE,
            "move-result-object" => INS_MOVE_RESULT_OBJECT,
            "move-exception" => INS_MOVE_EXCEPTION,
            "monitor-enter" => INS_MONITOR_ENTER,
            "monitor-exit" => INS_MONITOR_EXIT,
            "throw" => INS_THROW,
            "move" => INS_MOVE,
            "move-wide" => INS_MOVE_WIDE,
            "move-object" => INS_MOVE_OBJECT,
            "array-length" => INS_ARRAY_LENGTH,
            "neg-int" => INS_NEG_INT,
            "not-int" => INS_NOT_INT,
            "neg-long" => INS_NEG_LONG,
            "not-long" => INS_NOT_LONG,
            "neg-float" => INS_NEG_FLOAT,
            "neg-double" => INS_NEG_DOUBLE,
            "int-to-long" => INS_INT_TO_LONG,
            "int-to-float" => INS_INT_TO_FLOAT,
            "int-to-double" => INS_INT_TO_DOUBLE,
            "long-to-int" => INS_LONG_TO_INT,
            "long-to-float" => INS_LONG_TO_FLOAT,
            "long-to-double" => INS_LONG_TO_DOUBLE,
            "float-to-int" => INS_FLOAT_TO_INT,
            "float-to-long" => INS_FLOAT_TO_LONG,
            "float-to-double" => INS_FLOAT_TO_DOUBLE,
            "double-to-int" => INS_DOUBLE_TO_INT,
            "double-to-long" => INS_DOUBLE_TO_LONG,
            "double-to-float" => INS_DOUBLE_TO_FLOAT,
            "int-to-byte" => INS_INT_TO_BYTE,
            "int-to-char" => INS_INT_TO_CHAR,
            "int-to-short" => INS_INT_TO_SHORT,
            "add-int/2addr" => INS_ADD_INT_2ADDR,
            "sub-int/2addr" => INS_SUB_INT_2ADDR,
            "mul-int/2addr" => INS_MUL_INT_2ADDR,
            "div-int/2addr" => INS_DIV_INT_2ADDR,
            "rem-int/2addr" => INS_REM_INT_2ADDR,
            "and-int/2addr" => INS_AND_INT_2ADDR,
            "or-int/2addr" => INS_OR_INT_2ADDR,
            "xor-int/2addr" => INS_XOR_INT_2ADDR,
            "shl-int/2addr" => INS_SHL_INT_2ADDR,
            "shr-int/2addr" => INS_SHR_INT_2ADDR,
            "ushr-int/2addr" => INS_USHR_INT_2ADDR,
            "add-long/2addr" => INS_ADD_LONG_2ADDR,
            "sub-long/2addr" => INS_SUB_LONG_2ADDR,
            "mul-long/2addr" => INS_MUL_LONG_2ADDR,
            "div-long/2addr" => INS_DIV_LONG_2ADDR,
            "rem-long/2addr" => INS_REM_LONG_2ADDR,
            "and-long/2addr" => INS_AND_LONG_2ADDR,
            "or-long/2addr" => INS_OR_LONG_2ADDR,
            "xor-long/2addr" => INS_XOR_LONG_2ADDR,
            "shl-long/2addr" => INS_SHL_LONG_2ADDR,
            "shr-long/2addr" => INS_SHR_LONG_2ADDR,
            "ushr-long/2addr" => INS_USHR_LONG_2ADDR,
            "add-float/2addr" => INS_ADD_FLOAT_2ADDR,
            "sub-float/2addr" => INS_SUB_FLOAT_2ADDR,
            "mul-float/2addr" => INS_MUL_FLOAT_2ADDR,
            "div-float/2addr" => INS_DIV_FLOAT_2ADDR,
            "rem-float/2addr" => INS_REM_FLOAT_2ADDR,
            "add-double/2addr" => INS_ADD_DOUBLE_2ADDR,
            "sub-double/2addr" => INS_SUB_DOUBLE_2ADDR,
            "mul-double/2addr" => INS_MUL_DOUBLE_2ADDR,
            "div-double/2addr" => INS_DIV_DOUBLE_2ADDR,
            "rem-double/2addr" => INS_REM_DOUBLE_2ADDR,
            "sget" => INS_SGET,
            "sget-wide" => INS_SGET_WIDE,
            "sget-object" => INS_SGET_OBJECT,
            "sget-boolean" => INS_SGET_BOOLEAN,
            "sget-byte" => INS_SGET_BYTE,
            "sget-char" => INS_SGET_CHAR,
            "sget-short" => INS_SGET_SHORT,
            "sput" => INS_SPUT,
            "sput-wide" => INS_SPUT_WIDE,
            "sput-object" => INS_SPUT_OBJECT,
            "sput-boolean" => INS_SPUT_BOOLEAN,
            "sput-byte" => INS_SPUT_BYTE,
            "sput-char" => INS_SPUT_CHAR,
            "sput-short" => INS_SPUT_SHORT,
            "sget-volatile" => INS_SGET_VOLATILE,
            "sget-wide-volatile" => INS_SGET_WIDE_VOLATILE,
            "sget-object-volatile" => INS_SGET_OBJECT_VOLATILE,
            "sput-volatile" => INS_SPUT_VOLATILE,
            "sput-wide-volatile" => INS_SPUT_WIDE_VOLATILE,
            "sput-object-volatile" => INS_SPUT_OBJECT_VOLATILE,
            "check-cast" => INS_CHECK_CAST,
            "new-instance" => INS_NEW_INSTANCE,
            "const-class" => INS_CONST_CLASS,
            "add-int/lit8" => INS_ADD_INT_LIT8,
            "rsub-int/lit8" => INS_RSUB_INT_LIT8,
            "mul-int/lit8" => INS_MUL_INT_LIT8,
            "div-int/lit8" => INS_DIV_INT_LIT8,
            "rem-int/lit8" => INS_REM_INT_LIT8,
            "and-int/lit8" => INS_AND_INT_LIT8,
            "or-int/lit8" => INS_OR_INT_LIT8,
            "xor-int/lit8" => INS_XOR_INT_LIT8,
            "shl-int/lit8" => INS_SHL_INT_LIT8,
            "shr-int/lit8" => INS_SHR_INT_LIT8,
            "ushr-int/lit8" => INS_USHR_INT_LIT8,
            "iget" => INS_IGET,
            "iget-wide" => INS_IGET_WIDE,
            "iget-object" => INS_IGET_OBJECT,
            "iget-boolean" => INS_IGET_BOOLEAN,
            "iget-byte" => INS_IGET_BYTE,
            "iget-char" => INS_IGET_CHAR,
            "iget-short" => INS_IGET_SHORT,
            "iput" => INS_IPUT,
            "iput-wide" => INS_IPUT_WIDE,
            "iput-object" => INS_IPUT_OBJECT,
            "iput-boolean" => INS_IPUT_BOOLEAN,
            "iput-byte" => INS_IPUT_BYTE,
            "iput-char" => INS_IPUT_CHAR,
            "iput-short" => INS_IPUT_SHORT,
            "iget-volatile" => INS_IGET_VOLATILE,
            "iget-wide-volatile" => INS_IGET_WIDE_VOLATILE,
            "iget-object-volatile" => INS_IGET_OBJECT_VOLATILE,
            "iput-volatile" => INS_IPUT_VOLATILE,
            "iput-wide-volatile" => INS_IPUT_WIDE_VOLATILE,
            "iput-object-volatile" => INS_IPUT_OBJECT_VOLATILE,
            "instance-of" => INS_INSTANCE_OF,
            "new-array" => INS_NEW_ARRAY,
            "iget-quick" => INS_IGET_QUICK,
            "iget-wide-quick" => INS_IGET_WIDE_QUICK,
            "iget-object-quick" => INS_IGET_OBJECT_QUICK,
            "iput-quick" => INS_IPUT_QUICK,
            "iput-wide-quick" => INS_IPUT_WIDE_QUICK,
            "iput-object-quick" => INS_IPUT_OBJECT_QUICK,
            "iput-boolean-quick" => INS_IPUT_BOOLEAN_QUICK,
            "iput-byte-quick" => INS_IPUT_BYTE_QUICK,
            "iput-char-quick" => INS_IPUT_CHAR_QUICK,
            "iput-short-quick" => INS_IPUT_SHORT_QUICK,
            "rsub-int" => INS_RSUB_INT,
            "add-int/lit16" => INS_ADD_INT_LIT16,
            "mul-int/lit16" => INS_MUL_INT_LIT16,
            "div-int/lit16" => INS_DIV_INT_LIT16,
            "rem-int/lit16" => INS_REM_INT_LIT16,
            "and-int/lit16" => INS_AND_INT_LIT16,
            "or-int/lit16" => INS_OR_INT_LIT16,
            "xor-int/lit16" => INS_XOR_INT_LIT16,
            "move/from16" => INS_MOVE_FROM16,
            "move-wide/from16" => INS_MOVE_WIDE_FROM16,
            "move-object/from16" => INS_MOVE_OBJECT_FROM16,
            "cmpl-float" => INS_CMPL_FLOAT,
            "cmpg-float" => INS_CMPG_FLOAT,
            "cmpl-double" => INS_CMPL_DOUBLE,
            "cmpg-double" => INS_CMPG_DOUBLE,
            "cmp-long" => INS_CMP_LONG,
            "aget" => INS_AGET,
            "aget-wide" => INS_AGET_WIDE,
            "aget-object" => INS_AGET_OBJECT,
            "aget-boolean" => INS_AGET_BOOLEAN,
            "aget-byte" => INS_AGET_BYTE,
            "aget-char" => INS_AGET_CHAR,
            "aget-short" => INS_AGET_SHORT,
            "aput" => INS_APUT,
            "aput-wide" => INS_APUT_WIDE,
            "aput-object" => INS_APUT_OBJECT,
            "aput-boolean" => INS_APUT_BOOLEAN,
            "aput-byte" => INS_APUT_BYTE,
            "aput-char" => INS_APUT_CHAR,
            "aput-short" => INS_APUT_SHORT,
            "add-int" => INS_ADD_INT,
            "sub-int" => INS_SUB_INT,
            "mul-int" => INS_MUL_INT,
            "div-int" => INS_DIV_INT,
            "rem-int" => INS_REM_INT,
            "and-int" => INS_AND_INT,
            "or-int" => INS_OR_INT,
            "xor-int" => INS_XOR_INT,
            "shl-int" => INS_SHL_INT,
            "shr-int" => INS_SHR_INT,
            "ushr-int" => INS_USHR_INT,
            "add-long" => INS_ADD_LONG,
            "sub-long" => INS_SUB_LONG,
            "mul-long" => INS_MUL_LONG,
            "div-long" => INS_DIV_LONG,
            "rem-long" => INS_REM_LONG,
            "and-long" => INS_AND_LONG,
            "or-long" => INS_OR_LONG,
            "xor-long" => INS_XOR_LONG,
            "shl-long" => INS_SHL_LONG,
            "shr-long" => INS_SHR_LONG,
            "ushr-long" => INS_USHR_LONG,
            "add-float" => INS_ADD_FLOAT,
            "sub-float" => INS_SUB_FLOAT,
            "mul-float" => INS_MUL_FLOAT,
            "div-float" => INS_DIV_FLOAT,
            "rem-float" => INS_REM_FLOAT,
            "add-double" => INS_ADD_DOUBLE,
            "sub-double" => INS_SUB_DOUBLE,
            "mul-double" => INS_MUL_DOUBLE,
            "div-double" => INS_DIV_DOUBLE,
            "rem-double" => INS_REM_DOUBLE,
            "fill-array-data" => INS_FILL_ARRAY_DATA,
            "packed-switch" => INS_PACKED_SWITCH,
            "sparse-switch" => INS_SPARSE_SWITCH,
            "move/16" => INS_MOVE_16,
            "move-wide/16" => INS_MOVE_WIDE_16,
            "move-object/16" => INS_MOVE_OBJECT_16,
            "filled-new-array" => INS_FILLED_NEW_ARRAY,
            "filled-new-array/range" => INS_FILLED_NEW_ARRAY_RANGE,
            "execute-inline" => INS_EXECUTE_INLINE,
            "execute-inline/range" => INS_EXECUTE_INLINE_RANGE,
            "invoke-custom" => INS_INVOKE_CUSTOM,
            "invoke-virtual" => INS_INVOKE_VIRTUAL,
            "invoke-super" => INS_INVOKE_SUPER,
            "invoke-direct" => INS_INVOKE_DIRECT,
            "invoke-static" => INS_INVOKE_STATIC,
            "invoke-interface" => INS_INVOKE_INTERFACE,
            "invoke-direct-empty" => INS_INVOKE_DIRECT_EMPTY,
            "invoke-virtual-quick" => INS_INVOKE_VIRTUAL_QUICK,
            "invoke-super-quick" => INS_INVOKE_SUPER_QUICK,
            "invoke-polymorphic" => INS_INVOKE_POLYMORPHIC,
            "invoke-custom/range" => INS_INVOKE_CUSTOM_RANGE,
            "invoke-virtual/range" => INS_INVOKE_VIRTUAL_RANGE,
            "invoke-super/range" => INS_INVOKE_SUPER_RANGE,
            "invoke-direct/range" => INS_INVOKE_DIRECT_RANGE,
            "invoke-static/range" => INS_INVOKE_STATIC_RANGE,
            "invoke-interface/range" => INS_INVOKE_INTERFACE_RANGE,
            "invoke-object-init/range" => INS_INVOKE_OBJECT_INIT_RANGE,
            "invoke-virtual-quick/range" => INS_INVOKE_VIRTUAL_QUICK_RANGE,
            "invoke-super-quick/range" => INS_INVOKE_SUPER_QUICK_RANGE,
            "invoke-polymorphic/range" => INS_INVOKE_POLYMORPHIC_RANGE,
            _ => INS_UNKNOWN_INSTRUCTION,
        }
    }
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                INS_GOTO => "goto",
                INS_GOTO_16 => "goto/16",
                INS_GOTO_32 => "goto/32",
                INS_RETURN => "return",
                INS_RETURN_VOID => "return-void",
                INS_RETURN_WIDE => "return-wide",
                INS_RETURN_OBJECT => "return-object",
                INS_RETURN_VOID_BARRIER => "return-void-barrier",
                INS_RETURN_VOID_NO_BARRIER => "return-void-no-barrier",
                INS_NOP => "nop",
                INS_CONST => "const",
                INS_CONST_4 => "const/4",
                INS_CONST_16 => "const/16",
                INS_CONST_WIDE => "const-wide",
                INS_CONST_WIDE_16 => "const-wide/16",
                INS_CONST_WIDE_32 => "const-wide/32",
                INS_CONST_HIGH16 => "const-high16",
                INS_CONST_WIDE_HIGH16 => "const-wide-high16",
                INS_CONST_STRING => "const-string",
                INS_CONST_STRING_JUMBO => "const-string/jumbo",
                INS_CONST_METHOD_HANDLE => "const-method-handle",
                INS_CONST_METHOD_TYPE => "const-method-type",
                INS_IF_EQZ => "if-eqz",
                INS_IF_NEZ => "if-nez",
                INS_IF_LTZ => "if-ltz",
                INS_IF_GEZ => "if-gez",
                INS_IF_GTZ => "if-gtz",
                INS_IF_LEZ => "if-lez",
                INS_IF_EQ => "if-eq",
                INS_IF_NE => "if-ne",
                INS_IF_LT => "if-lt",
                INS_IF_GE => "if-ge",
                INS_IF_GT => "if-gt",
                INS_IF_LE => "if-le",
                INS_MOVE_RESULT => "move-result",
                INS_MOVE_RESULT_WIDE => "move-result-wide",
                INS_MOVE_RESULT_OBJECT => "move-result-object",
                INS_MOVE_EXCEPTION => "move-exception",
                INS_MONITOR_ENTER => "monitor-enter",
                INS_MONITOR_EXIT => "monitor-exit",
                INS_THROW => "throw",
                INS_MOVE => "move",
                INS_MOVE_WIDE => "move-wide",
                INS_MOVE_OBJECT => "move-object",
                INS_ARRAY_LENGTH => "array-length",
                INS_NEG_INT => "neg-int",
                INS_NOT_INT => "not-int",
                INS_NEG_LONG => "neg-long",
                INS_NOT_LONG => "not-long",
                INS_NEG_FLOAT => "neg-float",
                INS_NEG_DOUBLE => "neg-double",
                INS_INT_TO_LONG => "int-to-long",
                INS_INT_TO_FLOAT => "int-to-float",
                INS_INT_TO_DOUBLE => "int-to-double",
                INS_LONG_TO_INT => "long-to-int",
                INS_LONG_TO_FLOAT => "long-to-float",
                INS_LONG_TO_DOUBLE => "long-to-double",
                INS_FLOAT_TO_INT => "float-to-int",
                INS_FLOAT_TO_LONG => "float-to-long",
                INS_FLOAT_TO_DOUBLE => "float-to-double",
                INS_DOUBLE_TO_INT => "double-to-int",
                INS_DOUBLE_TO_LONG => "double-to-long",
                INS_DOUBLE_TO_FLOAT => "double-to-float",
                INS_INT_TO_BYTE => "int-to-byte",
                INS_INT_TO_CHAR => "int-to-char",
                INS_INT_TO_SHORT => "int-to-short",
                INS_ADD_INT_2ADDR => "add-int/2addr",
                INS_SUB_INT_2ADDR => "sub-int/2addr",
                INS_MUL_INT_2ADDR => "mul-int/2addr",
                INS_DIV_INT_2ADDR => "div-int/2addr",
                INS_REM_INT_2ADDR => "rem-int/2addr",
                INS_AND_INT_2ADDR => "and-int/2addr",
                INS_OR_INT_2ADDR => "or-int/2addr",
                INS_XOR_INT_2ADDR => "xor-int/2addr",
                INS_SHL_INT_2ADDR => "shl-int/2addr",
                INS_SHR_INT_2ADDR => "shr-int/2addr",
                INS_USHR_INT_2ADDR => "ushr-int/2addr",
                INS_ADD_LONG_2ADDR => "add-long/2addr",
                INS_SUB_LONG_2ADDR => "sub-long/2addr",
                INS_MUL_LONG_2ADDR => "mul-long/2addr",
                INS_DIV_LONG_2ADDR => "div-long/2addr",
                INS_REM_LONG_2ADDR => "rem-long/2addr",
                INS_AND_LONG_2ADDR => "and-long/2addr",
                INS_OR_LONG_2ADDR => "or-long/2addr",
                INS_XOR_LONG_2ADDR => "xor-long/2addr",
                INS_SHL_LONG_2ADDR => "shl-long/2addr",
                INS_SHR_LONG_2ADDR => "shr-long/2addr",
                INS_USHR_LONG_2ADDR => "ushr-long/2addr",
                INS_ADD_FLOAT_2ADDR => "add-float/2addr",
                INS_SUB_FLOAT_2ADDR => "sub-float/2addr",
                INS_MUL_FLOAT_2ADDR => "mul-float/2addr",
                INS_DIV_FLOAT_2ADDR => "div-float/2addr",
                INS_REM_FLOAT_2ADDR => "rem-float/2addr",
                INS_ADD_DOUBLE_2ADDR => "add-double/2addr",
                INS_SUB_DOUBLE_2ADDR => "sub-double/2addr",
                INS_MUL_DOUBLE_2ADDR => "mul-double/2addr",
                INS_DIV_DOUBLE_2ADDR => "div-double/2addr",
                INS_REM_DOUBLE_2ADDR => "rem-double/2addr",
                INS_SGET => "sget",
                INS_SGET_WIDE => "sget-wide",
                INS_SGET_OBJECT => "sget-object",
                INS_SGET_BOOLEAN => "sget-boolean",
                INS_SGET_BYTE => "sget-byte",
                INS_SGET_CHAR => "sget-char",
                INS_SGET_SHORT => "sget-short",
                INS_SPUT => "sput",
                INS_SPUT_WIDE => "sput-wide",
                INS_SPUT_OBJECT => "sput-object",
                INS_SPUT_BOOLEAN => "sput-boolean",
                INS_SPUT_BYTE => "sput-byte",
                INS_SPUT_CHAR => "sput-char",
                INS_SPUT_SHORT => "sput-short",
                INS_SGET_VOLATILE => "sget-volatile",
                INS_SGET_WIDE_VOLATILE => "sget-wide-volatile",
                INS_SGET_OBJECT_VOLATILE => "sget-object-volatile",
                INS_SPUT_VOLATILE => "sput-volatile",
                INS_SPUT_WIDE_VOLATILE => "sput-wide-volatile",
                INS_SPUT_OBJECT_VOLATILE => "sput-object-volatile",
                INS_CHECK_CAST => "check-cast",
                INS_NEW_INSTANCE => "new-instance",
                INS_CONST_CLASS => "const-class",
                INS_ADD_INT_LIT8 => "add-int/lit8",
                INS_RSUB_INT_LIT8 => "rsub-int/lit8",
                INS_MUL_INT_LIT8 => "mul-int/lit8",
                INS_DIV_INT_LIT8 => "div-int/lit8",
                INS_REM_INT_LIT8 => "rem-int/lit8",
                INS_AND_INT_LIT8 => "and-int/lit8",
                INS_OR_INT_LIT8 => "or-int/lit8",
                INS_XOR_INT_LIT8 => "xor-int/lit8",
                INS_SHL_INT_LIT8 => "shl-int/lit8",
                INS_SHR_INT_LIT8 => "shr-int/lit8",
                INS_USHR_INT_LIT8 => "ushr-int/lit8",
                INS_IGET => "iget",
                INS_IGET_WIDE => "iget-wide",
                INS_IGET_OBJECT => "iget-object",
                INS_IGET_BOOLEAN => "iget-boolean",
                INS_IGET_BYTE => "iget-byte",
                INS_IGET_CHAR => "iget-char",
                INS_IGET_SHORT => "iget-short",
                INS_IPUT => "iput",
                INS_IPUT_WIDE => "iput-wide",
                INS_IPUT_OBJECT => "iput-object",
                INS_IPUT_BOOLEAN => "iput-boolean",
                INS_IPUT_BYTE => "iput-byte",
                INS_IPUT_CHAR => "iput-char",
                INS_IPUT_SHORT => "iput-short",
                INS_IGET_VOLATILE => "iget-volatile",
                INS_IGET_WIDE_VOLATILE => "iget-wide-volatile",
                INS_IGET_OBJECT_VOLATILE => "iget-object-volatile",
                INS_IPUT_VOLATILE => "iput-volatile",
                INS_IPUT_WIDE_VOLATILE => "iput-wide-volatile",
                INS_IPUT_OBJECT_VOLATILE => "iput-object-volatile",
                INS_INSTANCE_OF => "instance-of",
                INS_NEW_ARRAY => "new-array",
                INS_IGET_QUICK => "iget-quick",
                INS_IGET_WIDE_QUICK => "iget-wide-quick",
                INS_IGET_OBJECT_QUICK => "iget-object-quick",
                INS_IPUT_QUICK => "iput-quick",
                INS_IPUT_WIDE_QUICK => "iput-wide-quick",
                INS_IPUT_OBJECT_QUICK => "iput-object-quick",
                INS_IPUT_BOOLEAN_QUICK => "iput-boolean-quick",
                INS_IPUT_BYTE_QUICK => "iput-byte-quick",
                INS_IPUT_CHAR_QUICK => "iput-char-quick",
                INS_IPUT_SHORT_QUICK => "iput-short-quick",
                INS_RSUB_INT => "rsub-int",
                INS_ADD_INT_LIT16 => "add-int/lit16",
                INS_MUL_INT_LIT16 => "mul-int/lit16",
                INS_DIV_INT_LIT16 => "div-int/lit16",
                INS_REM_INT_LIT16 => "rem-int/lit16",
                INS_AND_INT_LIT16 => "and-int/lit16",
                INS_OR_INT_LIT16 => "or-int/lit16",
                INS_XOR_INT_LIT16 => "xor-int/lit16",
                INS_MOVE_FROM16 => "move/from16",
                INS_MOVE_WIDE_FROM16 => "move-wide/from16",
                INS_MOVE_OBJECT_FROM16 => "move-object/from16",
                INS_CMPL_FLOAT => "cmpl-float",
                INS_CMPG_FLOAT => "cmpg-float",
                INS_CMPL_DOUBLE => "cmpl-double",
                INS_CMPG_DOUBLE => "cmpg-double",
                INS_CMP_LONG => "cmp-long",
                INS_AGET => "aget",
                INS_AGET_WIDE => "aget-wide",
                INS_AGET_OBJECT => "aget-object",
                INS_AGET_BOOLEAN => "aget-boolean",
                INS_AGET_BYTE => "aget-byte",
                INS_AGET_CHAR => "aget-char",
                INS_AGET_SHORT => "aget-short",
                INS_APUT => "aput",
                INS_APUT_WIDE => "aput-wide",
                INS_APUT_OBJECT => "aput-object",
                INS_APUT_BOOLEAN => "aput-boolean",
                INS_APUT_BYTE => "aput-byte",
                INS_APUT_CHAR => "aput-char",
                INS_APUT_SHORT => "aput-short",
                INS_ADD_INT => "add-int",
                INS_SUB_INT => "sub-int",
                INS_MUL_INT => "mul-int",
                INS_DIV_INT => "div-int",
                INS_REM_INT => "rem-int",
                INS_AND_INT => "and-int",
                INS_OR_INT => "or-int",
                INS_XOR_INT => "xor-int",
                INS_SHL_INT => "shl-int",
                INS_SHR_INT => "shr-int",
                INS_USHR_INT => "ushr-int",
                INS_ADD_LONG => "add-long",
                INS_SUB_LONG => "sub-long",
                INS_MUL_LONG => "mul-long",
                INS_DIV_LONG => "div-long",
                INS_REM_LONG => "rem-long",
                INS_AND_LONG => "and-long",
                INS_OR_LONG => "or-long",
                INS_XOR_LONG => "xor-long",
                INS_SHL_LONG => "shl-long",
                INS_SHR_LONG => "shr-long",
                INS_USHR_LONG => "ushr-long",
                INS_ADD_FLOAT => "add-float",
                INS_SUB_FLOAT => "sub-float",
                INS_MUL_FLOAT => "mul-float",
                INS_DIV_FLOAT => "div-float",
                INS_REM_FLOAT => "rem-float",
                INS_ADD_DOUBLE => "add-double",
                INS_SUB_DOUBLE => "sub-double",
                INS_MUL_DOUBLE => "mul-double",
                INS_DIV_DOUBLE => "div-double",
                INS_REM_DOUBLE => "rem-double",
                INS_FILL_ARRAY_DATA => "fill-array-data",
                INS_PACKED_SWITCH => "packed-switch",
                INS_SPARSE_SWITCH => "sparse-switch",
                INS_MOVE_16 => "move/16",
                INS_MOVE_WIDE_16 => "move-wide/16",
                INS_MOVE_OBJECT_16 => "move-object/16",
                INS_FILLED_NEW_ARRAY => "filled-new-array",
                INS_FILLED_NEW_ARRAY_RANGE => "filled-new-array/range",
                INS_EXECUTE_INLINE => "execute-inline",
                INS_EXECUTE_INLINE_RANGE => "execute-inline/range",
                INS_INVOKE_CUSTOM => "invoke-custom",
                INS_INVOKE_VIRTUAL => "invoke-virtual",
                INS_INVOKE_SUPER => "invoke-super",
                INS_INVOKE_DIRECT => "invoke-direct",
                INS_INVOKE_STATIC => "invoke-static",
                INS_INVOKE_INTERFACE => "invoke-interface",
                INS_INVOKE_DIRECT_EMPTY => "invoke-direct-empty",
                INS_INVOKE_VIRTUAL_QUICK => "invoke-virtual-quick",
                INS_INVOKE_SUPER_QUICK => "invoke-super-quick",
                INS_INVOKE_POLYMORPHIC => "invoke-polymorphic",
                INS_INVOKE_CUSTOM_RANGE => "invoke-custom/range",
                INS_INVOKE_VIRTUAL_RANGE => "invoke-virtual/range",
                INS_INVOKE_SUPER_RANGE => "invoke-super/range",
                INS_INVOKE_DIRECT_RANGE => "invoke-direct/range",
                INS_INVOKE_STATIC_RANGE => "invoke-static/range",
                INS_INVOKE_INTERFACE_RANGE => "invoke-interface/range",
                INS_INVOKE_OBJECT_INIT_RANGE => "invoke-object-init/range",
                INS_INVOKE_VIRTUAL_QUICK_RANGE => "invoke-virtual-quick/range",
                INS_INVOKE_SUPER_QUICK_RANGE => "invoke-super-quick/range",
                INS_INVOKE_POLYMORPHIC_RANGE => "invoke-polymorphic/range",
                _ => "?????",
            }
        )
    }
}

impl Deref for Instruction {
    type Target = InsBits;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Hash for Instruction {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state)
    }
}

impl Instruction {
    /// Return only the bits important for checking formats. This value
    /// can be compared directly to INS_FMT* constants.
    #[inline]
    pub fn fmt(self) -> InsBits {
        self.0 & InsBits::FMT_MASK
    }

    /// Returns only the bits important for checking actions. This value
    /// can be compared directly to INS_ACTION* constants.
    #[inline]
    pub fn action(self) -> InsBits {
        self.0 & InsBits::ACTION_MASK
    }

    /// Checks to see if the instruction sets a register.
    #[inline]
    pub fn sets_register(self) -> bool {
        self.0.contains(InsBits::ACTION_SETS_REGISTER)
    }

    /// Checks to see if the instruction returns.
    #[inline]
    pub fn is_return(self) -> bool {
        self.0.contains(InsBits::ACTION_RETURN)
    }

    /// Checks to see if the instruction returns a value instead of just simply returns
    #[inline]
    pub fn returns_value(self) -> bool {
        self.is_return() && !self.0.contains(InsBits::FMT_BARE)
    }

    /// Checks if the instruction can throw an exception
    #[inline]
    pub fn can_throw(self) -> bool {
        self.0.contains(InsBits::ACTION_CAN_THROW)
    }

    /// The instruction is a simple move, this doesn't include move-result or move-exception
    /// variants
    #[inline]
    pub fn is_move(self) -> bool {
        self.0.contains(InsBits::ACTION_MOVE)
    }

    /// The instruction is a move result variant
    #[inline]
    pub fn is_move_result(self) -> bool {
        self.0.contains(InsBits::ACTION_MOVE_RESULT)
    }

    #[inline]
    pub fn writes_result(self) -> bool {
        self.0.contains(InsBits::ACTION_SETS_RESULT)
    }

    #[inline]
    pub fn reads_result(self) -> bool {
        self.0.contains(InsBits::ACTION_MOVE_RESULT)
    }

    /// Checks whether the first register names a 64 bit pair, meaning the
    /// register numbered one higher is implicitly part of it.
    #[inline]
    pub fn pair_first(self) -> bool {
        self.0.contains(InsBits::PAIR_FIRST)
    }

    /// Checks whether the second register names a 64 bit pair.
    #[inline]
    pub fn pair_second(self) -> bool {
        self.0.contains(InsBits::PAIR_SECOND)
    }

    /// Checks whether the third register names a 64 bit pair.
    #[inline]
    pub fn pair_third(self) -> bool {
        self.0.contains(InsBits::PAIR_THIRD)
    }

    /// Checks whether any register operand names a 64 bit pair.
    #[inline]
    pub fn has_pair(self) -> bool {
        self.0.intersects(InsBits::PAIR_MASK)
    }

    /// Checks whether the first register is read before it is written. Always
    /// implies [Instruction::sets_register].
    #[inline]
    pub fn is_inout(self) -> bool {
        self.0.contains(InsBits::ACTION_INOUT)
    }

    /// Checks whether the instruction ends a basic block. Note that
    /// [Instruction::can_throw] is not a terminator on its own since those
    /// instructions still fall through when no exception is raised.
    #[inline]
    pub fn is_terminator(self) -> bool {
        self.is_return() || self == INS_THROW || self.is_cond() || self.is_jump()
    }

    /// Checks to see if the instruction calls another method.
    #[inline]
    pub fn is_call(self) -> bool {
        self.0.contains(InsBits::ACTION_INVOKE)
    }

    /// Checks to see if the instruction sets an array element.
    #[inline]
    pub fn sets_array_element(self) -> bool {
        self.0.contains(InsBits::ACTION_SETS_ARRAY_ELEMENT)
    }

    /// Checks to see if the instruction gets an array element.
    #[inline]
    pub fn gets_array_element(self) -> bool {
        self.0.contains(InsBits::ACTION_GETS_ARRAY_ELEMENT)
    }

    /// Checks to see if the instruction gets a static field
    #[inline]
    pub fn gets_static_field(self) -> bool {
        self.0.contains(InsBits::ACTION_GETS_STATIC_FIELD)
    }

    /// Checks to see if the instruction gets an instance field
    #[inline]
    pub fn gets_instance_field(self) -> bool {
        self.0.contains(InsBits::ACTION_GETS_INSTANCE_FIELD)
    }

    /// Checks to see if the instruction gets a field (static or instance)
    #[inline]
    pub fn gets_field(self) -> bool {
        self.gets_instance_field() || self.gets_static_field()
    }

    /// Checks to see if the instruction sets a static field
    #[inline]
    pub fn sets_static_field(self) -> bool {
        self.0.contains(InsBits::ACTION_SETS_STATIC_FIELD)
    }

    /// Checks to see if the instruction sets an instance field
    #[inline]
    pub fn sets_instance_field(self) -> bool {
        self.0.contains(InsBits::ACTION_SETS_INSTANCE_FIELD)
    }

    /// Checks to see if the instruction sets a field (static or instance)
    #[inline]
    pub fn sets_field(self) -> bool {
        self.sets_static_field() || self.sets_instance_field()
    }

    #[inline]
    pub fn is_jump(self) -> bool {
        self.0.contains(InsBits::ACTION_UNCOND_JUMP)
    }

    #[inline]
    pub fn is_switch(self) -> bool {
        self.0.contains(InsBits::ACTION_SWITCH)
    }

    #[inline]
    pub fn is_cond(self) -> bool {
        self.is_forking_cond() || self.is_switch()
    }

    /// Checks to see if the instruction is a conditional with only two options.
    #[inline]
    pub fn is_forking_cond(self) -> bool {
        self.0.contains(InsBits::ACTION_FORKING_COND)
    }

    /// Return the raw bits, this value should be treated as opaque but unique to a given
    /// instruction
    ///
    /// The raw bits used to encode an instruction should be stable within a major version of this
    /// crate
    #[inline]
    pub fn raw(self) -> u64 {
        self.0.bits()
    }

    /// Meant to be used as a counterpart to [Instruction::raw] for serialization purposes
    ///
    /// No checks are performed on the passed bits, it is possible to create an invalid
    /// instruction this way!
    #[inline]
    pub fn from_raw(raw: u64) -> Self {
        Self(InsBits::from_bits_retain(raw))
    }
}

impl fmt::Debug for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn raw_roundtrip() {
        let val = INS_SUB_LONG_2ADDR;
        assert_eq!(val, Instruction::from_raw(val.raw()));
    }
}
