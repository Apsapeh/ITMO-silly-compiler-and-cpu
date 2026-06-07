use crate::data_path;
use crate::general::*;

#[derive(Default, Clone, Copy)]
pub enum IpSrc {
    #[default]
    Increment,
    AluOutput,
}

#[derive(Default, Clone, Copy)]
pub enum AluSrcA {
    #[default]
    RegisterA,
    Mdr,
}

#[derive(Default, Clone, Copy)]
pub enum AluSrcB {
    #[default]
    RegisterB,
    Tmp,
}

#[derive(Default, Clone, Copy)]
pub enum MarSrc {
    #[default]
    Ip,
    AluOutput,
}

#[derive(Default, Copy, Clone)]
pub struct ControlSignals {
    // IR
    pub ir_write_enable: bool,

    // IP
    pub ip_src: IpSrc,
    pub ip_write_enable: bool,

    // Register File
    pub rf_read_a: data_path::GeneralPurposeRegister,
    pub rf_read_b: data_path::GeneralPurposeRegister,
    pub rf_write_dst: data_path::GeneralPurposeRegister,
    pub rf_write_pair_mode: bool,
    pub rf_write_enable: bool,

    // ALU
    pub alu_operation: data_path::AluOperation,
    pub alu_src_a: AluSrcA,
    pub alu_src_b: AluSrcB,
    pub alu_flags_write_enable: bool,

    // Memory
    pub mem_read: bool,
    pub mem_write: bool,
    pub mar_src: MarSrc,
    pub mar_write_enable: bool,
    pub mdr_write_enable: bool,
    pub tmp_write_enable: bool,
}

enum CUState {
    InstructionFetch,
    Exec,
}

pub struct ControlUnit {
    state: CUState,
    tick_counter: usize,
    // Registers
    interruption_flag: Register<bool>,
}

impl ControlUnit {
    pub fn new() -> Self {
        Self {
            state: CUState::InstructionFetch,
            tick_counter: 0,
            interruption_flag: Register::new(),
        }
    }

    pub fn tick(&mut self) -> ControlSignals {
        let mut cu_signals = ControlSignals::default();

        match self.state {
            CUState::InstructionFetch => {}
            CUState::Exec => {}
        }

        self.interruption_flag.tick();
        cu_signals
    }
}
