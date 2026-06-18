use std::collections::BTreeMap;

use crate::codegen::{Codegen, Command};

const MAIN_PROG_START: u16 = 256;

pub fn link(mut code: Codegen) -> Vec<u8> {
    let mut result = vec![];

    code.add_labels_offset(MAIN_PROG_START as i32);

    let init_asm = if code.labels.contains_key("IO_ISR") {
        "
            ; Set SP to end of memory
            mov immtoreg sp al
            # 65535
    
            ; Bind interruption handler
            mov immtomema al al
            # 64
            @ __io_isr_wrapper
    
            ; Enable interruptions
            sti regtoreg al al
    
            ; Go to entry point
            call immtoreg al al
            @ main
    
            hlt regtoreg al al
    
            : __io_isr_wrapper
            call immtoreg al al
            @ io_isr
            iret regtoreg al al
        "
    } else {
        "
            ; Set SP to end of memory
            mov immtoreg sp al
            # 65535
            
            ; Go to entry point
            call immtoreg al al
            @ main
            
            hlt regtoreg al al
        "
    };

    let mut init_code = Codegen::new_from_asm(init_asm.to_string());
    // Link main program label to init program
    init_code.labels.extend(code.labels.clone());

    if init_code.commands.len() > 63 {
        panic!("Initial program is too big!. It's compiler error");
    }

    if code.commands.len() + MAIN_PROG_START as usize > 0xFFFF - 256 {
        panic!("Main program is too big! You must simplify your code!");
    }

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

fn add_command(mem: &mut Vec<u8>, cmd: Command, labels: &BTreeMap<String, usize>) {
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
