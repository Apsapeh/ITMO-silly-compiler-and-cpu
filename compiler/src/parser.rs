use crate::error::error;
use crate::protolexer;

#[derive(Debug, Clone)]
pub enum ASTNode {
    Block {
        children: Vec<ASTNode>,
    },

    Procedure {
        name: String,
        args: Vec<Argument>,
        block: Box<ASTNode>,
    },

    Function {
        name: String,
        args: Vec<Argument>,
        rtype: VarType,
        block: Box<ASTNode>,
    },

    If {
        expr: Expression,
        block: Box<ASTNode>,
        else_block: Option<Box<ASTNode>>,
    },

    Loop {
        block: Box<ASTNode>,
    },

    FnCall(FnCall),

    Variable {
        name: String,
        vtype: VarType,
    },

    VariableSet {
        var: VariableUse,
        expr: Expression,
    },

    Return {
        expr: Option<Expression>,
    },

    Stop,
}

#[derive(Debug, Clone)]
pub enum VarType {
    Word,  // 16 bit
    DWord, // 32 bit
    Ptr { vtype: Box<VarType>, size: u32 },
}

impl VarType {
    pub fn from_raw(raw: &[String]) -> Result<Self, String> {
        // Just type
        if raw.len() == 1 {
            match raw[0].as_str() {
                "WORD" => Ok(Self::Word),
                "DWORD" => Ok(Self::DWord),
                _ => Err(format!("Syntax error - Unknown type '{}'", raw.join(" "))),
            }
        } else {
            // []Type - Pointer
            let (vtype, size) = if raw.len() == 3 && raw[0] == "[" && raw[1] == "]" {
                (Self::from_raw(&raw[2..])?, 0)
            // [N]Type - Array
            } else if raw.len() == 4 && raw[0] == "[" && raw[2] == "]" {
                (Self::from_raw(&raw[3..])?, raw[1].parse::<u32>().unwrap())
            } else {
                return Err(format!("Syntax error - Unknown type '{}'", raw.join(" ")));
            };

            Ok(Self::Ptr {
                vtype: Box::new(vtype),
                size,
            })
        }
    }
}

type LinesIter<'a> = std::iter::Peekable<std::slice::Iter<'a, protolexer::SourceLineWords>>;
type WordsIter<'a> = std::iter::Peekable<std::slice::Iter<'a, String>>;
type SubParserResult = Result<ASTNode, String>;

pub fn parse(lines: Vec<protolexer::SourceLineWords>) -> Vec<ASTNode> {
    let mut root = vec![];
    let mut iter = lines.iter().peekable();
    while let Some(line) = iter.peek() {
        let node = match line.words[0].as_str() {
            "FUNCTION" => parse_function(&mut iter),
            "PROCEDURE" => parse_procedure(&mut iter),
            _ => error(
                "Syntax error - unexpected construction. Only PROCEDURE and FUNCTION allowed at zero level",
                &line.source_line,
            ),
        };

        match node {
            Ok(n) => root.push(n),
            Err(msg) => {
                // Very hacky way to get a real error line
                let el = match iter.next() {
                    Some(next_el) => {
                        let line_num = next_el.source_line.line_number;
                        let el = lines
                            .iter()
                            .rev()
                            .find(|e| e.source_line.line_number < line_num);

                        match el {
                            Some(el) => el,
                            None => lines.first().unwrap(),
                        }
                    }
                    None => lines.last().unwrap(),
                };

                error(msg.as_str(), &el.source_line)
            }
        }
    }

    // println!("{:#?}", root);
    root
}

fn parse_block(iter: &mut LinesIter, exit_depth: usize) -> SubParserResult {
    let mut children = vec![];
    while let Some(line) = iter.peek() {
        if line.source_line.power <= exit_depth {
            break;
        }

        #[rustfmt::skip]
        let node = match line.words[0].as_str() {
            "IF"       => parse_if(iter),
            "LOOP"     => parse_loop(iter),
            "VARIABLE" => parse_variable(iter),
            "CALL"     => parse_fn_call(iter),
            "STOP"     => parse_stop(iter),
            "RETURN"   => parse_return(iter),
            _          => parse_variable_set(iter),
        };

        children.push(node?);
    }

    Ok(ASTNode::Block { children })
}

fn try_get_word(
    idx: usize,
    line: &protolexer::SourceLineWords,
    err: &str,
) -> Result<String, String> {
    match line.words.get(idx) {
        Some(w) => Ok(w.clone()),
        None => Err(String::from(err)),
    }
}

/* =============================> Zero level Syntax parsers <==================================== */

fn parse_function(iter: &mut LinesIter) -> SubParserResult {
    let line = iter.next().unwrap();
    let words = &line.words;

    let name = try_get_word(1, line, "Syntax error - name of the function not found")?;

    let return_idx = words.iter().position(|w| w == "RETURN").unwrap();
    let args = Argument::from_raw_vec(&words[2..return_idx])?;

    let rtype = VarType::from_raw(&words[return_idx + 1..])?;

    let block = parse_block(iter, line.source_line.power)?;

    Ok(ASTNode::Function {
        name,
        args,
        rtype,
        block: Box::new(block),
    })
}

fn parse_procedure(iter: &mut LinesIter) -> SubParserResult {
    let line = iter.next().unwrap();
    let words = &line.words;

    let name = try_get_word(1, line, "Syntax error - name of the function not found")?;

    let args = Argument::from_raw_vec(&words[2..])?;

    let block = parse_block(iter, line.source_line.power)?;

    Ok(ASTNode::Procedure {
        name,
        args,
        block: Box::new(block),
    })
}

/* =============================> Block level Syntax parsers <==================================== */

fn parse_if(iter: &mut LinesIter) -> SubParserResult {
    let line = iter.next().unwrap();
    let block = parse_block(iter, line.source_line.power)?;

    let mut line_iter = line.words.iter().peekable();
    line_iter.next();
    let expr = Expression::iter_parse(&mut line_iter, &[])?;

    let else_block = if let Some(else_line) =
        iter.next_if(|l| l.words.first().map(|s| s.as_str()) == Some("ELSE"))
    {
        Some(Box::new(parse_block(iter, else_line.source_line.power)?))
    } else {
        None
    };

    Ok(ASTNode::If {
        expr,
        block: Box::new(block),
        else_block,
    })
}

fn parse_loop(iter: &mut LinesIter) -> SubParserResult {
    let line = iter.next().unwrap();
    let block = parse_block(iter, line.source_line.power)?;

    Ok(ASTNode::Loop {
        block: Box::new(block),
    })
}

fn parse_fn_call(iter: &mut LinesIter) -> SubParserResult {
    let line = iter.next().unwrap();
    let words = &line.words;
    Ok(ASTNode::FnCall(FnCall::from_raw(&words[1..])?))
}

fn parse_variable(iter: &mut LinesIter) -> SubParserResult {
    let arg = Argument::from_raw(&iter.next().unwrap().words[1..])?;
    Ok(ASTNode::Variable {
        name: arg.name,
        vtype: arg.vtype,
    })
}

fn parse_variable_set(iter: &mut LinesIter) -> SubParserResult {
    let line = iter.next().unwrap();
    let words = &line.words;
    let mut words_iter = words.iter().peekable();

    let var = match words_iter.next().map(|s| s.as_str()) {
        Some("[") => VariableUse::iter_parse_arr_deref(&mut words_iter)?,
        Some(name) => VariableUse::new_just_var(name.to_string()),
        None => return Err("Syntax error - expected variable name".into()),
    };

    // Check next symbol is '='
    words_iter
        .next()
        .filter(|&s| s == "=")
        .ok_or_else(|| "Syntax error - expected '=' sign.".to_string())?;

    let expr = Expression::iter_parse(&mut words_iter, &[])?;

    Ok(ASTNode::VariableSet { var, expr })
}

fn parse_return(iter: &mut LinesIter) -> SubParserResult {
    let line = iter.next().unwrap();
    let words = &line.words;
    let mut expr_iter = words.iter().peekable();
    expr_iter.next(); // Skip "RETURN"

    let expr = match expr_iter.peek() {
        Some(_) => Some(Expression::iter_parse(&mut expr_iter, &[])?),
        None => None,
    };

    Ok(ASTNode::Return { expr })
}

fn parse_stop(iter: &mut LinesIter) -> SubParserResult {
    iter.next();
    Ok(ASTNode::Stop)
}

/* ===================================> Expression parser <======================================= */
#[derive(Debug, Clone)]
pub enum Operator {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    And,
    Or,
}

impl Operator {
    #[rustfmt::skip]
    pub fn from_raw_with_power(raw: &str) -> Option<(Self, u8)> {
        match raw {
            "MUL"  => Some(( Self::Mul,   50 )),
            "DIV"  => Some(( Self::Div,   50 )),
            "MOD"  => Some(( Self::Mod,   50 )),
            "ADD"  => Some(( Self::Add,   40 )),
            "SUB"  => Some(( Self::Sub,   40 )),
            "LT"   => Some(( Self::Lt,    30 )),
            "GT"   => Some(( Self::Gt,    30 )),
            "LTEQ" => Some(( Self::LtEq,  30 )),
            "GTEQ" => Some(( Self::GtEq,  30 )),
            "EQ"   => Some(( Self::Eq,    20 )),
            "NEQ"  => Some(( Self::NotEq, 20 )),
            "AND"  => Some(( Self::And,   10 )),
            "OR"   => Some(( Self::Or,    0) ),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Expression {
    Number(i32),
    CString(usize),
    Variable(VariableUse),
    FnCall(FnCall),
    Expr(Box<Self>),
    BinaryOp {
        op: Operator,
        left: Box<Self>,
        right: Box<Self>,
    },
}

// Temp struct for pratt parser
#[derive(Debug)]
enum ExprUnit {
    Operator((Operator, u8)),
    Operand(Expression),
}

impl Expression {
    pub fn iter_parse(iter: &mut WordsIter, terminators: &[&str]) -> Result<Self, String> {
        // Flatten expression 1 + (2 * call aboba(1, 1)) -> 1 + Expr
        let mut units = vec![];
        while let Some(&word) = iter.peek() {
            if terminators.contains(&word.as_str()) {
                break;
            }

            // Consume word
            iter.next();

            let unit = if word == "CALL" {
                ExprUnit::Operand(Self::FnCall(FnCall::iter_parse(iter)?))
            } else if word == "[" {
                ExprUnit::Operand(Self::Variable(VariableUse::iter_parse_arr_deref(iter)?))
            } else if word == "(" {
                ExprUnit::Operand(Self::Expr(Box::new(Expression::iter_parse(iter, &[")"])?)))
            } else if word.starts_with("\"") {
                let str_num = word.strip_prefix("\"").unwrap().parse::<usize>().unwrap();
                ExprUnit::Operand(Self::CString(str_num))
            } else {
                if let Some(op) = Operator::from_raw_with_power(word) {
                    ExprUnit::Operator(op)
                } else if let Ok(n) = word.parse::<i32>() {
                    ExprUnit::Operand(Self::Number(n))
                } else {
                    ExprUnit::Operand(Self::Variable(VariableUse::new_just_var(String::from(
                        word,
                    ))))
                }
            };

            units.push(unit);
        }

        let mut pratt_iter = units.iter().peekable();
        Self::pratt_parser(&mut pratt_iter, 0)
    }

    /*
        Many thx this tutor: https://matklad.github.io/2020/04/13/simple-but-powerful-pratt-parsing.html
    */
    fn pratt_parser(
        iter: &mut std::iter::Peekable<std::slice::Iter<'_, ExprUnit>>,
        min_bp: u8,
    ) -> Result<Self, String> {
        let lhs = match iter.next() {
            Some(u) => u,
            None => {
                return Err(String::from(
                    "Syntax error - unexpected end of file. Expected operand",
                ));
            }
        };

        let mut lhs = match lhs {
            ExprUnit::Operand(n) => n.clone(),
            ExprUnit::Operator(o) => {
                return Err(format!(
                    "Syntax error - unexpected operator ({:?}), expected operand",
                    o.0
                ));
            }
        };

        loop {
            let (op, l_bp, r_bp) = match iter.peek() {
                Some(ExprUnit::Operator((op, l_bp))) => (op.clone(), *l_bp, l_bp + 1),
                Some(ExprUnit::Operand(o)) => {
                    return Err(format!(
                        "Syntax error - unexpected operand ({:?}), expected operator",
                        o
                    ));
                }
                None => break,
            };

            if l_bp < min_bp {
                break;
            }

            iter.next();

            let rhs = Self::pratt_parser(iter, r_bp)?;

            lhs = Expression::BinaryOp {
                left: Box::new(lhs),
                op,
                right: Box::new(rhs),
            }
        }

        Ok(lhs)
    }
}

/* ===============================> Other (shared) parsers and types <============================== */

#[derive(Debug, Clone)]
pub struct Argument {
    name: String,
    vtype: VarType,
}

impl Argument {
    // Var decl or func/proc decl
    // N WORD
    // N [3]WORD
    pub fn from_raw(raw: &[String]) -> Result<Self, String> {
        let name = match raw.first() {
            Some(n) => n.clone(),
            None => {
                return Err(String::from("Syntax error - variable name not found"));
            }
        };

        let vtype = VarType::from_raw(&raw[1..raw.len()])?;

        Ok(Self { name, vtype })
    }

    pub fn from_raw_vec(raw: &[String]) -> Result<Vec<Self>, String> {
        if raw.is_empty() {
            return Ok(vec![]);
        }
        raw.split(|w| w == ",").map(Self::from_raw).collect()
    }
}

#[derive(Debug, Clone)]
pub struct FnCall {
    name: String,
    args: Vec<Expression>,
}

impl FnCall {
    pub fn iter_parse(iter: &mut WordsIter) -> Result<Self, String> {
        let name = match iter.next() {
            Some(n) => n,
            None => {
                return Err(String::from("Syntax error - function name not found"));
            }
        };

        let mut args = vec![];
        if let Some(&rnd_beg) = iter.peek()
            && rnd_beg == "("
        {
            iter.next();
            loop {
                let arg = Expression::iter_parse(iter, &[",", ")"])?;
                args.push(arg);

                let sep = match iter.next() {
                    Some(s) => s,
                    None => {
                        return Err(String::from(
                            "Syntax error - unexpected end of line. Function call isn't closed",
                        ));
                    }
                };

                match sep.as_str() {
                    "," => {
                        continue;
                    }
                    ")" => {
                        break;
                    }
                    _ => return Err(format!("Syntax error - unexpected sumbol '{}'", sep)),
                }
            }
        }

        Ok(Self {
            name: name.clone(),
            args,
        })
    }

    pub fn from_raw(raw: &[String]) -> Result<Self, String> {
        let mut iter = raw.iter().peekable();
        Self::iter_parse(&mut iter)
    }
}

#[derive(Debug, Clone)]
pub struct VariableUse {
    pub name: String,
    pub deref_offset: Option<Box<Expression>>,
}

impl VariableUse {
    pub fn new_just_var(name: String) -> Self {
        Self {
            name,
            deref_offset: None,
        }
    }

    // Parse [offset_expr]array
    // [0]arr
    // [i + 2]arr
    pub fn iter_parse_arr_deref(iter: &mut WordsIter) -> Result<Self, String> {
        let offset = Expression::iter_parse(iter, &["]"])?;
        let name = match iter.nth(1) {
            Some(n) => n,
            None => {
                return Err(String::from("Syntax error - variable name not found"));
            }
        };

        Ok(Self {
            name: name.clone(),
            deref_offset: Some(Box::new(offset)),
        })
    }
}
