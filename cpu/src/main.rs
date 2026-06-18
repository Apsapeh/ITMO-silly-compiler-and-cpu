use clap::Parser;
use std::collections::HashMap;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long)]
    pub bin: String,
    #[arg(short, long, default_value_t = 1_000_000usize)]
    pub limit: usize,
    #[arg(short, long)]
    pub input: Vec<String>,
    #[arg(short, long)]
    pub out: Option<String>,
    #[arg(short, long, default_value_t = false)]
    pub debug: bool,
}

fn main() {
    let args = Args::parse();

    let bin_code = std::fs::read(args.bin).unwrap();
    let in_map = parse_input(args.input);

    let out = cpu::run(bin_code, in_map, args.limit, args.debug);

    println!("Output: {:?}", out);

    if let Some(out_path) = args.out {
        let s = out
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join("\n");

        std::fs::write(out_path, s).unwrap();
    }
}

fn parse_input(input: Vec<String>) -> HashMap<usize, u16> {
    let mut result = HashMap::new();

    for i in input {
        if let Some((tick, value)) = i.split_once(':') {
            let tick = tick.parse().unwrap_or_else(|_| {
                panic!("Unexpected 'tick' value in '{}' - it must be a number", i)
            });
            let value = value.parse().unwrap_or_else(|_| {
                panic!("Unexpected 'value' value in '{}' - it must be a number", i)
            });
            result.insert(tick, value);
        } else {
            panic!("Unexpected input '{}' - rigt format 'tick:value'", i)
        }
    }

    result
}
