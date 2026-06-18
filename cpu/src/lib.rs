use std::collections::HashMap;

mod control_unit;
mod data_path;
mod general;

pub fn run(bin_code: Vec<u8>, in_map: HashMap<usize, u16>, limit: usize, debug: bool) -> Vec<u16> {
    let memory = load_bin(bin_code);
    let mut cu = control_unit::ControlUnit::new(debug);
    let mut dp = data_path::DataPath::new(memory, debug);

    let mut tick = 0usize;
    while tick < limit && !cu.get_is_halted() {
        if debug {
            println!("\nTick {}", tick);
        }

        if let Some(v) = in_map.get(&tick) {
            dp.set_io_read_buffer(*v);
        }

        let signals = cu.tick(&dp);
        dp.tick(signals);
        tick += 1;
    }

    dp.get_io_write_data()
}

fn load_bin(bin_code: Vec<u8>) -> [u16; 65536] {
    let mut memory = [0u16; 0x10000];

    let mut iter = bin_code.chunks(2).peekable();
    while iter.peek().is_some() {
        let chunk = iter.next().unwrap();
        let offset = u16::from_le_bytes([chunk[0], chunk[1]]);
        let chunk = iter.next().unwrap();
        let size = u16::from_le_bytes([chunk[0], chunk[1]]);

        for i in 0..size {
            let chunk = iter.next().unwrap();
            memory[(offset + i) as usize] = u16::from_le_bytes([chunk[0], chunk[1]]);
        }
    }

    memory
}
