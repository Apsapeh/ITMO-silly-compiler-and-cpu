use clap::Parser;

#[derive(Parser, Debug)]
pub struct Args {
    #[arg(short, long)]
    pub src: String,
    #[arg(short, long)]
    pub asm_src: Vec<String>,
    #[arg(short, long, default_value = "out")]
    pub out: String,
    #[arg(short, long, default_value_t = false)]
    pub debug: bool,
}

fn main() {
    let args = Args::parse();

    let src = std::fs::read_to_string(&args.src)
        .unwrap_or_else(|_| panic!("File {} not found", args.src));

    let asm_src = args
        .asm_src
        .iter()
        .map(|asm_src_path| {
            std::fs::read_to_string(asm_src_path)
                .unwrap_or_else(|_| panic!("File {} not found", asm_src_path))
        })
        .collect::<Vec<_>>();

    let out = compiler::copmile(src, asm_src, args.debug);
    std::fs::write(args.out, out).unwrap();
}
