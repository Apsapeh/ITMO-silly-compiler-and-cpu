#[derive(Clone, Copy, Default, Debug)]
#[repr(u8)]
pub enum Opcode {
    #[default]
    Nop = 0u8,
    Hlt = 1u8,
    Mov = 2u8,
    Add = 3u8,
    AddC = 4u8,
    Sub = 5u8,
    SubC = 6u8,
    Mul = 7u8,
    Div = 8u8,
    Inc = 9u8,
    Dec = 10u8,
    Neg = 11u8,
    And = 12u8,
    Or = 13u8,
    Xor = 14u8,
    Not = 15u8,
    Shl = 16u8,
    Shr = 17u8,
    Cmp = 20u8,
    Test = 21u8,
    Jmp = 22u8,
    Je = 23u8,
    Jne = 24u8,
    Jns = 25u8,
    Jnc = 26u8,
    Jcs = 27u8,
    Jcc = 28u8,
    Jos = 29u8,
    Joc = 30u8,
    Jl = 31u8,
    Jle = 32u8,
    Jg = 33u8,
    Jge = 34u8,
    Call = 35u8,
    Ret = 36u8,
    Push = 37u8,
    Pop = 38u8,
    Enter = 39u8,
    Leave = 40u8,
    Iret = 42u8,
    Sti = 43u8,
    Cli = 44u8,
}

impl Opcode {
    pub fn from_raw(value: u8) -> Option<Self> {
        if matches!(value, 0..=17 | 20..=40 | 42..=44) {
            Some(unsafe { std::mem::transmute::<u8, Opcode>(value) })
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Default, Debug)]
#[repr(u8)]
pub enum Mode {
    #[default]
    RegToReg = 0x0u8,
    ImmToReg = 0x1u8,
    MemrToReg = 0x2u8,
    MemaToReg = 0x3u8,
    MemraToReg = 0x4u8,
    RegToMemr = 0x5u8,
    ImmToMemr = 0x6u8,
    RegToMema = 0x7u8,
    ImmToMema = 0x8u8,
    RegToMemra = 0x9u8,
    ImmToMemra = 0xAu8,
}

impl Mode {
    pub fn from_raw(value: u8) -> Option<Self> {
        if value <= 0xAu8 {
            Some(unsafe { std::mem::transmute::<u8, Mode>(value) })
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Default, Debug)]
#[repr(u8)]
pub enum Register {
    #[default]
    AL = 0x0u8,
    AH = 0x1u8,
    BL = 0x2u8,
    BH = 0x3u8,
    CL = 0x4u8,
    CH = 0x5u8,
    SP = 0x6u8,
    BP = 0x7u8,
}

impl Register {
    pub fn from_raw(value: u8) -> Option<Self> {
        if value <= 0x7u8 {
            Some(unsafe { std::mem::transmute::<u8, Register>(value) })
        } else {
            None
        }
    }
}
