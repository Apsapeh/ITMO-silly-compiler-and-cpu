use crate::control_unit;
use crate::control_unit::*;
use crate::general::*;

const IO_READ: u16 = 0x80;
const IO_WRITE: u16 = 0x81;
pub const IO_IRQ_ADDR: u16 = 0x10;

#[derive(Clone, Copy, Default, Debug)]
pub enum AluOperation {
    #[default]
    PassLeft,
    PassRight,
    Add,
    AddC,
    Sub,
    SubC,
    Mul,
    Div,
    Inc,
    Dec,
    Neg,
    And,
    Or,
    Xor,
    Not,
    ShiftR,
    ShiftL,
    Cmp,
    Test,
    PassFlags,
    LoadFlagsLeft,
}

#[derive(Clone, Copy)]
enum CpuFlags {
    ZF,
    NF,
    CF,
    OF,
}

const ZF: usize = CpuFlags::ZF as usize;
const NF: usize = CpuFlags::NF as usize;
const CF: usize = CpuFlags::CF as usize;
const OF: usize = CpuFlags::OF as usize;

#[derive(Clone, Copy, Default, Debug)]
pub enum GeneralPurposeRegister {
    #[default]
    AL,
    AH,
    BL,
    BH,
    CL,
    CH,
    SP,
    BP,
}

pub struct DataPath {
    memory: [u16; 0x10000],
    io_read_buffer: u16,
    io_write_data: Vec<u16>,
    io_irq: bool,

    // Registers
    register_file: [Register<u16>; 8],
    alu_state_flags: [Register<bool>; 4],
    instruction_pointer: Register<u16>,
    instruction_register: Register<u16>,
    // Memory registers
    mar_register: Register<u16>,
    mdr_register: Register<u16>,
    tmp_register: Register<u16>,
}

impl DataPath {
    pub fn new(memory: [u16; 0x10000]) -> Self {
        Self {
            memory,
            io_read_buffer: 0,
            io_write_data: vec![],
            io_irq: false,
            register_file: Default::default(),
            alu_state_flags: Default::default(),
            instruction_pointer: Default::default(),
            instruction_register: Default::default(),
            mar_register: Default::default(),
            mdr_register: Default::default(),
            tmp_register: Default::default(),
        }
    }

    pub fn tick(&mut self, cu_signals: control_unit::ControlSignals) {
        if true {
            println!("Signals: {:#?}", cu_signals);
            println!("IP:  {}", self.instruction_pointer.get());
            println!("MAR: {}", self.mar_register.get());
            println!("MDR: {}", self.mdr_register.get());
            println!("TMP: {}", self.tmp_register.get());
            println!(
                "SP:  {}",
                self.register_file[GeneralPurposeRegister::SP as usize].get()
            );
            println!("Mem[8190]: {}", self.memory[8190]);
            println!("Mem[8191]: {}", self.memory[8191]);
        }

        let gpr_read_a = self.register_file_get_register(cu_signals.rf_read_a);
        let gpr_read_b = self.register_file_get_register(cu_signals.rf_read_b);

        // MUX LEFT
        let left_in = match cu_signals.alu_src_a {
            AluSrcA::RegisterA => gpr_read_a,
            AluSrcA::Mdr => self.mdr_register.get(),
            AluSrcA::Ip => self.instruction_pointer.get(),
        };

        // MUX RIGHT
        let right_in = match cu_signals.alu_src_b {
            AluSrcB::RegisterB => gpr_read_b,
            AluSrcB::Mdr => self.mdr_register.get(),
            AluSrcB::Tmp => self.tmp_register.get(),
        };

        let (alu_out_low, alu_out_high) =
            self.alu_compute(cu_signals.alu_operation, left_in, right_in);

        self.register_file_write(
            alu_out_low,
            alu_out_high,
            cu_signals.rf_write_dst,
            cu_signals.rf_write_enable,
            cu_signals.rf_write_pair_mode,
        );

        // MUX IP
        let new_ip = match cu_signals.ip_src {
            IpSrc::Increment => self.instruction_pointer.get() + 1,
            IpSrc::AluOutput => alu_out_low,
        };
        self.instruction_pointer.set(new_ip);

        self.instruction_pointer
            .set_write(cu_signals.ip_write_enable);

        // MUX MAR
        let new_mar = match cu_signals.mar_src {
            MarSrc::Ip => self.instruction_pointer.get(),
            MarSrc::IpInc => self.instruction_pointer.get() + 1,
            MarSrc::AluOutput => alu_out_low,
            MarSrc::IoIrq => IO_IRQ_ADDR,
        };
        self.mar_register.set(new_mar);
        self.mar_register.set_write(cu_signals.mar_write_enable);

        // Latch TMP
        self.tmp_register.set(self.mdr_register.get());
        self.tmp_register.set_write(cu_signals.tmp_write_enable);

        // Memory
        let mem_out = self.memory_access(
            self.mar_register.get(),
            alu_out_low,
            cu_signals.mem_read,
            cu_signals.mem_write,
        );

        // MDR
        self.mdr_register.set(mem_out);
        self.mdr_register.set_write(cu_signals.mdr_write_enable);

        // IR
        self.instruction_register.set(mem_out);
        self.instruction_register
            .set_write(cu_signals.ir_write_enable);

        // Tick Registers
        for reg in &mut self.register_file {
            reg.tick();
        }

        for reg in &mut self.alu_state_flags {
            reg.set_write(cu_signals.alu_flags_write_enable);
            reg.tick();
        }

        self.instruction_register.tick();
        self.instruction_pointer.tick();
        self.mar_register.tick();
        self.mdr_register.tick();
        self.tmp_register.tick();
    }

    pub fn set_io_read_buffer(&mut self, value: u16) {
        self.io_read_buffer = value;
        self.io_irq = true;
    }

    pub fn get_io_write_data(&self) -> Vec<u16> {
        self.io_write_data.clone()
    }

    pub fn get_memory(&self) -> [u16; 0x10000] {
        self.memory
    }

    // =====> CU Wires <======

    pub fn get_instruction_register(&self) -> u16 {
        self.instruction_register.get()
    }

    pub fn get_zero_flag(&self) -> bool {
        self.alu_state_flags[CpuFlags::ZF as usize].get()
    }

    pub fn get_negative_flag(&self) -> bool {
        self.alu_state_flags[CpuFlags::NF as usize].get()
    }

    pub fn get_carry_flag(&self) -> bool {
        self.alu_state_flags[CpuFlags::CF as usize].get()
    }

    pub fn get_overflow_flag(&self) -> bool {
        self.alu_state_flags[CpuFlags::OF as usize].get()
    }

    pub fn get_io_irq(&self) -> bool {
        self.io_irq
    }

    // ======> Inner functions <======

    fn register_file_get_register(&self, register: GeneralPurposeRegister) -> u16 {
        self.register_file[register as usize].get()
    }

    fn register_file_write(
        &mut self,
        low: u16,
        high: u16,
        register: GeneralPurposeRegister,
        is_write_enabled: bool,
        is_pair_mode: bool,
    ) {
        use GeneralPurposeRegister::*;

        self.register_file[AL as usize].set(low);
        self.register_file[BL as usize].set(low);
        self.register_file[CL as usize].set(low);
        self.register_file[SP as usize].set(low);
        self.register_file[BP as usize].set(low);

        // Some sugar, also can be writen as bunch of ifs
        let high_reg_to_latch = if is_pair_mode { high } else { low };
        self.register_file[AH as usize].set(high_reg_to_latch);
        self.register_file[BH as usize].set(high_reg_to_latch);
        self.register_file[CH as usize].set(high_reg_to_latch);

        // Register Write Decoder
        let to_write = match (is_pair_mode, register) {
            (true, AL | AH) => [Some(AL), Some(AH)],
            (true, BL | BH) => [Some(BL), Some(BH)],
            (true, CL | CH) => [Some(CL), Some(CH)],
            (_, reg) => [Some(reg), None],
        };

        for reg in to_write.into_iter().flatten() {
            self.register_file[reg as usize].set_write(is_write_enabled);
        }
    }

    fn alu_compute(&mut self, operation: AluOperation, left_in: u16, right_in: u16) -> (u16, u16) {
        match operation {
            AluOperation::PassLeft => (left_in, 0),

            AluOperation::PassRight => (right_in, 0),

            AluOperation::Add => {
                let (res, overflow) = left_in.overflowing_add(right_in);

                self.alu_state_flags[ZF].set(res == 0);
                self.alu_state_flags[NF].set((res as i16) < 0);
                self.alu_state_flags[CF].set(overflow);
                self.alu_state_flags[OF].set(((left_in ^ res) & (right_in ^ res) & 0x8000) != 0);

                (res, 0)
            }

            AluOperation::AddC => {
                let carry_in = if self.alu_state_flags[CF].get() { 1 } else { 0 };
                let (res1, o1) = left_in.overflowing_add(right_in);
                let (res, o2) = res1.overflowing_add(carry_in);

                self.alu_state_flags[ZF].set(res == 0);
                self.alu_state_flags[NF].set((res as i16) < 0);
                self.alu_state_flags[CF].set(o1 || o2);
                self.alu_state_flags[OF].set(((left_in ^ res) & (right_in ^ res) & 0x8000) != 0);

                (res, 0)
            }

            AluOperation::Sub => {
                let (res, borrow) = left_in.overflowing_sub(right_in);

                self.alu_state_flags[ZF].set(res == 0);
                self.alu_state_flags[NF].set((res as i16) < 0);
                self.alu_state_flags[CF].set(borrow);
                self.alu_state_flags[OF]
                    .set(((left_in ^ right_in) & (left_in ^ res) & 0x8000) != 0);

                (res, 0)
            }

            AluOperation::SubC => {
                let borrow_in = if self.alu_state_flags[CF].get() { 1 } else { 0 };
                let (res1, b1) = left_in.overflowing_sub(right_in);
                let (res, b2) = res1.overflowing_sub(borrow_in);

                self.alu_state_flags[ZF].set(res == 0);
                self.alu_state_flags[NF].set((res as i16) < 0);
                self.alu_state_flags[CF].set(b1 || b2);
                self.alu_state_flags[OF]
                    .set(((left_in ^ right_in) & (left_in ^ res) & 0x8000) != 0);

                (res, 0)
            }

            AluOperation::Mul => {
                let full_res = (left_in as u32) * (right_in as u32);
                let low = full_res as u16;
                let high = (full_res >> 16) as u16;
                let is_overflow = high != 0;

                self.alu_state_flags[CF].set(is_overflow);
                self.alu_state_flags[OF].set(is_overflow);
                self.alu_state_flags[ZF].set(low == 0);
                self.alu_state_flags[NF].set((low as i16) < 0);

                (low, high)
            }

            AluOperation::Div => {
                if right_in == 0 {
                    panic!("Division by 0!");
                }

                let low = left_in / right_in;
                let high = left_in % right_in;

                self.alu_state_flags[ZF].set(low == 0);
                self.alu_state_flags[NF].set((low as i16) < 0);

                (low, high)
            }

            AluOperation::Inc => {
                let res = left_in.wrapping_add(1);

                self.alu_state_flags[ZF].set(res == 0);
                self.alu_state_flags[NF].set((res as i16) < 0);
                self.alu_state_flags[OF].set((left_in & 0x8000) != (right_in & 0x8000));
                // CF skipped

                (res, 0)
            }

            AluOperation::Dec => {
                let res = left_in.wrapping_sub(1);

                self.alu_state_flags[ZF].set(res == 0);
                self.alu_state_flags[NF].set((res as i16) < 0);
                self.alu_state_flags[OF].set((left_in & 0x8000) != (right_in & 0x8000));
                // CF skipped

                (res, 0)
            }

            AluOperation::Neg => {
                // Strange construction, cuz it's u16 and it can't be negativeded
                let (res, _) = 0u16.overflowing_sub(left_in);

                self.alu_state_flags[ZF].set(res == 0);
                self.alu_state_flags[NF].set((res as i16) < 0);
                self.alu_state_flags[CF].set(left_in != 0);
                self.alu_state_flags[OF].set(left_in == 0x8000);

                (res, 0)
            }

            AluOperation::And | AluOperation::Or | AluOperation::Xor => {
                let res = match operation {
                    AluOperation::And => left_in & right_in,
                    AluOperation::Or => left_in | right_in,
                    AluOperation::Xor => left_in ^ right_in,
                    _ => unreachable!(),
                };

                self.alu_state_flags[ZF].set(res == 0);
                self.alu_state_flags[NF].set((res as i16) < 0);
                self.alu_state_flags[CF].set(false);
                self.alu_state_flags[OF].set(false);

                (res, 0)
            }

            AluOperation::Not => (!left_in, 0),

            AluOperation::ShiftR => {
                let shift = (right_in % 16) as u32;
                if shift == 0 {
                    (left_in, 0)
                } else {
                    // Store last bit to CF
                    let last_out_bit = ((left_in >> (shift - 1)) & 1) == 1;
                    let res = left_in >> shift;

                    self.alu_state_flags[ZF].set(res == 0);
                    self.alu_state_flags[NF].set((res as i16) < 0);
                    self.alu_state_flags[CF].set(last_out_bit);
                    self.alu_state_flags[OF].set((left_in & 0x8000) != (res & 0x8000));

                    (res, 0)
                }
            }

            AluOperation::ShiftL => {
                let shift = (right_in % 16) as u32;
                if shift == 0 {
                    (left_in, 0)
                } else {
                    // Store last out bit to CF
                    let last_out_bit = ((left_in >> (16 - shift)) & 1) == 1;
                    let res = left_in << shift;

                    self.alu_state_flags[ZF].set(res == 0);
                    self.alu_state_flags[NF].set((res as i16) < 0);
                    self.alu_state_flags[CF].set(last_out_bit);
                    self.alu_state_flags[OF].set((left_in & 0x8000) != (res & 0x8000));

                    (res, 0)
                }
            }

            AluOperation::Cmp => {
                let (res, borrow) = left_in.overflowing_sub(right_in);

                self.alu_state_flags[ZF].set(res == 0);
                self.alu_state_flags[NF].set((res as i16) < 0);
                self.alu_state_flags[CF].set(borrow);
                self.alu_state_flags[OF].set((left_in & 0x8000) != (res & 0x8000));

                (0, 0)
            }

            AluOperation::Test => {
                let res = left_in & right_in;

                self.alu_state_flags[ZF].set(res == 0);
                self.alu_state_flags[NF].set((res as i16) < 0);
                self.alu_state_flags[CF].set(false);
                self.alu_state_flags[OF].set(false);

                (0, 0)
            }

            AluOperation::PassFlags => {
                let zf: u16 = if self.alu_state_flags[ZF].get() { 1 } else { 0 };
                let nf: u16 = if self.alu_state_flags[NF].get() { 1 } else { 0 };
                let cf: u16 = if self.alu_state_flags[CF].get() { 1 } else { 0 };
                let of: u16 = if self.alu_state_flags[OF].get() { 1 } else { 0 };
                let val = (zf << 3) | (nf << 2) | (cf << 1) | (of);
                (val, 0)
            }

            AluOperation::LoadFlagsLeft => {
                self.alu_state_flags[ZF].set(left_in >> 3 == 1);
                self.alu_state_flags[NF].set((left_in >> 2) & 0b1 == 1);
                self.alu_state_flags[CF].set((left_in >> 1) & 0b1 == 1);
                self.alu_state_flags[OF].set(left_in & 0b1 == 0);
                (0, 0)
            }
        }
    }

    fn memory_access(&mut self, addr: u16, write_data: u16, is_read: bool, is_write: bool) -> u16 {
        match (is_read, is_write, addr) {
            (true, false, IO_READ) => {
                if self.io_irq {
                    let data = self.io_read_buffer;
                    self.io_read_buffer = 0;
                    self.io_irq = false;
                    data
                } else {
                    panic!("IO_Read is empty!")
                }
            }

            (false, true, IO_READ) => {
                panic!("Attempt to write into IO_READ!")
            }

            (true, false, IO_WRITE) => {
                panic!("Attempt to read into IO_WRITE!")
            }

            (false, true, IO_WRITE) => {
                self.io_write_data.push(write_data);
                0
            }

            (true, false, _) => self.memory[addr as usize],

            (false, true, _) => {
                self.memory[addr as usize] = write_data;
                0
            }

            (false, false, _) => {
                // Idle
                0
            }

            (true, true, _) => {
                panic!("Memory can not be readed and writed at same time!")
            }
        }
    }
}
