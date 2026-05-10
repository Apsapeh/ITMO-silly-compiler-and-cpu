use crate::protolexer;

pub fn error(msg: &str, line: &protolexer::SourceLine) -> ! {
    eprintln!("Error at line {}: {}", line.line_number, msg);
    std::process::exit(1);
}
