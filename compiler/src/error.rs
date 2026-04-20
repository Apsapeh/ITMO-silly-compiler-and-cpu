use std::fmt::Display;

use crate::token::{Token, TokenKind};

pub enum ParserError<'a> {
    UnexpectedToken {
        token: Token<'a>,
        expected: &'static [TokenKind],
    },
    EndOfFile {
        expected: Option<&'static [TokenKind]>,
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

// pub enum ExpectedTokenKind {
//     StaicSlice(&'static [TokenKind]),
//     Vector(Vec<TokenKind>),
// }

// impl ExpectedTokenKind {
//     pub fn contains(&self, token_kind: &TokenKind) -> bool {
//         match self {
//             Self::StaicSlice(a) => a.contains(token_kind),
//             Self::Vector(a) => a.contains(token_kind),
//         }
//     }
// }

// impl Display for ExpectedTokenKind {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         // TODO: impl
//         let placeholder = "";
//         f.write_str(placeholder)
//     }
// }
