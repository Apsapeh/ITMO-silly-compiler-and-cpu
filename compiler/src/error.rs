use std::fmt::Display;

use crate::lexer::{Token, TokenKind};

pub enum ParserError<'a> {
    UnexpectedToken {
        token: Token<'a>,
        expected: Vec<TokenKind>,
    },
    EndOfFile {
        expected: Vec<TokenKind>,
    },
}

impl<'a> Display for ParserError<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let m = match self {
            Self::UnexpectedToken { token, expected } => {
                format!("unexpected token '{}', expected", token.word.word)
            }
            Self::EndOfFile { expected } => "found end of file".to_string(),
        };

        f.write_str(m.as_str())
    }
}
