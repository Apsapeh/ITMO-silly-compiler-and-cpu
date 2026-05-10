mod error;
mod parser;
mod protolexer;

fn main() {
    let src = std::fs::read_to_string("examples/dev.shit").unwrap();
    let lex_result = protolexer::lex(src);
    let parse_result = parser::parse(lex_result.lines);
}
