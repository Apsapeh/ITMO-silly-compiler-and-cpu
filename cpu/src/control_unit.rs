use crate::data_path;
use crate::data_path::AluOperation;
use crate::general::*;

#[derive(Default, Clone, Copy, Debug)]
pub enum IpSrc {
    #[default]
    Increment,
    AluOutput,
}

#[derive(Default, Clone, Copy, Debug)]
pub enum AluSrcA {
    #[default]
    RegisterA,
    Mdr,
    Ip,
}

#[derive(Default, Clone, Copy, Debug)]
pub enum AluSrcB {
    #[default]
    RegisterB,
    Mdr,
    Tmp,
}

#[derive(Default, Clone, Copy, Debug)]
pub enum MarSrc {
    #[default]
    Ip,
    IpInc,
    AluOutput,
    IoIrq,
}

#[derive(Default, Copy, Clone, Debug)]
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

#[derive(Debug)]
enum CUState {
    InstructionFetch,
    InstructionAnalyze,
    SecondWordFetch,
    ThirdWordFetch,
    Exec,
    Interruption,
}

pub struct ControlUnit {
    state: CUState,
    step_counter: u8,
    is_halted: bool,
    // Registers
    interruption_flag: Register<bool>,
    interruption_block: Register<bool>,

    debug: bool,
}

impl ControlUnit {
    pub fn new(debug: bool) -> Self {
        Self {
            state: CUState::InstructionFetch,
            step_counter: 0,
            interruption_flag: Register::new(),
            interruption_block: Register::new(),
            is_halted: false,
            debug,
        }
    }

    pub fn tick(&mut self, dp: &data_path::DataPath) -> ControlSignals {
        if self.debug {
            println!("\tState: {:?} ({})", self.state, self.step_counter);

            let ir = dp.get_instruction_register();
            let di = DecodedInstruction::from_raw(ir);

            println!(
                "\t{:<6}  {:<10}  {:<2}  {:<2}",
                di.opcode,
                di.mode,
                format!("{:?}", di.rd),
                format!("{:?}", di.rs),
            );

            println!("\tIR:  0x{:<4x}", ir,);

            println!(
                "\tIF:  {:<5}    IB:  {:<5}    IRQ: {:<5}",
                self.interruption_flag.get(),
                self.interruption_block.get(),
                dp.get_io_irq(),
            );
        }

        let (cu_signals, new_state) = match self.state {
            CUState::InstructionFetch => match self.step_counter {
                0 => {
                    self.step_counter += 1;
                    (
                        ControlSignals {
                            mar_src: MarSrc::Ip,
                            mar_write_enable: true,
                            ..Default::default()
                        },
                        CUState::InstructionFetch,
                    )
                }
                1 => {
                    self.step_counter = 0;
                    (
                        ControlSignals {
                            ir_write_enable: true,
                            ip_src: IpSrc::Increment,
                            ip_write_enable: true,
                            mem_read: true,
                            ..Default::default()
                        },
                        CUState::InstructionAnalyze,
                    )
                }
                _ => unreachable!(),
            },

            CUState::InstructionAnalyze => {
                let di = DecodedInstruction::from_raw(dp.get_instruction_register());
                let next_step = match di.get_len() {
                    1 => CUState::Exec,
                    2 | 3 => CUState::SecondWordFetch,
                    _ => unreachable!(),
                };

                (
                    ControlSignals {
                        mar_src: MarSrc::Ip,
                        mar_write_enable: true,
                        ..Default::default()
                    },
                    next_step,
                )
            }

            CUState::SecondWordFetch => {
                let di = DecodedInstruction::from_raw(dp.get_instruction_register());
                let next_step = if di.get_len() == 3 {
                    CUState::ThirdWordFetch
                } else {
                    CUState::Exec
                };

                (
                    ControlSignals {
                        ip_src: IpSrc::Increment,
                        ip_write_enable: true,
                        mdr_write_enable: true,
                        mar_src: MarSrc::IpInc,
                        mar_write_enable: true,
                        mem_read: true,
                        ..Default::default()
                    },
                    next_step,
                )
            }

            CUState::ThirdWordFetch => (
                ControlSignals {
                    ip_src: IpSrc::Increment,
                    ip_write_enable: true,
                    mdr_write_enable: true,
                    mar_src: MarSrc::IpInc,
                    tmp_write_enable: true,
                    mar_write_enable: true,
                    mem_read: true,
                    ..Default::default()
                },
                CUState::Exec,
            ),

            CUState::Exec => {
                let (cu_signals, is_done) = self.tick_exec_state(dp);
                if is_done {
                    self.step_counter = 0;

                    if dp.get_io_irq()
                        && self.interruption_flag.get()
                        && !self.interruption_block.get()
                    {
                        self.interruption_block.set(true);
                        self.interruption_block.set_write(true);
                        (cu_signals, CUState::Interruption)
                    } else {
                        (cu_signals, CUState::InstructionFetch)
                    }
                } else {
                    self.step_counter += 1;
                    (cu_signals, CUState::Exec)
                }
            }

            CUState::Interruption => {
                let (cu_signals, is_done) = match self.step_counter {
                    // Push flags onto stack
                    0 => (Self::op_dec_sp(), false),
                    1 => (Self::op_sp_to_mar(), false),
                    2 => (
                        ControlSignals {
                            alu_operation: AluOperation::PassFlags,
                            mem_write: true,
                            ..Default::default()
                        },
                        false,
                    ),
                    // Push IP onto stack
                    3 => (Self::op_dec_sp(), false),
                    4 => (Self::op_sp_to_mar(), false),
                    5 => (
                        ControlSignals {
                            alu_operation: AluOperation::PassLeft,
                            alu_src_a: AluSrcA::Ip,
                            mem_write: true,
                            ..Default::default()
                        },
                        false,
                    ),
                    6 => (
                        ControlSignals {
                            mar_src: MarSrc::IoIrq,
                            mar_write_enable: true,
                            ..Default::default()
                        },
                        false,
                    ),
                    7 => (
                        ControlSignals {
                            mem_read: true,
                            mdr_write_enable: true,
                            ..Default::default()
                        },
                        false,
                    ),
                    8 => (
                        ControlSignals {
                            ip_src: IpSrc::AluOutput,
                            ip_write_enable: true,
                            mar_src: MarSrc::AluOutput,
                            mar_write_enable: true,
                            alu_src_a: AluSrcA::Mdr,
                            alu_operation: AluOperation::PassLeft,
                            ..Default::default()
                        },
                        true,
                    ),
                    _ => unreachable!(),
                };

                if !is_done {
                    self.step_counter += 1;
                    (cu_signals, CUState::Interruption)
                } else {
                    self.step_counter = 0;
                    (cu_signals, CUState::InstructionFetch)
                }
            }
        };

        self.interruption_flag.tick();
        self.interruption_block.tick();
        self.state = new_state;
        cu_signals
    }

    pub fn get_is_halted(&self) -> bool {
        self.is_halted
    }

    fn tick_exec_state(&mut self, dp: &data_path::DataPath) -> (ControlSignals, bool) {
        let ir = dp.get_instruction_register();
        let di = DecodedInstruction::from_raw(ir);

        match (di.opcode, di.mode) {
            (isa::Opcode::Nop, isa::Mode::RegToReg) => (
                ControlSignals {
                    ..Default::default()
                },
                true,
            ),

            (isa::Opcode::Hlt, isa::Mode::RegToReg) => {
                self.is_halted = true;
                (
                    ControlSignals {
                        ..Default::default()
                    },
                    true,
                )
            }

            (isa::Opcode::Mov, _) => Self::op_binary_alu_any(
                di,
                self.step_counter,
                AluOperation::PassRight,
                false,
                true,
                false,
                false,
            ),

            (isa::Opcode::Add, _) => Self::op_binary_alu_any(
                di,
                self.step_counter,
                AluOperation::Add,
                false,
                false,
                false,
                true,
            ),

            (isa::Opcode::AddC, _) => Self::op_binary_alu_any(
                di,
                self.step_counter,
                AluOperation::AddC,
                false,
                false,
                false,
                true,
            ),

            (isa::Opcode::Sub, _) => Self::op_binary_alu_any(
                di,
                self.step_counter,
                AluOperation::Sub,
                false,
                false,
                false,
                true,
            ),

            (isa::Opcode::SubC, _) => Self::op_binary_alu_any(
                di,
                self.step_counter,
                AluOperation::SubC,
                false,
                false,
                false,
                true,
            ),

            (isa::Opcode::And, _) => Self::op_binary_alu_any(
                di,
                self.step_counter,
                AluOperation::And,
                false,
                false,
                false,
                true,
            ),

            (isa::Opcode::Or, _) => Self::op_binary_alu_any(
                di,
                self.step_counter,
                AluOperation::Or,
                false,
                false,
                false,
                true,
            ),

            (isa::Opcode::Xor, _) => Self::op_binary_alu_any(
                di,
                self.step_counter,
                AluOperation::Xor,
                false,
                false,
                false,
                true,
            ),

            (isa::Opcode::Mul, _) => Self::op_binary_alu_any(
                di,
                self.step_counter,
                AluOperation::Mul,
                false,
                false,
                true,
                true,
            ),

            (isa::Opcode::Div, _) => Self::op_binary_alu_any(
                di,
                self.step_counter,
                AluOperation::Div,
                false,
                false,
                true,
                true,
            ),

            (isa::Opcode::Cmp, _) => Self::op_binary_alu_any(
                di,
                self.step_counter,
                AluOperation::Sub,
                true,
                false,
                false,
                true,
            ),

            (isa::Opcode::Test, _) => Self::op_binary_alu_any(
                di,
                self.step_counter,
                AluOperation::And,
                true,
                false,
                false,
                true,
            ),

            (isa::Opcode::Inc, isa::Mode::RegToReg) => Self::op_unary_rtr(di, AluOperation::Inc),
            (isa::Opcode::Dec, isa::Mode::RegToReg) => Self::op_unary_rtr(di, AluOperation::Dec),
            (isa::Opcode::Neg, isa::Mode::RegToReg) => Self::op_unary_rtr(di, AluOperation::Neg),
            (isa::Opcode::Not, isa::Mode::RegToReg) => Self::op_unary_rtr(di, AluOperation::Not),

            (isa::Opcode::Shl, isa::Mode::ImmToReg) => Self::op_shift_itr(di, AluOperation::ShiftL),
            (isa::Opcode::Shr, isa::Mode::ImmToReg) => Self::op_shift_itr(di, AluOperation::ShiftR),

            (isa::Opcode::Jmp, isa::Mode::ImmToReg) => (Self::op_jmp_itr(true), true),
            (isa::Opcode::Je, isa::Mode::ImmToReg) => (Self::op_jmp_itr(dp.get_zero_flag()), true),
            (isa::Opcode::Jne, isa::Mode::ImmToReg) => {
                (Self::op_jmp_itr(!dp.get_zero_flag()), true)
            }
            (isa::Opcode::Jns, isa::Mode::ImmToReg) => {
                (Self::op_jmp_itr(dp.get_negative_flag()), true)
            }
            (isa::Opcode::Jnc, isa::Mode::ImmToReg) => {
                (Self::op_jmp_itr(!dp.get_negative_flag()), true)
            }
            (isa::Opcode::Jcs, isa::Mode::ImmToReg) => {
                (Self::op_jmp_itr(dp.get_carry_flag()), true)
            }
            (isa::Opcode::Jcc, isa::Mode::ImmToReg) => {
                (Self::op_jmp_itr(!dp.get_carry_flag()), true)
            }
            (isa::Opcode::Jos, isa::Mode::ImmToReg) => {
                (Self::op_jmp_itr(dp.get_overflow_flag()), true)
            }
            (isa::Opcode::Joc, isa::Mode::ImmToReg) => {
                (Self::op_jmp_itr(!dp.get_overflow_flag()), true)
            }
            (isa::Opcode::Jl, isa::Mode::ImmToReg) => (
                Self::op_jmp_itr(dp.get_negative_flag() != dp.get_overflow_flag()),
                true,
            ),
            (isa::Opcode::Jle, isa::Mode::ImmToReg) => (
                Self::op_jmp_itr(
                    dp.get_zero_flag() || (dp.get_negative_flag() != dp.get_overflow_flag()),
                ),
                true,
            ),
            (isa::Opcode::Jg, isa::Mode::ImmToReg) => (
                Self::op_jmp_itr(
                    !dp.get_zero_flag() && (dp.get_negative_flag() == dp.get_overflow_flag()),
                ),
                true,
            ),
            (isa::Opcode::Jge, isa::Mode::ImmToReg) => (
                Self::op_jmp_itr(dp.get_negative_flag() == dp.get_overflow_flag()),
                true,
            ),

            (isa::Opcode::Call, isa::Mode::ImmToReg) => match self.step_counter {
                0 => (Self::op_dec_sp(), false),
                1 => (Self::op_sp_to_mar(), false),
                2 => (
                    ControlSignals {
                        alu_operation: AluOperation::PassLeft,
                        alu_src_a: AluSrcA::Ip,
                        mem_write: true,
                        ..Default::default()
                    },
                    false,
                ),
                3 => (
                    ControlSignals {
                        alu_operation: AluOperation::PassLeft,
                        alu_src_a: AluSrcA::Mdr,
                        ip_src: IpSrc::AluOutput,
                        ip_write_enable: true,
                        ..Default::default()
                    },
                    true,
                ),
                _ => unreachable!(),
            },

            (isa::Opcode::Ret, isa::Mode::RegToReg) => match self.step_counter {
                0 => (Self::op_sp_to_mar(), false),
                1 => (Self::op_inc_sp(), false),
                2 => (
                    ControlSignals {
                        mem_read: true,
                        mdr_write_enable: true,
                        ..Default::default()
                    },
                    false,
                ),
                3 => (
                    ControlSignals {
                        alu_operation: AluOperation::PassLeft,
                        alu_src_a: AluSrcA::Mdr,
                        ip_src: IpSrc::AluOutput,
                        ip_write_enable: true,
                        ..Default::default()
                    },
                    true,
                ),
                _ => unreachable!(),
            },

            (isa::Opcode::Ret, isa::Mode::ImmToReg) => match self.step_counter {
                0 => (Self::op_sp_to_mar(), false),
                1 => (Self::op_inc_sp(), false),
                2 => (
                    // Mem[SP] -> MDR
                    // SP + #n (MDR) -> SP  ; Pop args
                    ControlSignals {
                        rf_read_a: data_path::GeneralPurposeRegister::SP,
                        alu_src_a: AluSrcA::RegisterA,
                        alu_src_b: AluSrcB::Mdr,
                        alu_operation: AluOperation::Add,
                        rf_write_dst: data_path::GeneralPurposeRegister::SP,
                        rf_write_enable: true,
                        mem_read: true,
                        mdr_write_enable: true,
                        ..Default::default()
                    },
                    false,
                ),
                3 => (
                    ControlSignals {
                        alu_operation: AluOperation::PassLeft,
                        alu_src_a: AluSrcA::Mdr,
                        ip_src: IpSrc::AluOutput,
                        ip_write_enable: true,
                        ..Default::default()
                    },
                    true,
                ),
                _ => unreachable!(),
            },

            (isa::Opcode::Push, isa::Mode::RegToReg) => match self.step_counter {
                0 => (Self::op_dec_sp(), false),
                1 => (Self::op_sp_to_mar(), false),
                2 => (
                    ControlSignals {
                        rf_read_a: di.rs,
                        alu_src_a: AluSrcA::RegisterA,
                        alu_operation: AluOperation::PassLeft,
                        mem_write: true,
                        ..Default::default()
                    },
                    true,
                ),
                _ => unreachable!(),
            },

            (isa::Opcode::Pop, isa::Mode::RegToReg) => match self.step_counter {
                0 => (Self::op_sp_to_mar(), false),
                1 => (Self::op_inc_sp(), false),
                2 => (
                    ControlSignals {
                        mem_read: true,
                        mdr_write_enable: true,
                        ..Default::default()
                    },
                    false,
                ),
                3 => (
                    ControlSignals {
                        alu_operation: AluOperation::PassLeft,
                        alu_src_a: AluSrcA::Mdr,
                        rf_write_dst: di.rd,
                        rf_write_enable: true,
                        ..Default::default()
                    },
                    true,
                ),
                _ => unreachable!(),
            },

            (isa::Opcode::Enter, isa::Mode::ImmToReg) => match self.step_counter {
                0 => (Self::op_dec_sp(), false),
                1 => (Self::op_sp_to_mar(), false),
                2 => (
                    ControlSignals {
                        rf_read_a: data_path::GeneralPurposeRegister::BP,
                        alu_src_a: AluSrcA::RegisterA,
                        alu_operation: AluOperation::PassLeft,
                        mem_write: true,
                        ..Default::default()
                    },
                    false,
                ),
                3 => (
                    ControlSignals {
                        rf_read_a: data_path::GeneralPurposeRegister::SP,
                        alu_src_a: AluSrcA::RegisterA,
                        alu_operation: AluOperation::PassLeft,
                        rf_write_dst: data_path::GeneralPurposeRegister::BP,
                        rf_write_enable: true,
                        ..Default::default()
                    },
                    false,
                ),
                4 => (
                    ControlSignals {
                        rf_read_a: data_path::GeneralPurposeRegister::SP,
                        alu_src_a: AluSrcA::RegisterA,
                        alu_src_b: AluSrcB::Mdr,
                        alu_operation: AluOperation::Sub,
                        rf_write_dst: data_path::GeneralPurposeRegister::SP,
                        rf_write_enable: true,
                        ..Default::default()
                    },
                    true,
                ),
                _ => unreachable!(),
            },

            (isa::Opcode::Leave, isa::Mode::RegToReg) => match self.step_counter {
                0 => (
                    ControlSignals {
                        rf_read_a: data_path::GeneralPurposeRegister::BP,
                        alu_src_a: AluSrcA::RegisterA,
                        alu_operation: AluOperation::PassLeft,
                        rf_write_dst: data_path::GeneralPurposeRegister::SP,
                        rf_write_enable: true,
                        ..Default::default()
                    },
                    false,
                ),
                1 => (Self::op_sp_to_mar(), false),
                2 => (Self::op_inc_sp(), false),
                3 => (
                    ControlSignals {
                        mem_read: true,
                        mdr_write_enable: true,
                        ..Default::default()
                    },
                    false,
                ),
                4 => (
                    ControlSignals {
                        alu_operation: AluOperation::PassLeft,
                        alu_src_a: AluSrcA::Mdr,
                        rf_write_dst: data_path::GeneralPurposeRegister::BP,
                        rf_write_enable: true,
                        ..Default::default()
                    },
                    true,
                ),
                _ => unreachable!(),
            },

            (isa::Opcode::Iret, isa::Mode::RegToReg) => match self.step_counter {
                // Pop IP
                0 => (Self::op_sp_to_mar(), false),
                1 => {
                    let mut cu = Self::op_inc_sp();
                    cu.mem_read = true;
                    cu.mdr_write_enable = true;
                    (cu, false)
                }
                2 => (
                    ControlSignals {
                        alu_operation: AluOperation::PassLeft,
                        alu_src_a: AluSrcA::Mdr,
                        ip_src: IpSrc::AluOutput,
                        ip_write_enable: true,
                        mar_src: MarSrc::AluOutput,
                        mar_write_enable: true,
                        ..Default::default()
                    },
                    false,
                ),
                // Pop flags
                3 => (Self::op_sp_to_mar(), false),
                4 => {
                    let mut cu = Self::op_inc_sp();
                    cu.mem_read = true;
                    cu.mdr_write_enable = true;
                    (cu, false)
                }
                5 => {
                    self.interruption_block.set(false);
                    self.interruption_block.set_write(true);

                    (
                        ControlSignals {
                            alu_src_a: AluSrcA::Mdr,
                            alu_operation: AluOperation::LoadFlagsLeft,
                            alu_flags_write_enable: true,
                            ..Default::default()
                        },
                        true,
                    )
                }
                _ => unreachable!(),
            },

            (isa::Opcode::Sti, isa::Mode::RegToReg) => {
                (Self::op_set_if(true, &mut self.interruption_flag), true)
            }

            (isa::Opcode::Cli, isa::Mode::RegToReg) => {
                (Self::op_set_if(false, &mut self.interruption_flag), true)
            }

            _ => {
                panic!("Unexpected opcode or mode!: {:#?}", di);
            }
        }
    }

    fn op_binary_alu_any(
        di: DecodedInstruction,
        mut step_counter: u8,
        alu_op: data_path::AluOperation,
        disable_mem_or_rf_write: bool,
        skip_mem_read: bool,
        pair_mode: bool,
        write_flags: bool,
    ) -> (ControlSignals, bool) {
        let mut cs = ControlSignals::default();

        // Mov hack for opt.
        if skip_mem_read
            && step_counter == 1
            && matches!(
                di.mode,
                isa::Mode::RegToMemr
                    | isa::Mode::ImmToMemr
                    | isa::Mode::RegToMema
                    | isa::Mode::ImmToMema
                    | isa::Mode::RegToMemra
                    | isa::Mode::ImmToMemra
            )
        {
            step_counter += 1;
        }

        let is_done = match di.mode {
            isa::Mode::RegToReg => {
                // Rd _ Rs -> Rd
                cs.rf_read_a = di.rd;
                cs.rf_read_b = di.rs;
                cs.alu_src_a = AluSrcA::RegisterA;
                cs.alu_src_b = AluSrcB::RegisterB;
                cs.rf_write_dst = di.rd;
                cs.rf_write_enable = true;
                true
            }

            isa::Mode::ImmToReg => {
                // Rd _ #addr -> Rd
                cs.rf_read_a = di.rd;
                cs.alu_src_a = AluSrcA::RegisterA;
                cs.alu_src_b = AluSrcB::Mdr;
                cs.rf_write_dst = di.rd;
                cs.rf_write_enable = true;
                true
            }

            isa::Mode::MemrToReg => match step_counter {
                0 => {
                    // Rs -> Mar
                    cs.rf_read_a = di.rs;
                    cs.alu_src_a = AluSrcA::RegisterA;
                    cs.alu_operation = data_path::AluOperation::PassLeft;
                    cs.mar_src = MarSrc::AluOutput;
                    cs.mar_write_enable = true;
                    false
                }
                1 => {
                    // mem[Mar] -> Mdr
                    cs.mem_read = true;
                    cs.mdr_write_enable = true;
                    false
                }
                2 => {
                    // Rd _ Mdr -> Rd
                    cs.rf_read_a = di.rd;
                    cs.alu_src_a = AluSrcA::RegisterA;
                    cs.alu_src_b = AluSrcB::Mdr;
                    cs.rf_write_dst = di.rd;
                    cs.rf_write_enable = true;
                    true
                }
                _ => unreachable!(),
            },

            isa::Mode::MemaToReg => match step_counter {
                0 => {
                    // #addr -> Mar
                    cs.alu_src_a = AluSrcA::Mdr;
                    cs.alu_operation = data_path::AluOperation::PassLeft;
                    cs.mar_src = MarSrc::AluOutput;
                    cs.mar_write_enable = true;
                    false
                }
                1 => {
                    // mem[Mar] -> Mdr
                    cs.mem_read = true;
                    cs.mdr_write_enable = true;
                    false
                }
                2 => {
                    // Rd _ Mdr -> Rd
                    cs.rf_read_a = di.rd;
                    cs.alu_src_a = AluSrcA::RegisterA;
                    cs.alu_src_b = AluSrcB::Mdr;
                    cs.rf_write_dst = di.rd;
                    cs.rf_write_enable = true;
                    true
                }
                _ => unreachable!(),
            },

            isa::Mode::MemraToReg => match step_counter {
                0 => {
                    // Rs + #addr -> Mar
                    cs.rf_read_a = di.rs;
                    cs.alu_src_a = AluSrcA::RegisterA;
                    cs.alu_src_b = AluSrcB::Mdr;
                    cs.alu_operation = data_path::AluOperation::Add;
                    cs.mar_src = MarSrc::AluOutput;
                    cs.mar_write_enable = true;
                    false
                }
                1 => {
                    // mem[Mar] -> Mdr
                    cs.mem_read = true;
                    cs.mdr_write_enable = true;
                    false
                }
                2 => {
                    // Rd _ Mdr -> Rd
                    cs.rf_read_a = di.rd;
                    cs.alu_src_a = AluSrcA::RegisterA;
                    cs.alu_src_b = AluSrcB::Mdr;
                    cs.rf_write_dst = di.rd;
                    cs.rf_write_enable = true;
                    true
                }
                _ => unreachable!(),
            },

            isa::Mode::RegToMemr => match step_counter {
                0 => {
                    // Rd -> Mar
                    cs.rf_read_a = di.rd;
                    cs.alu_src_a = AluSrcA::RegisterA;
                    cs.alu_operation = data_path::AluOperation::PassLeft;
                    cs.mar_src = MarSrc::AluOutput;
                    cs.mar_write_enable = true;
                    false
                }

                1 => {
                    // mem[Mar] -> Mdr
                    cs.mem_read = true;
                    cs.mdr_write_enable = true;
                    false
                }

                2 => {
                    // Mdr _ Rs -> mem[Mar]
                    cs.rf_read_b = di.rs;
                    cs.alu_src_a = AluSrcA::Mdr;
                    cs.alu_src_b = AluSrcB::RegisterB;
                    cs.mem_write = true;
                    true
                }

                _ => unreachable!(),
            },

            isa::Mode::ImmToMemr => match step_counter {
                0 => {
                    // Mdr -> Tmp; Rd -> Mar
                    cs.rf_read_a = di.rd;
                    cs.alu_src_a = AluSrcA::RegisterA;
                    cs.alu_operation = data_path::AluOperation::PassLeft;
                    cs.mar_src = MarSrc::AluOutput;
                    cs.mar_write_enable = true;
                    cs.tmp_write_enable = true;
                    false
                }

                1 => {
                    // mem[Mar] -> Mdr
                    cs.mem_read = true;
                    cs.mdr_write_enable = true;
                    false
                }

                2 => {
                    // Mdr _ Tmp -> mem[Mar]
                    cs.alu_src_a = AluSrcA::Mdr;
                    cs.alu_src_b = AluSrcB::Tmp;
                    cs.mem_write = true;
                    true
                }

                _ => unreachable!(),
            },

            isa::Mode::RegToMema => match step_counter {
                0 => {
                    // #addr -> Mar
                    cs.alu_src_a = AluSrcA::Mdr;
                    cs.alu_operation = data_path::AluOperation::PassLeft;
                    cs.mar_src = MarSrc::AluOutput;
                    cs.mar_write_enable = true;
                    false
                }

                1 => {
                    // mem[Mar] -> Mdr
                    cs.mem_read = true;
                    cs.mdr_write_enable = true;
                    false
                }

                2 => {
                    // Mdr _ Rs -> mem[Mar]
                    cs.rf_read_b = di.rs;
                    cs.alu_src_a = AluSrcA::Mdr;
                    cs.alu_src_b = AluSrcB::RegisterB;
                    cs.mem_write = true;
                    true
                }

                _ => unreachable!(),
            },

            isa::Mode::ImmToMema => match step_counter {
                0 => {
                    // #addr (Tmp) -> Mar; Mdr -> Tmp
                    cs.alu_src_b = AluSrcB::Tmp;
                    cs.alu_operation = data_path::AluOperation::PassRight;
                    cs.mar_src = MarSrc::AluOutput;
                    cs.mar_write_enable = true;
                    cs.tmp_write_enable = true;
                    false
                }

                1 => {
                    // mem[Mar] -> Mdr
                    cs.mem_read = true;
                    cs.mdr_write_enable = true;
                    false
                }

                2 => {
                    // Mdr _ #imm -> mem[Mar]
                    cs.alu_src_a = AluSrcA::Mdr;
                    cs.alu_src_b = AluSrcB::Tmp;
                    cs.mem_write = true;
                    true
                }

                _ => unreachable!(),
            },

            isa::Mode::RegToMemra => match step_counter {
                0 => {
                    // Rd + #addr (Mdr) -> Mar
                    cs.rf_read_a = di.rd;
                    cs.alu_src_a = AluSrcA::RegisterA;
                    cs.alu_src_b = AluSrcB::Mdr;
                    cs.alu_operation = data_path::AluOperation::Add;
                    cs.mar_src = MarSrc::AluOutput;
                    cs.mar_write_enable = true;
                    cs.tmp_write_enable = true;
                    false
                }

                1 => {
                    // mem[Mar] -> Mdr
                    cs.mem_read = true;
                    cs.mdr_write_enable = true;
                    false
                }

                2 => {
                    // Mdr _ Rs -> mem[Mar]
                    cs.rf_read_b = di.rs;
                    cs.alu_src_a = AluSrcA::Mdr;
                    cs.alu_src_b = AluSrcB::RegisterB;
                    cs.mem_write = true;
                    true
                }

                _ => unreachable!(),
            },

            isa::Mode::ImmToMemra => match step_counter {
                0 => {
                    // Rd + #addr (Tmp) -> Mar; Mdr -> Tmp
                    cs.rf_read_a = di.rd;
                    cs.alu_src_a = AluSrcA::RegisterA;
                    cs.alu_src_b = AluSrcB::Tmp;
                    cs.alu_operation = data_path::AluOperation::Add;
                    cs.mar_src = MarSrc::AluOutput;
                    cs.mar_write_enable = true;
                    cs.tmp_write_enable = true;
                    false
                }

                1 => {
                    // mem[Mar] -> Mdr
                    cs.mem_read = true;
                    cs.mdr_write_enable = true;
                    false
                }

                2 => {
                    // Mdr _ #imm -> mem[Mar]
                    cs.alu_src_a = AluSrcA::Mdr;
                    cs.alu_src_b = AluSrcB::Tmp;
                    cs.mem_write = true;
                    true
                }

                _ => unreachable!(),
            },
        };

        if is_done {
            cs.alu_operation = alu_op;
            cs.mar_src = MarSrc::Ip;
            cs.mar_write_enable = true;

            // Tmp and Cmp hack
            if disable_mem_or_rf_write {
                cs.mem_write = false;
                cs.rf_write_enable = false;
            }

            // Mul and Div hack
            if pair_mode {
                cs.rf_write_pair_mode = true;
            }

            if write_flags {
                cs.alu_flags_write_enable = true;
            }
        }

        (cs, is_done)
    }

    fn op_unary_rtr(
        di: DecodedInstruction,
        alu_op: data_path::AluOperation,
    ) -> (ControlSignals, bool) {
        (
            ControlSignals {
                rf_read_a: di.rd,
                rf_read_b: di.rs,
                rf_write_dst: di.rd,
                rf_write_enable: true,
                alu_operation: alu_op,
                ..Default::default()
            },
            true,
        )
    }

    fn op_shift_itr(
        di: DecodedInstruction,
        alu_op: data_path::AluOperation,
    ) -> (ControlSignals, bool) {
        (
            ControlSignals {
                rf_read_a: di.rd,
                alu_src_a: AluSrcA::RegisterA,
                alu_src_b: AluSrcB::Mdr,
                alu_operation: alu_op,
                rf_write_dst: di.rd,
                rf_write_enable: true,
                ..Default::default()
            },
            true,
        )
    }

    fn op_jmp_itr(cond: bool) -> ControlSignals {
        if cond {
            ControlSignals {
                ip_write_enable: true,
                ip_src: IpSrc::AluOutput,
                alu_src_a: AluSrcA::Mdr,
                mar_src: MarSrc::AluOutput,
                mar_write_enable: true,
                alu_operation: data_path::AluOperation::PassLeft,
                ..Default::default()
            }
        } else {
            ControlSignals {
                ..Default::default()
            }
        }
    }

    fn op_set_if(state: bool, reg: &mut Register<bool>) -> ControlSignals {
        reg.set(state);
        reg.set_write(true);
        ControlSignals {
            ..Default::default()
        }
    }

    fn op_dec_sp() -> ControlSignals {
        ControlSignals {
            rf_read_a: data_path::GeneralPurposeRegister::SP,
            rf_write_dst: data_path::GeneralPurposeRegister::SP,
            rf_write_enable: true,
            alu_src_a: AluSrcA::RegisterA,
            alu_operation: AluOperation::Dec,

            ..Default::default()
        }
    }

    fn op_inc_sp() -> ControlSignals {
        ControlSignals {
            rf_read_a: data_path::GeneralPurposeRegister::SP,
            rf_write_dst: data_path::GeneralPurposeRegister::SP,
            rf_write_enable: true,
            alu_src_a: AluSrcA::RegisterA,
            alu_operation: AluOperation::Inc,

            ..Default::default()
        }
    }

    fn op_sp_to_mar() -> ControlSignals {
        ControlSignals {
            rf_read_a: data_path::GeneralPurposeRegister::SP,
            mar_src: MarSrc::AluOutput,
            mar_write_enable: true,
            alu_src_a: AluSrcA::RegisterA,
            alu_operation: AluOperation::PassLeft,
            ..Default::default()
        }
    }
}

#[derive(Debug)]
struct DecodedInstruction {
    opcode: isa::Opcode,
    mode: isa::Mode,
    rd: data_path::GeneralPurposeRegister,
    rs: data_path::GeneralPurposeRegister,
}

impl DecodedInstruction {
    pub fn from_raw(raw: u16) -> Self {
        Self {
            opcode: isa::Opcode::from_raw((raw >> 10) as u8)
                .unwrap_or_else(|| panic!("Unexpected opcode {} ({})!", raw >> 10, raw)),
            mode: isa::Mode::from_raw(((raw >> 6) & 0b1111) as u8).expect("Unexpected mode!"),
            rd: Self::raw_isa_reg_to_alu_reg(((raw >> 3) & 0b111) as u8),
            rs: Self::raw_isa_reg_to_alu_reg(((raw) & 0b111) as u8),
        }
    }

    pub fn get_len(&self) -> u8 {
        use isa::Mode::*;
        match self.mode {
            RegToReg | MemrToReg | RegToMemr => 1,
            ImmToReg | MemaToReg | MemraToReg | ImmToMemr | RegToMema | RegToMemra => 2,
            ImmToMema | ImmToMemra => 3,
        }
    }

    fn raw_isa_reg_to_alu_reg(reg: u8) -> data_path::GeneralPurposeRegister {
        let isa_reg = isa::Register::from_raw(reg).expect("Unexpected register!");
        match isa_reg {
            isa::Register::AL => data_path::GeneralPurposeRegister::AL,
            isa::Register::AH => data_path::GeneralPurposeRegister::AH,
            isa::Register::BL => data_path::GeneralPurposeRegister::BL,
            isa::Register::BH => data_path::GeneralPurposeRegister::BH,
            isa::Register::CL => data_path::GeneralPurposeRegister::CL,
            isa::Register::CH => data_path::GeneralPurposeRegister::CH,
            isa::Register::SP => data_path::GeneralPurposeRegister::SP,
            isa::Register::BP => data_path::GeneralPurposeRegister::BP,
        }
    }
}
