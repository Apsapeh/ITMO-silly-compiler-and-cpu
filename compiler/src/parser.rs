use crate::{diagnostic::Diagnostic, error::*, lexer::*, token::tk::*, token::*, types::NumWord};

use TokenKind::General as Gn;
use TokenKind::Operator as Op;
use TokenKind::Setter as St;
use TokenKind::Unknown;

struct ASTNode<'a> {
    token: Token<'a>,
    kind: ASTNodeKind<'a>,
}

impl<'a> ASTNode<'a> {
    fn new(token: Token<'a>, kind: ASTNodeKind<'a>) -> Self {
        Self { token, kind }
    }
}

enum ASTNodeKind<'a> {
    FnDecl {
        name: NumWord<'a>,
        args: Vec<Field<'a>>,
        return_type: Option<NumWord<'a>>,
        block: Box<ASTNode<'a>>,
    },

    LetDecl {
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

    // VarUse {
    //     name: NumWord<'a>,
    // },

    // VarSet {
    //     name: NumWord<'a>,
    //     expr: Box<ASTNode<'a>>,
    // },
    String {
        string: NumWord<'a>,
    },

    Number {},

    Let {},

    UnaryOperator {
        op: UnaryOperator,
        operand: Box<ASTNode<'a>>,
    },

    BinaryOperator {
        op: BinaryOperator,
        left: Box<ASTNode<'a>>,
        right: Box<ASTNode<'a>>,
    },
}

enum UnaryOperator {
    Not,
    BitInv,
    Ref,
    Deref,
}

enum BinaryOperator {
    Set,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    BitAnd,
    BitOr,
    LShift,
    RShift,
    Eq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    // TODO: Add '.' to access to struct's fields
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

    pub fn next(&mut self) -> Option<&'a Token<'a>> {
        if let Some(t) = self.data.get(self.cursor) {
            self.cursor += 1;
            Some(t)
        } else {
            None
        }
    }

    pub fn peek_prev(&self, offset: usize) -> Option<&Token<'a>> {
        if self.cursor < offset + 1 {
            return None;
        }

        self.data.get(self.cursor - offset - 1)
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

type SubParserOutput<'a> = Result<(ASTNodeKind<'a>, LexerOutputCursor<'a>), ParserError<'a>>;

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
        match token.kind {
            TokenKind::General(tk::LBrace) => parse_block(cursor),
            TokenKind::General(tk::Fn) => parse_function_decl(cursor),
            TokenKind::General(tk::RBrace) => {
                break;
            }
            _ => unimplemented!(),
        }?;
    }
    todo!()
    // Ok((ASTNode::Block { children: vec![] }))
}

enum ExprUnit<'a> {
    Operator,
    Operand(ASTNode<'a>),
}

enum ExprUnitOperator {}

fn parse_expression<'a>(
    mut cursor: LexerOutputCursor<'a>,
    terminator: &[TokenKind],
) -> SubParserOutput<'a> {
    enum ExprParserExpected {
        Operator,
        Operand,
    }
    use ExprParserExpected::*;

    let mut state = Operand;

    while let Some(token) = cursor.next() {
        if terminator.contains(&token.kind) {
            break;
        }

        let s = match token.kind {
            TokenKind::General(g) => {}
            TokenKind::Operator(opertator) => {}
            _ => {}
        };

        // let s = match token.kind {
        //     // TokenKind::LBrace => parse_block(cursor),
        //     // (1 + 2)
        //     TokenKind::LRndBracket => parse_expression(cursor, &[TokenKind::RRndBracket]),

        //     // TokenKind::String => {
        //     //     todo!();
        //     // }

        //     // TokenKind::Number(n) => {
        //     //     todo!();
        //     // }
        //     TokenKind::Ident => {
        //         // fn_name (10, 2)
        //         if let Some(fcall) = try_parse_function_call(cursor) {
        //             fcall
        //         }
        //         // var_name
        //         else {
        //             todo!()
        //             // Ok((ASTNode::VarUse { name: token.word }, cursor))
        //         }
        //     }

        //     // Add all operators
        //     _ => {
        //         todo!();
        //     }
        // };
    }
    todo!();
}

fn try_parse_function_call<'a>(mut cursor: LexerOutputCursor<'a>) -> Option<SubParserOutput<'a>> {
    // Try to get function name and opening left round bracket
    let name_token = cursor.next()?;
    let lrndbr_token = cursor.next()?;

    // Check if name and opening left round bracket was found
    if !(name_token.kind == Gn(Ident) && lrndbr_token.kind == Gn(LRndBracket)) {
        return None;
    }

    let name = name_token.word;

    // Extract arguments, which separated with comma ','
    // let mut args = vec![];
    while let Some(token) = cursor.peek(0) {
        let (expr, new_cursor) = match parse_expression(cursor, &[Gn(Comma), Gn(RRndBracket)]) {
            Ok(ok) => ok,
            Err(e) => return Some(Err(e)),
        };

        cursor = new_cursor;

        if new_cursor
            .peek_prev(0)
            .unwrap_or_else(|| unreachable!())
            .kind
            == Gn(RRndBracket)
        {
            break;
        }
    }

    todo!();
}

fn expect<'a>(
    cursor: &mut LexerOutputCursor<'a>,
    expected: &'static [TokenKind],
) -> Result<Token<'a>, ParserError<'a>> {
    match cursor.next() {
        Some(token) => {
            if expected.contains(&token.kind) {
                Ok(*token)
            } else {
                Err(ParserError::UnexpectedToken {
                    token: *token,
                    expected,
                })
            }
        }

        None => Err(ParserError::EndOfFile { expected }),
    }
}

// fn <name> ( [ <arg_name> : <arg_type>],* ) ? -> <return_type> ? %Block%
fn parse_function_decl<'a>(mut cursor: LexerOutputCursor<'a>) -> SubParserOutput<'a> {
    let name = expect(&mut cursor, &[Gn(Ident)])?.word;
    // Args
    expect(&mut cursor, &[Gn(LRndBracket)])?;
    let mut args = vec![];
    cursor = parse_fields_recursive(cursor, Gn(RRndBracket), &mut args)?;
    expect(&mut cursor, &[Gn(RRndBracket)])?;

    todo!("Impl return type");

    let block_start_token = expect(&mut cursor, &[Gn(LBrace)])?;

    let (block, cursor) = parse_block(cursor)?;

    Ok((
        ASTNodeKind::FnDecl {
            name,
            args,
            return_type: None,
            block: Box::new(ASTNode::new(block_start_token, block)),
        },
        cursor,
    ))
}

// let <name> : <var_type> = %Expr%
fn parse_let_decl<'a>(mut cursor: LexerOutputCursor<'a>) -> SubParserOutput<'a> {
    let name = expect(&mut cursor, &[Gn(Ident)])?.word;
    expect(&mut cursor, &[Gn(Colon)])?;
    let var_type = expect(&mut cursor, &[Gn(Ident)])?.word;
    expect(&mut cursor, TOKEN_KIND_OPERATOR_CATEGORY)?;

    let (expr, new_cursor) = parse_expression(cursor, &[Gn(Semicolon)])?;

    cursor = new_cursor;
    expect(&mut cursor, &[Gn(Semicolon)])?;
    todo!();

    // Ok((
    //     ASTNodeKind::Let {
    //         name,
    //         var_type,
    //         expr: Box::new(expr),
    //     },
    //     cursor,
    // ))
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
        && token.kind == Gn(Comma)
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
    let name = expect(&mut cursor, &[Gn(Ident)])?.word;
    expect(&mut cursor, &[Gn(Colon)])?;
    let var_type = expect(&mut cursor, &[Gn(Ident)])?.word;

    Ok((Field { name, var_type }, cursor))
}

#[cfg(test)]
mod tests {
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
        test_try_parse_field(&[Gn(Ident), Gn(Colon), Gn(Ident)], true);
    }

    #[test]
    fn try_parse_field_incorrect() {
        test_try_parse_field(&[Gn(Ident), Gn(Ident), Gn(Ident)], false);
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
        let end_token_kind = Gn(RRndBracket);
        let to_test = [Gn(Ident), Gn(Colon), Gn(Ident), Gn(Comma), Gn(Comma)];
        test_parse_field_recursive(&to_test, end_token_kind, false, 1);
    }

    #[test]
    fn parse_field_recursive_three_args() {
        let end_token_kind = Gn(RRndBracket);
        let mut to_test = [Gn(Ident), Gn(Colon), Gn(Ident), Gn(Comma)].repeat(3);
        to_test.push(end_token_kind);
        test_parse_field_recursive(&to_test, end_token_kind, true, 3);
    }

    #[test]
    fn parse_field_recursive_three_hundred_args() {
        let end_token_kind = Gn(RRndBracket);
        let mut to_test = [Gn(Ident), Gn(Colon), Gn(Ident), Gn(Comma)].repeat(300);
        to_test.push(end_token_kind);
        test_parse_field_recursive(&to_test, end_token_kind, true, 300);
    }

    #[test]
    fn parse_field_recursive_three_hundred_args_wo_ending() {
        let end_token_kind = Gn(RRndBracket);
        let to_test = [Gn(Ident), Gn(Colon), Gn(Ident), Gn(Comma)].repeat(300);
        test_parse_field_recursive(&to_test, end_token_kind, false, 300);
    }

    #[test]
    fn parse_field_recursive_three_args_wo_comma() {
        let end_token_kind = Gn(RRndBracket);
        let mut to_test = [Gn(Ident), Gn(Colon), Gn(Ident)].repeat(3);
        to_test.push(end_token_kind);
        test_parse_field_recursive(&to_test, end_token_kind, true, 1);
    }
}
