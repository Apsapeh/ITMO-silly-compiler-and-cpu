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
}

impl ControlUnit {
    pub fn new() -> Self {
        Self {
            state: CUState::InstructionFetch,
            step_counter: 0,
            interruption_flag: Register::new(),
            interruption_block: Register::new(),
            is_halted: false,
        }
    }

    pub fn tick(&mut self, dp: &data_path::DataPath) -> ControlSignals {
        println!("State: {:?}", self.state);

        let (cu_signals, new_state) = match self.state {
            CUState::InstructionFetch => (
                ControlSignals {
                    ir_write_enable: true,
                    ip_src: IpSrc::Increment,
                    ip_write_enable: true,
                    mem_read: true,
                    ..Default::default()
                },
                CUState::InstructionAnalyze,
            ),

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
                    mar_src: MarSrc::Ip,
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
                println!("Int step: {}", self.step_counter);
                let (cu_signals, is_done) = match self.step_counter {
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

        println!("IR: {}", ir);
        println!("DI: {:#?}", di);

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
            ),

            (isa::Opcode::Add, _) => Self::op_binary_alu_any(
                di,
                self.step_counter,
                AluOperation::Add,
                false,
                false,
                false,
            ),

            (isa::Opcode::AddC, _) => Self::op_binary_alu_any(
                di,
                self.step_counter,
                AluOperation::AddC,
                false,
                false,
                false,
            ),

            (isa::Opcode::Sub, _) => Self::op_binary_alu_any(
                di,
                self.step_counter,
                AluOperation::Sub,
                false,
                false,
                false,
            ),

            (isa::Opcode::SubC, _) => Self::op_binary_alu_any(
                di,
                self.step_counter,
                AluOperation::SubC,
                false,
                false,
                false,
            ),

            (isa::Opcode::And, _) => Self::op_binary_alu_any(
                di,
                self.step_counter,
                AluOperation::And,
                false,
                false,
                false,
            ),

            (isa::Opcode::Or, _) => Self::op_binary_alu_any(
                di,
                self.step_counter,
                AluOperation::Or,
                false,
                false,
                false,
            ),

            (isa::Opcode::Xor, _) => Self::op_binary_alu_any(
                di,
                self.step_counter,
                AluOperation::Xor,
                false,
                false,
                false,
            ),

            (isa::Opcode::Mul, _) => Self::op_binary_alu_any(
                di,
                self.step_counter,
                AluOperation::Mul,
                false,
                false,
                true,
            ),

            (isa::Opcode::Div, _) => Self::op_binary_alu_any(
                di,
                self.step_counter,
                AluOperation::Div,
                false,
                false,
                true,
            ),

            (isa::Opcode::Cmp, _) => Self::op_binary_alu_any(
                di,
                self.step_counter,
                AluOperation::Cmp,
                true,
                false,
                false,
            ),

            (isa::Opcode::Test, _) => Self::op_binary_alu_any(
                di,
                self.step_counter,
                AluOperation::Test,
                true,
                false,
                false,
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
                    dp.get_zero_flag() || dp.get_negative_flag() != dp.get_overflow_flag(),
                ),
                true,
            ),
            (isa::Opcode::Jg, isa::Mode::ImmToReg) => (
                Self::op_jmp_itr(
                    !dp.get_zero_flag() || dp.get_negative_flag() == dp.get_overflow_flag(),
                ),
                true,
            ),
            (isa::Opcode::Jge, isa::Mode::ImmToReg) => (
                Self::op_jmp_itr(dp.get_negative_flag() == dp.get_overflow_flag()),
                true,
            ),

            (isa::Opcode::Call, isa::Mode::ImmToReg) => {
                unimplemented!()
            }

            (isa::Opcode::Ret, isa::Mode::RegToReg) => {
                unimplemented!()
            }

            (isa::Opcode::Push, isa::Mode::RegToReg) => {
                unimplemented!()
            }

            (isa::Opcode::Pop, isa::Mode::RegToReg) => {
                unimplemented!()
            }

            (isa::Opcode::Enter, isa::Mode::ImmToReg) => {
                unimplemented!()
            }

            (isa::Opcode::Leave, isa::Mode::RegToReg) => {
                unimplemented!()
            }

            (isa::Opcode::Iret, isa::Mode::RegToReg) => {
                unimplemented!()
            }

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

        // Tmp and Cmp hack
        if disable_mem_or_rf_write {
            cs.mem_write = false;
            cs.rf_write_enable = false;
        }

        // Mul and Div hack
        if pair_mode {
            cs.rf_write_pair_mode = true;
        }

        if is_done {
            cs.alu_operation = alu_op;
            cs.mar_src = MarSrc::Ip;
            cs.mar_write_enable = true;
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

    fn op_sp_to_mar() -> ControlSignals {
        ControlSignals {
            rf_read_a: data_path::GeneralPurposeRegister::SP,
            mar_src: MarSrc::AluOutput,
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
            opcode: isa::Opcode::from_raw((raw >> 10) as u8).expect("Unexpected opcode!"),
            mode: isa::Mode::from_raw(((raw >> 6) & 0b1111) as u8).expect("Unexpected mode!"),
            rd: Self::raw_isa_reg_to_alu_reg(((raw >> 2) & 0b111) as u8),
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
