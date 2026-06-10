mod control_unit;
mod data_path;
mod general;

const IO_READ_PORT: u16 = 0x80;
const IO_WRITE_PORT: u16 = 0x81;

const IO_INPUT_TICK: u64 = 100;
const IO_INPUT_VALUE: u16 = 21;

fn encode_insn(opcode: isa::Opcode, mode: isa::Mode, rd: u8, rs: u8) -> u16 {
    ((opcode as u16) << 10) | ((mode as u16) << 6) | ((rd as u16) << 3) | (rs as u16)
}

fn load_io_irq_test(memory: &mut [u16; 0x10000]) {
    use isa::Mode::*;
    use isa::Opcode::*;

    let loop_addr = 0x0001u16;
    memory[0x0000] = encode_insn(isa::Opcode::Sti, isa::Mode::RegToReg, 0, 0);
    memory[loop_addr as usize] = encode_insn(isa::Opcode::Jmp, isa::Mode::ImmToReg, 0, 0);
    memory[loop_addr as usize + 1] = loop_addr;

    memory[data_path::IO_IRQ_ADDR as usize] = 0x0100;

    memory[0x0100] = encode_insn(isa::Opcode::Mov, isa::Mode::MemaToReg, 0, 0);
    memory[0x0101] = IO_READ_PORT;
    memory[0x0102] = encode_insn(isa::Opcode::Add, isa::Mode::RegToReg, 0, 0);
    memory[0x0103] = encode_insn(isa::Opcode::Mov, isa::Mode::RegToMema, 0, 0);
    memory[0x0104] = IO_WRITE_PORT;
    memory[0x0105] = encode_insn(Hlt, RegToReg, 0, 0);
    // memory[0x0105] = encode_insn(isa::Opcode::Iret, isa::Mode::RegToReg, 0, 0);
}

fn load_add_test(memory: &mut [u16; 0x10000]) {
    use isa::Mode::*;
    use isa::Opcode::*;

    memory[0x0] = encode_insn(Jmp, ImmToReg, 0, 0);
    memory[0x1] = 0x1001;
    memory[0x1000] = 50;
    memory[0x1001] = encode_insn(Add, ImmToMema, 0, 0);
    memory[0x1002] = 0x1000;
    memory[0x1003] = 17;
    memory[0x1004] = encode_insn(Hlt, RegToReg, 0, 0);
}

fn main() {
    let mut memory = [0u16; 0x10000];
    load_io_irq_test(&mut memory);
    // load_add_test(&mut memory);
    //
    println!("Mem[0x1000..0x1004]: {:#?}", &memory[0x0..0x110]);

    let mut cu = control_unit::ControlUnit::new();
    let mut dp = data_path::DataPath::new(memory);

    let mut tick = 0u64;
    let max_ticks = 50;

    while tick < max_ticks && !cu.get_is_halted() {
        if tick == 10 {
            dp.set_io_read_buffer(IO_INPUT_VALUE);
        }
        println!("\nTick {}", tick);

        println!("IRQ: {}", dp.get_io_irq());
        let signals = cu.tick(&dp);
        // println!("Signal: {:#?}", signals);
        dp.tick(signals);
        tick += 1;
    }

    let new_memory = dp.get_memory();
    println!("Mem[0x1000..0x1004]: {:?}", &new_memory[0x0..0x110]);

    let output = dp.get_io_write_data();

    println!("Memory: {:?}", dp.get_io_write_data());

    println!(
        "OK: tick {IO_INPUT_TICK} input={IO_INPUT_VALUE} -> output={} (total ticks={tick})",
        output[0]
    );
}
