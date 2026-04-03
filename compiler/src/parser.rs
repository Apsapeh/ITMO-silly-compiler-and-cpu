use crate::error::ParserError;

use crate::{diagnostic::Diagnostic, lexer::*, types::NumWord};

enum ASTNode<'a> {
    FnDecl {
        name: NumWord<'a>,
        args: Vec<Field<'a>>,
        return_type: Option<NumWord<'a>>,
        block: Box<ASTNode<'a>>,
    },

    Let {
        name: NumWord<'a>,
        var_type: NumWord<'a>,
        expr: Box<ASTNode<'a>>,
    },

    If {
        cond: Box<ASTNode<'a>>,
        block: Box<ASTNode<'a>>,
        else_block: Option<Box<ASTNode<'a>>>,
    },

    While {
        expr: Box<ASTNode<'a>>,
        block: Box<ASTNode<'a>>,
    },

    Block {
        children: Vec<ASTNode<'a>>,
    },

    // Expression
    Expression {},

    FnCall {
        name: NumWord<'a>,
        args: Vec<ASTNode<'a>>,
    },
}

struct Field<'a> {
    name: NumWord<'a>,
    var_type: NumWord<'a>,
}

#[derive(Clone, Copy)]
struct LexerOutputCursor<'a> {
    data: &'a LexerOutput<'a>,
    cursor: usize,
}

impl<'a> LexerOutputCursor<'a> {
    pub fn new(data: &'a LexerOutput<'a>) -> Self {
        Self { data, cursor: 0 }
    }

    pub fn next(&mut self) -> Option<&Token<'a>> {
        if let Some(t) = self.data.get(self.cursor) {
            self.cursor += 1;
            Some(t)
        } else {
            None
        }
    }

    pub fn peek(&self, offset: usize) -> Option<&Token<'a>> {
        self.data.get(self.cursor + offset)
    }

    pub fn find(&self, to_find: TokenKind) -> Option<(usize, &Token<'a>)> {
        self.data
            .iter()
            .skip(self.cursor)
            .enumerate()
            .find(|e| e.1.kind == to_find)
    }
}

type SubParserOutput<'a> = Result<(ASTNode<'a>, LexerOutputCursor<'a>), ParserError<'a>>;

pub fn parse<'a>(src: LexerOutput<'a>, diag: &mut Diagnostic) {
    // let parser = Parser::new(src);
    let mut cursor = LexerOutputCursor::new(&src);
    let root = parse_block_with_params(cursor, true);
}

fn parse_block<'a>(mut cursor: LexerOutputCursor<'a>) -> SubParserOutput<'a> {
    parse_block_with_params(cursor, false)
}

fn parse_block_with_params<'a>(
    mut cursor: LexerOutputCursor<'a>,
    is_root: bool,
) -> SubParserOutput<'a> {
    while let Some(token) = cursor.next() {
        let d = match token.kind {
            TokenKind::LBrace => parse_block(cursor),
            TokenKind::Fn => parse_function_decl(cursor),
            TokenKind::RBrace => {
                break;
            }
            _ => unimplemented!(),
        };
    }
    unreachable!()
    // Ok((ASTNode::Block { children: vec![] }))
}

macro_rules! match_token {
    ($cursor:ident is $($token:ident)or+) => {
        match $cursor.next() {
            Some(token) => match token.kind {
                $(TokenKind::$token => token, )*
                _ => {
                    return Err(ParserError::UnexpectedToken {
                        token: *token,
                        expected: vec![$(TokenKind::$token, )*]
                    });
                }
            },
            None => {
                return Err(ParserError::EndOfFile {
                    expected: vec![$(TokenKind::$token, )*]
                });
            }
        }
    };
}

fn parse_expression<'a>(
    mut cursor: LexerOutputCursor<'a>,
    terminator: TokenKind,
) -> SubParserOutput<'a> {
    enum ExprParserExpected {
        Operator,
        Operand,
    }
    use ExprParserExpected::*;

    let mut state = Operand;

    while let Some(token) = cursor.next() {
        let s = match token.kind {
            TokenKind::LBrace => parse_block(cursor),
            TokenKind::LRndBracket => parse_expression(cursor, TokenKind::RRndBracket),
            TokenKind::Ident => {
                if let Some(next_token) = cursor.peek(1)
                    && next_token.kind == TokenKind::LRndBracket
                {
                    cursor.next();
                    let e = parse_expression(cursor, TokenKind::Comma)?;
                    match_token!(cursor is RRndBracket);
                    todo!();
                } else {
                    todo!();
                }
            }
            _ => {
                todo!();
            }
        };
    }
    todo!();
}
// fn <name> ( [ <arg_name> : <arg_type>],* ) ? -> <return_type> ? %Block%
fn parse_function_decl<'a>(mut cursor: LexerOutputCursor<'a>) -> SubParserOutput<'a> {
    let name = match_token!(cursor is Ident).word;
    // Args
    match_token!(cursor is LRndBracket);
    let mut args = vec![];
    cursor = parse_fields_recursive(cursor, TokenKind::RRndBracket, &mut args)?;
    match_token!(cursor is RRndBracket);

    let (block, cursor) = parse_block(cursor)?;

    Ok((
        ASTNode::FnDecl {
            name,
            args,
            return_type: None,
            block: Box::new(block),
        },
        cursor,
    ))
}

// let <name> : <var_type> = %Block%
fn parse_let_decl<'a>(mut cursor: LexerOutputCursor<'a>) -> SubParserOutput<'a> {
    let name = match_token!(cursor is Ident).word;
    match_token!(cursor is Colon);
    let var_type = match_token!(cursor is Ident).word;
    match_token!(
        cursor is Set    or PlusSet   or MinusSet  or StarSet   or SlashSet  or
                  ModSet or BitInvSet or LShiftSet or RShiftSet or BitAndSet or BitOrSet
    );
    let (expr, new_cursor) = parse_expression(cursor, TokenKind::Semicolon)?;
    cursor = new_cursor;
    match_token!(cursor is Semicolon);
    todo!();

    Ok((
        ASTNode::Let {
            name,
            var_type,
            expr: Box::new(expr),
        },
        cursor,
    ))
}

fn parse_fields_recursive<'a>(
    mut cursor: LexerOutputCursor<'a>,
    end_token_kind: TokenKind,
    result: &mut Vec<Field<'a>>,
) -> Result<LexerOutputCursor<'a>, ParserError<'a>> {
    // Don't move the cursor to check the end, cuz it is not a part of fields
    if let Some(token) = cursor.peek(0)
        && token.kind == end_token_kind
    {
        return Ok(cursor);
    }

    let (field, new_cursor) = try_parse_field(cursor)?;
    result.push(field);
    cursor = new_cursor;

    // If next char is Comma (','), then next must be a field
    if let Some(token) = cursor.peek(0)
        && token.kind == TokenKind::Comma
    {
        cursor.next(); // cuz 'peek'. Need to skip Comma 
        cursor = parse_fields_recursive(cursor, end_token_kind, result)?;
    };

    Ok(cursor)
}

// <arg_name> : <arg_type>
fn try_parse_field<'a>(
    mut cursor: LexerOutputCursor<'a>,
) -> Result<(Field<'a>, LexerOutputCursor<'a>), ParserError<'a>> {
    let name = match_token!(cursor is Ident).word;
    match_token!(cursor is Colon);
    let var_type = match_token!(cursor is Ident).word;

    Ok((Field { name, var_type }, cursor))
}

#[cfg(test)]
mod tests {
    use super::TokenKind::*;
    use super::*;

    struct MockedTokens<'a> {
        tokens: Vec<Token<'a>>,
    }

    impl<'a> MockedTokens<'a> {
        fn new(to_gen: &[TokenKind]) -> Self {
            let tokens = to_gen
                .iter()
                .map(|&kind| {
                    let word = NumWord::new("", 0, 0);
                    Token { word, kind }
                })
                .collect::<Vec<_>>();
            Self { tokens }
        }

        fn cursor(&'a self) -> LexerOutputCursor<'a> {
            LexerOutputCursor::new(&self.tokens)
        }
    }

    fn test_try_parse_field(to_test: &[TokenKind], is_ok: bool) {
        let tokens = MockedTokens::new(to_test);

        let result = try_parse_field(tokens.cursor());
        assert_eq!(result.is_ok(), is_ok);
    }

    #[test]
    fn try_parse_field_correct() {
        test_try_parse_field(&[Ident, Colon, Ident], true);
    }

    #[test]
    fn try_parse_field_incorrect() {
        test_try_parse_field(&[Ident, Ident, Ident], false);
    }

    #[test]
    fn try_parse_field_empty() {
        test_try_parse_field(&[], false);
    }

    fn test_parse_field_recursive(
        to_test: &[TokenKind],
        end_token_kind: TokenKind,
        is_ok: bool,
        result_fields_count: usize,
    ) {
        let tokens = MockedTokens::new(to_test);

        let mut fields = vec![];
        let result = parse_fields_recursive(tokens.cursor(), end_token_kind, &mut fields);
        assert_eq!(result.is_ok(), is_ok);
        assert_eq!(fields.len(), result_fields_count);
    }

    #[test]
    fn parse_field_recursive_double_comma() {
        let end_token_kind = RRndBracket;
        let mut to_test = [Ident, Colon, Ident, Comma, Comma];
        test_parse_field_recursive(&to_test, end_token_kind, false, 1);
    }

    #[test]
    fn parse_field_recursive_three_args() {
        let end_token_kind = RRndBracket;
        let mut to_test = [Ident, Colon, Ident, Comma].repeat(3);
        to_test.push(end_token_kind);
        test_parse_field_recursive(&to_test, end_token_kind, true, 3);
    }

    #[test]
    fn parse_field_recursive_three_hundred_args() {
        let end_token_kind = RRndBracket;
        let mut to_test = [Ident, Colon, Ident, Comma].repeat(300);
        to_test.push(end_token_kind);
        test_parse_field_recursive(&to_test, end_token_kind, true, 300);
    }

    #[test]
    fn parse_field_recursive_three_hundred_args_wo_ending() {
        let end_token_kind = RRndBracket;
        let to_test = [Ident, Colon, Ident, Comma].repeat(300);
        test_parse_field_recursive(&to_test, end_token_kind, false, 300);
    }

    #[test]
    fn parse_field_recursive_three_args_wo_comma() {
        let end_token_kind = RRndBracket;
        let mut to_test = [Ident, Colon, Ident].repeat(3);
        to_test.push(end_token_kind);
        test_parse_field_recursive(&to_test, end_token_kind, true, 1);
    }
}
