use crate::types::NumWord;

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Token<'a> {
    pub word: NumWord<'a>,
    pub kind: TokenKind,
}

pub mod tk {
    /// https://danielkeep.github.io/tlborm/book/blk-counting.html
    /// 0usize $(+ replace_expr!($tts 1usize))*
    macro_rules! replace_expr {
        ($_t:tt $sub:expr) => {
            $sub
        };
    }

    macro_rules! define_token_enum {
        (
            $vis:vis enum $name:ident { $($variant:ident),* $(,)? }
            => $st_vis:vis $static_name:ident
        ) => {
            #[derive(Debug, PartialEq, Clone, Copy)]
            $vis enum $name { $($variant),* }

            $vis static $static_name: [super::TokenKind; 0usize $(+ replace_expr!($variant 1usize))*] = [
                $( super::TokenKind::$name($name::$variant) ),*
            ];
        }
    }

    define_token_enum! {
        pub enum General {
            // Keywords
            Fn,
            Let,
            If,
            Else,
            While,
            Return,

            Ident,
            String, // "..."
            Number,

            // Brackets
            // LBracket,    // [
            // RBracket,    // ]
            LRndBracket, // (
            RRndBracket, // )
            LBrace,      // {
            RBrace,      // }

            Arrow,     // ->
            Semicolon, // ;
            Colon,     // :
            Comma,     // ,
        } => pub TOKEN_KIND_GENERAL_CATEGORY
    }

    define_token_enum! {
        pub enum Operator {
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

            // Logical Operators
            And, // &&
            Or,  // ||

            // Comparators
            Eq,    // ==
            NotEq, // !=
            Lt,    // <
            Gt,    // >
            LtEq,  // <=
            GtEq,  // >=
        } => pub TOKEN_KIND_OPERATOR_CATEGORY
    }

    define_token_enum! {
        pub enum Setter {
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
        } => pub TOKEN_KIND_SETTER_CATEGORY
    }

    pub use General::*;
    pub use Operator::*;
    pub use Setter::*;
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum TokenKind {
    General(tk::General),
    Operator(tk::Operator),
    Setter(tk::Setter),
    Unknown,
}

impl TokenKind {
    pub fn is_general(&self) -> bool {
        match self {
            Self::General(_) => true,
            _ => false,
        }
    }

    pub fn is_operator(&self) -> bool {
        match self {
            Self::Operator(_) => true,
            _ => false,
        }
    }

    pub fn is_setter(&self) -> bool {
        match self {
            Self::Setter(_) => true,
            _ => false,
        }
    }

    pub fn is_unknown(&self) -> bool {
        match self {
            Self::Unknown => true,
            _ => false,
        }
    }
}
