use std::collections::HashMap;

use crate::codegen::{Codegen, Command};

const MAIN_PROG_START: u16 = 256;

pub fn link(mut code: Codegen) -> Vec<u8> {
    let mut result = vec![];

    code.add_labels_offset(MAIN_PROG_START as i32);

    let init_asm = "
        sti regtoreg al al
        mov immtoreg sp al
        # 65535
        call immtoreg al al
        @ main
        hlt regtoreg al al
    ";

    let mut init_code = Codegen::new_from_asm(init_asm.to_string());
    // Link main program label to init program
    init_code.labels.extend(code.labels.clone());

    result.extend_from_slice(&0u16.to_le_bytes()); // Mem offset
    result.extend_from_slice(&(init_code.commands.len() as u16).to_le_bytes()); // Block size
    for c in init_code.commands {
        add_command(&mut result, c, &init_code.labels);
    }

    result.extend_from_slice(&MAIN_PROG_START.to_le_bytes()); // Mem offset
    result.extend_from_slice(&(code.commands.len() as u16).to_le_bytes()); // Block size
    for c in code.commands {
        add_command(&mut result, c, &code.labels);
    }

    result
}

fn add_command(mem: &mut Vec<u8>, cmd: Command, labels: &HashMap<String, usize>) {
    match cmd {
        Command::Word(w) => {
            mem.extend_from_slice(&w.to_le_bytes());
        }
        Command::Label(l) => match labels.get(&l) {
            Some(lbl) => {
                mem.extend_from_slice(&(*lbl as u16).to_le_bytes());
            }
            None => {
                panic!("Linking error: Label '{}' not found", l);
            }
        },
    }
}
