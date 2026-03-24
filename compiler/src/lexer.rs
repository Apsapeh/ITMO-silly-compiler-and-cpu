use crate::{
    diagnostic::Diagnostic,
    types::{self, NumWord},
};

const AVERAGE_WORD_LEN: usize = 4;

#[derive(Debug, PartialEq)]
pub struct Token<'a> {
    word: NumWord<'a>,
    kind: TokenKind,
}

#[derive(Debug, PartialEq)]
pub enum TokenKind {
    // Keywords
    Fn,
    Let,
    If,
    Else,
    While,
    Return,

    Ident,
    String, // "..."

    // Operators
    Plus,      // +
    Minus,     // -
    Star,      // *
    Slash,     // /
    Mod,       // %
    Not,       // !
    BitInv,    // ~
    LShift,    // <<
    RShift,    // >>
    Ampersand, // &
    Bar,       // |

    // Setters
    Set,       // =
    PlusSet,   // +=
    MinusSet,  // -=
    StarSet,   // *=
    SlashSet,  // /=
    ModSet,    // %=
    BitInvSet, // ~=
    LShiftSet, // <<=
    RShiftSet, // >>=
    BitAndSet, // &=
    BitOrSet,  // |=

    // Logical
    And, // &&
    Or,  // ||

    // Comparators
    Eq,    // ==
    NotEq, // !=
    Lt,    // <
    Gt,    // >
    LtEq,  // <=
    GtEq,  // >=

    // Brackets
    LBracket,    // [
    RBracket,    // ]
    LRndBracket, // (
    RRndBracket, // )
    LBrace,      // {
    RBrace,      // }

    Arrow,     // ->
    Semicolon, // ;
    Colon,     // :
    Comma,     // ,
    Dot,       // .
    Unknown,
}

type LexerOutput<'a> = Vec<Token<'a>>;

pub fn lex<'a>(source_code: &'a str, diag: &mut Diagnostic) -> Result<LexerOutput<'a>, ()> {
    let mut lexer_splitter = LexerSplitter::new(source_code, diag);
    let spltted = lexer_splitter.run()?;

    let mut result = Vec::with_capacity(spltted.len());
    for num_word in spltted {
        #[rustfmt::skip]
        let kind = match num_word.word {
            "fn"      => TokenKind::Fn,
            "let"     => TokenKind::Let,
            "if"      => TokenKind::If,
            "else"    => TokenKind::Else,
            "while"   => TokenKind::While,
            "return"  => TokenKind::Return,
            "+"       => TokenKind::Plus,
            "-"       => TokenKind::Minus,
            "*"       => TokenKind::Star,
            "/"       => TokenKind::Slash,
            "%"       => TokenKind::Mod,
            "!"       => TokenKind::Not,
            "~"       => TokenKind::BitInv,
            "<<"      => TokenKind::LShift,
            ">>"      => TokenKind::RShift,
            "&"       => TokenKind::Ampersand,
            "|"       => TokenKind::Bar,
            "="       => TokenKind::Set,
            "+="      => TokenKind::PlusSet,
            "-="      => TokenKind::MinusSet,
            "*="      => TokenKind::StarSet,
            "/="      => TokenKind::SlashSet,
            "%="      => TokenKind::ModSet,
            "!="      => TokenKind::NotEq,
            "~="      => TokenKind::BitInvSet,
            "<<="     => TokenKind::LShiftSet,
            ">>="     => TokenKind::RShiftSet,
            "&="      => TokenKind::BitAndSet,
            "|="      => TokenKind::BitOrSet,
            "&&"      => TokenKind::And,
            "||"      => TokenKind::Or,
            "=="      => TokenKind::Eq,
            "<"       => TokenKind::Lt,
            ">"       => TokenKind::Gt,
            "<="      => TokenKind::LtEq,
            ">="      => TokenKind::GtEq,
            "["       => TokenKind::LBracket, 
            "]"       => TokenKind::RBracket, 
            "("       => TokenKind::LRndBracket, 
            ")"       => TokenKind::RRndBracket, 
            "{"       => TokenKind::LBrace, 
            "}"       => TokenKind::RBrace,
            "->"      => TokenKind::Arrow,
            ";"       => TokenKind::Semicolon,
            ":"       => TokenKind::Colon,
            ","       => TokenKind::Comma,
            "."       => TokenKind::Dot,
            _ if num_word.word.starts_with('"') => TokenKind::String,
            _ if word_is_ident(num_word.word)   => TokenKind::Ident,
            _ => {
                if let Some(number) = word_is_number(num_word.word) {
                    todo!()
                } else {
                    diag.error(
                        format!("unexpected token '{}'", num_word.word),
                        num_word.line,
                        num_word.col
                    );
                    TokenKind::Unknown
                }
            }
        };

        result.push(Token {
            word: num_word,
            kind,
        });
    }

    Ok(result)
}

fn word_is_ident(word: &str) -> bool {
    let mut iter = word.chars();
    // First chars always exists
    if iter.next().unwrap().is_numeric() {
        return false;
    }
    for c in iter {
        if !c.is_alphanumeric() || c != '_' {
            return false;
        }
    }
    true
}

fn word_is_number(word: &str) -> Option<()> {
    todo!()
}

struct WordStart {
    idx: usize,
    line: usize,
    col: usize,
}

impl WordStart {
    fn new(idx: usize, line: usize, col: usize) -> Self {
        Self { idx, line, col }
    }
}

#[derive(PartialEq, Eq)]
enum LexerSplitterState {
    Normal,
    InString,
    InSingleComment,
}

struct LexerSplitter<'a, 'b> {
    diag: &'b mut Diagnostic,
    src: &'a str,
    iter: std::iter::Peekable<std::str::CharIndices<'a>>,
    result: Vec<NumWord<'a>>,
    state: LexerSplitterState,
    word_start_line: usize,
    word_start_column: usize,
    word_start_idx: usize,
    cur_line: usize,
    cur_column: usize,
}

impl<'a, 'b> LexerSplitter<'a, 'b> {
    fn new(source_code: &'a str, diag: &'b mut Diagnostic) -> Self {
        Self {
            diag,
            src: source_code,
            iter: source_code.char_indices().peekable(),
            result: Vec::with_capacity(source_code.len() / AVERAGE_WORD_LEN),
            state: LexerSplitterState::Normal,
            word_start_line: 1,
            word_start_column: 1,
            word_start_idx: 0,
            cur_line: 1,
            cur_column: 1,
        }
    }

    fn run(mut self) -> Result<Vec<NumWord<'a>>, ()> {
        while let Some((cur_idx, cur_ch)) = self.iter.next() {
            let cur_ch_is_alphanum = cur_ch.is_alphanumeric();
            let (peek_idx, peek_ch, peek_ch_is_alphanum) = match self.iter.peek() {
                Some(&p) => (p.0, p.1, p.1.is_alphanumeric()),
                None => (self.src.len(), '\0', !cur_ch_is_alphanum),
            };

            self.process(
                cur_idx,
                cur_ch,
                cur_ch_is_alphanum,
                peek_idx,
                peek_ch,
                peek_ch_is_alphanum,
            );

            self.cur_column += 1;
        }

        if self.state == LexerSplitterState::InString {
            self.diag.fatal(
                "unclosed string",
                self.word_start_line,
                self.word_start_column,
            );
            return Err(());
        }

        Ok(self.result)
    }

    fn process(
        &mut self,
        cur_idx: usize,
        cur_ch: char,
        cur_ch_is_alphanum: bool,
        peek_idx: usize,
        peek_ch: char,
        peek_ch_is_alphanum: bool,
    ) {
        let word_start = match self.state {
            LexerSplitterState::InSingleComment => {
                if let Some(r) = self.try_feed_line(cur_ch, peek_ch) {
                    self.state = LexerSplitterState::Normal;
                    Some(r)
                } else {
                    None
                }
            }

            LexerSplitterState::InString => {
                if cur_ch == '"' {
                    self.state = LexerSplitterState::Normal;
                    self.push_word(peek_idx);
                    Some(WordStart::new(peek_idx, self.cur_line, self.cur_column + 1))
                } else {
                    None
                }
            }

            LexerSplitterState::Normal => {
                // New line
                if let Some(r) = self.try_feed_line(cur_ch, peek_ch) {
                    self.push_word(cur_idx);
                    Some(r)
                // Single line comment
                } else if cur_ch == '/' && peek_ch == '/' {
                    self.push_word(cur_idx);
                    self.state = LexerSplitterState::InSingleComment;
                    None
                // String start
                } else if cur_ch == '"' {
                    self.push_word(cur_idx);
                    self.state = LexerSplitterState::InString;
                    Some(WordStart::new(cur_idx, self.cur_line, self.cur_column))
                // Some of separators (special chars)
                } else if matches!(cur_ch, ';' | '.' | ',' | '{' | '}' | '[' | ']' | '(' | ')') {
                    self.push_word(cur_idx);
                    self.word_start_idx = cur_idx;
                    self.word_start_line = self.cur_line;
                    self.word_start_column = self.cur_column;
                    self.push_word(peek_idx);
                    Some(WordStart::new(peek_idx, self.cur_line, self.cur_column + 1))
                //
                } else if cur_ch_is_alphanum != peek_ch_is_alphanum {
                    self.push_word(peek_idx);
                    Some(WordStart::new(peek_idx, self.cur_line, self.cur_column + 1))
                } else {
                    None
                }
            }
        };

        if let Some(ws) = word_start {
            self.word_start_idx = ws.idx;
            self.word_start_line = ws.line;
            self.word_start_column = ws.col;
        }
    }

    fn push_word(&mut self, word_end_idx: usize) {
        let mess_word = &self.src[self.word_start_idx..word_end_idx];
        let end_mess_word = mess_word.trim_start();

        if !end_mess_word.is_empty() {
            let num_word = NumWord::new(
                end_mess_word.trim_end(),
                self.word_start_line,
                self.word_start_column + mess_word.len() - end_mess_word.len(),
            );
            self.result.push(num_word);
        }
    }

    fn try_feed_line(&mut self, cur_ch: char, peek_ch: char) -> Option<WordStart> {
        if cur_ch == '\n' || cur_ch == '\r' {
            if cur_ch == '\r' && peek_ch == '\n' {
                self.iter.next();
            }

            self.cur_line += 1;
            self.cur_column = 0;
            Some(WordStart::new(
                self.iter.peek().unwrap_or(&(self.src.len(), '\0')).0,
                self.cur_line,
                self.cur_column,
            ))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{LexerOutput, TokenKind, TokenKind::*};
    use crate::diagnostic::Diagnostic;

    fn call_lexer<'a>(source_code: &'a str) -> (Result<LexerOutput<'a>, ()>, Diagnostic) {
        let mut diag = Diagnostic::new();
        let output = super::lex(source_code, &mut diag);
        (output, diag)
    }

    fn token_kinds(output: LexerOutput) -> Vec<TokenKind> {
        output.into_iter().map(|t| t.kind).collect()
    }

    #[test]
    fn simple_test_1() {
        let (result, diag) = call_lexer(r#" let a: 32 = 10 * 5 - 10; "#);

        assert!(diag.is_clear());
        assert_eq!(
            token_kinds(result.unwrap()),
            vec![
                Let, Ident, Colon, Ident, Set, Ident, Star, Ident, Minus, Ident, Semicolon
            ]
        );
    }

    #[test]
    fn string() {
        let (result, diag) = call_lexer(
            r#" "сдкйа;кй
        eeeeeeeee    usoatuh
                    ausaotehucg
            aoe          cuhча 324
            84688[{[{  };;,.u ao'qjk {(y;)}]}
            oeu]

            ііїѡіѳѳѳёшццѧѕ
            ѧѧѧ
            іі҃҃҃с

            ір
            ѹ҃҃ѹѹѹ
            їїї
            8648" "#,
        );

        assert!(diag.is_clear());
        assert_eq!(token_kinds(result.unwrap()), vec![String]);
    }

    #[test]
    fn unfmt() {
        let (result1, diag1) = call_lexer(
            r#"
            fn fib(n: u32) -> u32 {
                "Ансасдф ... 45 }() {[&]}
                
                Тофик";
                if n < 2 {return 1;}//Aboba
                return fib(n - 2) + fib(n - 1);
            }"#,
        );

        let (result2, diag2) = call_lexer(
            r#"
            fn fib(n:u32)->u32{"Ансасдф ... 45 }() {[&]}
                
                Тофик";if n<2{return 1;}return fib(n-2)+fib(n-1);}"#,
        );

        assert!(diag1.is_clear());
        assert!(diag2.is_clear());
        assert_eq!(token_kinds(result1.unwrap()), token_kinds(result2.unwrap()));
    }

    #[test]
    fn empty() {
        let (result, diag) = call_lexer(r#""#);

        assert!(diag.is_clear());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn unclosed_string() {
        let (result, diag) = call_lexer(r#" a = "Hi there! "#);

        assert!(result.is_err());
        assert!(diag.has_fatal());
    }

    #[test]
    fn unexpected_token() {
        let (result, diag) = call_lexer(r#" a = b === c "#);

        assert!(diag.has_error());
        assert_eq!(
            token_kinds(result.unwrap()),
            vec![Ident, Set, Ident, Unknown, Ident]
        );
    }
}
