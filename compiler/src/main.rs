use clap::Parser;
use std::collections::HashMap;

use crate::codegen::Command;

mod codegen;
mod error;
mod linker;
mod parser;
mod protolexer;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long)]
    pub src: String,
    #[arg(short, long)]
    pub asm_src: Option<String>,
    #[arg(short, long, default_value = "out")]
    pub out: String,
    #[arg(short, long, default_value_t = false)]
    pub debug: bool,
}

fn main() {
    let args = Args::parse();

    let src = std::fs::read_to_string(&args.src)
        .unwrap_or_else(|_| panic!("File {} not found", args.src));
    let lex_result = protolexer::lex(src);
    let parse_result = parser::parse(lex_result.lines);
    let mut code = codegen::Codegen::new(parse_result.clone(), lex_result.string_register);

    if let Some(asm_src_path) = args.asm_src {
        let asm_src = std::fs::read_to_string(asm_src_path)
            .unwrap_or_else(|_| panic!("File {} not found", args.src));
        code.add_asm_src(asm_src);
    }

    let out = linker::link(code.clone());
    std::fs::write(args.out, out).unwrap();

    if args.debug {
        for n in parse_result {
            println!("{}", n.to_termtree());
        }

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
}

fn print_cmd(cmd: Command, lbls: &HashMap<String, usize>) {
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
                    "{:10?}   {:10?}   {:10?},  {:10?}",
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
