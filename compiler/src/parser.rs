use std::fmt::Pointer;
use std::iter::Peekable;

use crate::{diagnostic::Diagnostic, error::*, lexer::*, token::tk::*, token::*, types::NumWord};
use array_concat::*;

use TokenKind::General as Gn;
use TokenKind::Operator as Op;
use TokenKind::Setter as St;
use TokenKind::Unknown;

#[derive(Debug)]
struct ASTNode<'a> {
    token: Token<'a>,
    kind: ASTNodeKind<'a>,
}

impl<'a> ASTNode<'a> {
    fn new(token: Token<'a>, kind: ASTNodeKind<'a>) -> Self {
        Self { token, kind }
    }
}

#[derive(Debug)]
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
    String,

    Number,

    Let,

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

#[derive(Debug)]
enum UnaryOperator {
    Not,    //  !
    BitInv, //  ~
    Ref,    //  &
    Deref,  //  *
    Neg,    //  -
}

#[derive(Debug)]
enum BinaryOperator {
    // Set,
    Mul,
    Div,
    Mod,

    Add,
    Sub,

    LShift,
    RShift,

    Lt,
    Gt,
    LtEq,
    GtEq,

    Eq,
    NotEq,

    BitAnd,

    BitOr,

    And,

    Or, // TODO: Add '.' to access to struct's fields
}

#[derive(Debug)]
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

#[derive(Debug)]
enum ExprUnit<'a> {
    Operator { token: Token<'a>, op: tk::Operator },
    Operand(ASTNode<'a>),
}

fn parse_expression<'a>(
    mut cursor: LexerOutputCursor<'a>,
    terminator: &[TokenKind],
) -> SubParserOutput<'a> {
    let mut units = vec![];

    while let Some(token) = cursor.next() {
        if terminator.contains(&token.kind) {
            break;
        }

        let unit = match token.kind {
            // Subexpressions, surrounded by round brackets: (1 + 2)
            TokenKind::General(tk::LRndBracket) => {
                let (expr_node, new_cursor) = parse_expression(cursor, &[Gn(tk::RRndBracket)])?;
                cursor = new_cursor;
                ExprUnit::Operand(ASTNode::new(*token, expr_node))
            }

            TokenKind::General(tk::String) => {
                ExprUnit::Operand(ASTNode::new(*token, ASTNodeKind::String))
            }

            TokenKind::General(tk::Number) => {
                ExprUnit::Operand(ASTNode::new(*token, ASTNodeKind::Number))
            }

            TokenKind::General(tk::Ident) => {
                // Function calling: foo(10, 2)
                if let Some(fcall) = try_parse_function_call(cursor) {
                    let (node, new_cursor) = fcall?;
                    cursor = new_cursor;
                    ExprUnit::Operand(ASTNode::new(*token, node))
                // Just variable
                } else {
                    ExprUnit::Operand(ASTNode::new(*token, ASTNodeKind::Let))
                }
            }

            TokenKind::Operator(op) => ExprUnit::Operator { token: *token, op },
            _ => {
                // It's stupid, but concat two const array at compile time is non-trivial
                static A: [TokenKind; 3] = [Gn(tk::LRndBracket), Gn(tk::String), Gn(tk::Number)];
                static EXPECTED: [TokenKind; concat_arrays_size!(TOKEN_KIND_OPERATOR_CATEGORY, A)] =
                    concat_arrays!(TOKEN_KIND_OPERATOR_CATEGORY, A);

                return Err(ParserError::UnexpectedToken {
                    token: *token,
                    expected: &EXPECTED,
                });
            }
        };

        units.push(unit);
    }

    if units.is_empty() {
        // TODO: Impl error: expression is empty
        unimplemented!();
    }

    for u in &units {
        println!("{:?}", u)
    }

    // resolve
    let mut iter = units.into_iter().peekable();

    let out = pratt_parser(&mut iter, 0, 0)?;
    println!("{:#?}", out);
    Ok((out.kind, cursor))
}

// a   +   b   *   c   *   d   +   e
//   1   2   3   4   3   4   1   2

fn pratt_parser<'a>(
    // cursor: &mut impl Iterator<Item = ExprUnit<'a>>,
    cursor: &mut Peekable<impl Iterator<Item = ExprUnit<'a>>>,
    min_bp: u8,
    depth: u16,
) -> Result<ASTNode<'a>, ParserError<'a>> {
    if depth > 500 {
        unimplemented!();
    }

    let lhs = match cursor.next() {
        Some(u) => u,
        None => {
            return Err(ParserError::EndOfFile { expected: None });
        }
    };

    let mut lhs = match lhs {
        ExprUnit::Operand(n) => n,
        ExprUnit::Operator { token, op } => {
            // Unary
            unimplemented!()
        }
    };

    loop {
        // I don't know other way to defeat the borrow checker (it's triggers at &mut cursor)
        let (token, op) = match cursor.peek() {
            Some(ExprUnit::Operator { token, op }) => (*token, *op),
            Some(ExprUnit::Operand(node)) => {
                return Err(ParserError::UnexpectedToken {
                    token: node.token,
                    expected: &TOKEN_KIND_OPERATOR_CATEGORY,
                });
            }
            None => break,
        };

        let (l_bp, r_bp, bin_op) = binary_op_power(op, token)?;
        if l_bp < min_bp {
            break;
        }

        cursor.next();

        let rhs = pratt_parser(cursor, r_bp, depth + 1)?;

        lhs = ASTNode::new(
            token,
            ASTNodeKind::BinaryOperator {
                op: bin_op,
                left: Box::new(lhs),
                right: Box::new(rhs),
            },
        )
    }

    Ok(lhs)
}

fn unary_op_power<'a>(
    operator: Operator,
    token: Token<'a>,
) -> Result<(u8, UnaryOperator), ParserError<'a>> {
    use Operator as op;
    use UnaryOperator as uo;

    let ok = match operator {
        op::Not => (30, uo::Not),
        op::BitInv => (30, uo::BitInv),
        op::Ampersand => (30, uo::Ref),
        op::Star => (30, uo::Deref),
        op::Minus => (30, uo::Neg),

        _ => {
            return Err(ParserError::UnexpectedToken {
                token,
                expected: &[
                    Op(op::Not),
                    Op(op::BitInv),
                    Op(op::Ampersand),
                    Op(op::Star),
                    Op(op::Minus),
                ],
            });
        }
    };

    Ok(ok)
}

fn binary_op_power<'a>(
    operator: Operator,
    token: Token<'a>,
) -> Result<(u8, u8, BinaryOperator), ParserError<'a>> {
    use BinaryOperator as bo;
    use Operator as op;

    let ok = match operator {
        op::Star => (130, 131, bo::Mul),
        op::Slash => (130, 131, bo::Div),
        op::Mod => (130, 131, bo::Mod),

        op::Plus => (120, 121, bo::Add),
        op::Minus => (120, 121, bo::Sub),

        op::LShift => (110, 111, bo::LShift),
        op::RShift => (110, 111, bo::RShift),

        op::Lt => (100, 101, bo::Lt),
        op::Gt => (100, 101, bo::Gt),
        op::LtEq => (100, 101, bo::LtEq),
        op::GtEq => (100, 101, bo::GtEq),

        op::Eq => (90, 91, bo::Eq),
        op::NotEq => (90, 91, bo::NotEq),

        op::Ampersand => (80, 81, bo::BitAnd),

        op::Bar => (70, 71, bo::BitOr),

        op::And => (60, 61, bo::And),

        op::Or => (50, 51, bo::Or),

        op::Not | op::BitInv => {
            return Err(ParserError::UnexpectedToken {
                token,
                expected: &[
                    Op(op::Star),
                    Op(op::Slash),
                    Op(op::Mod),
                    Op(op::Plus),
                    Op(op::Minus),
                    Op(op::LShift),
                    Op(op::RShift),
                    Op(op::Lt),
                    Op(op::Gt),
                    Op(op::LtEq),
                    Op(op::GtEq),
                    Op(op::Eq),
                    Op(op::NotEq),
                    Op(op::Ampersand),
                    Op(op::Bar),
                    Op(op::And),
                    Op(op::Or),
                ],
            });
        }
    };

    Ok(ok)
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

        None => Err(ParserError::EndOfFile {
            expected: Some(expected),
        }),
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
    expect(&mut cursor, &TOKEN_KIND_OPERATOR_CATEGORY)?;

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

    // Expressions
    fn test_parse_expression(to_test: &[TokenKind]) {
        let tokens = MockedTokens::new(to_test);

        let result = parse_expression(tokens.cursor(), &[Gn(tk::Semicolon)]);
    }

    #[test]
    fn parse_expression_success() {
        // 1 + 2 * A
        // +
        //   1
        //   *
        //     2
        //     A
        test_parse_expression(&[
            Gn(Number),
            Op(Plus),
            Gn(Number),
            Op(Star),
            Gn(Ident),
            Op(Plus),
            Gn(LRndBracket),
            Gn(Ident),
            Op(Minus),
            Gn(Ident),
            Gn(RRndBracket),
        ]);
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
