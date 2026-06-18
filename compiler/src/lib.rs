use std::collections::{BTreeMap, HashMap};

use crate::codegen::Command;

mod codegen;
mod error;
mod linker;
mod parser;
mod protolexer;

pub fn copmile(src: String, asm_src: Vec<String>, debug: bool) -> Vec<u8> {
    let lex_result = protolexer::lex(src.clone());
    let parse_result = parser::parse(lex_result.lines);
    let mut code = codegen::Codegen::new(parse_result.clone(), lex_result.string_register);

    asm_src.into_iter().for_each(|src| code.add_asm_src(src));

    if debug {
        for n in parse_result {
            println!("{}", n.to_termtree());
        }

        let code = code.clone();
        let int_lbls = code
            .labels
            .iter()
            .map(|(k, v)| (v, k))
            .collect::<HashMap<&usize, &String>>();

        for (i, cmd) in code.commands.into_iter().enumerate() {
            if let Some(lbl) = int_lbls.get(&i) {
                println!("\n{}:", lbl);
            }
            print_cmd(cmd, &code.labels);
        }
    }

    linker::link(code)
}

fn print_cmd(cmd: Command, lbls: &BTreeMap<String, usize>) {
    match cmd {
        Command::Label(l) => {
            println!(
                "\t@ {} ({})",
                l,
                if let Some(n) = lbls.get(&l) {
                    n.to_string()
                } else {
                    "NOT_FOUND!!!".to_string()
                }
            );
        }

        Command::Word(raw) => {
            let opcode = isa::Opcode::from_raw((raw >> 10) as u8);
            let mode = isa::Mode::from_raw(((raw >> 6) & 0b1111) as u8);
            let rd = isa::Register::from_raw(((raw >> 3) & 0b111) as u8);
            let rs = isa::Register::from_raw(((raw) & 0b111) as u8);

            print!("\t{:<6}   ({:^7})", raw as i16, raw);
            print!("     |     ");
            let char = char::decode_utf16([raw]).next().unwrap().unwrap();
            if char.is_ascii() && !char.is_ascii_control() {
                print!("{}", char);
            } else {
                print!(" ");
            }
            print!("     |     ");

            #[allow(clippy::unnecessary_unwrap)]
            if opcode.is_some() && mode.is_some() && rd.is_some() && rs.is_some() {
                print!(
                    "{:6}   {:10}   {:2},  {:2}",
                    opcode.unwrap(),
                    mode.unwrap(),
                    rd.unwrap(),
                    rs.unwrap()
                );
            }
            println!();
        }
    }
}
