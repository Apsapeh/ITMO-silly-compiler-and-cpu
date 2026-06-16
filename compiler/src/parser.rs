use crate::error::error;
use crate::protolexer;

#[derive(Debug, Clone)]
pub enum ASTNode {
    Block {
        children: Vec<ASTNode>,
    },

    Procedure {
        name: String,
        args: Vec<String>,
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

    FnCall {
        name: String,
        args: Vec<Expression>,
    },

    Variable {
        name: String,
        vtype: VarType,
    },

    VariableSet {
        var: VariableUse,
        expr: Expression,
    },

    Return,

    Stop,
}

// impl std::fmt::Display for ASTNode {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(f, "")
//     }
// }

impl ASTNode {
    pub fn to_termtree(&self) -> termtree::Tree<String> {
        let (root_str, leaves) = match self {
            Self::Block { children } => {
                let children = children.iter().map(|k| k.to_termtree()).collect::<Vec<_>>();
                ("Block".to_string(), children)
            }
            Self::Procedure { name, args, block } => {
                let mut tree_args = termtree::Tree::new("Args".to_string());
                args.iter().for_each(|a| {
                    tree_args.push(termtree::Tree::new(a.clone()));
                });
                let body = block.to_termtree();
                (format!("Procedure ({})", name), vec![tree_args, body])
            }

            Self::If {
                expr,
                block,
                else_block,
            } => {
                let mut v = vec![];
                let mut cond = termtree::Tree::new("Condition".to_string());
                cond.push(expr.to_termtree());
                v.push(cond);
                v.push(block.to_termtree());
                if let Some(e) = else_block {
                    let mut else_block = e.to_termtree();
                    else_block.root = "Else".to_string();
                    v.push(else_block);
                }
                ("If".to_string(), v)
            }

            Self::Loop { block } => ("Loop".to_string(), vec![block.to_termtree()]),

            Self::FnCall { name, args } => {
                let mut v = vec![];
                args.iter().for_each(|a| {
                    v.push(a.to_termtree());
                });
                (format!("Call ({})", name.clone()), v)
            }

            Self::Variable { name, vtype } => {
                (format!("Variable: {}", name), vec![vtype.to_termtree()])
            }

            Self::VariableSet { var, expr } => {
                let mut expr_t = termtree::Tree::new("Expression".to_string());
                expr_t.push(expr.to_termtree());
                ("Set".to_string(), vec![var.to_termtree(), expr_t])
            }

            Self::Return => ("Return".to_string(), vec![]),

            Self::Stop => ("Stop".to_string(), vec![]),
        };

        let mut root = termtree::Tree::new(root_str);
        for leave in leaves {
            root.push(leave);
        }

        root
    }
}

#[derive(Debug, Clone)]
pub enum VarType {
    Word, // 16 bit
    Array(u16),
}

impl VarType {
    pub fn get_size_in_words(&self) -> u16 {
        match self {
            Self::Word => 1,
            Self::Array(size) => {
                // Ptr (Word) + Data
                1 + size
            }
        }
    }

    pub fn to_termtree(&self) -> termtree::Tree<String> {
        let root_str = match self {
            Self::Word => "WORD".to_string(),
            Self::Array(size) => format!("ARRAY ({})", size),
        };
        termtree::Tree::new(format!("Type: {}", root_str))
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
            "PROCEDURE" => parse_procedure(&mut iter),
            "VARIABLE" => parse_variable(&mut iter),
            "ARRAY" => parse_array(&mut iter),
            _ => error(
                "Syntax error - unexpected construction. Only PROCEDURE, VARIABLE or ARRAY are allowed at zero level",
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
            "ARRAY" => parse_array(iter),
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
fn parse_procedure(iter: &mut LinesIter) -> SubParserResult {
    let line = iter.next().unwrap();
    let words = &line.words;

    let name = try_get_word(1, line, "Syntax error - name of the procedure not found")?;

    let args = Vec::from(&words[2..]);

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
    let mut words_iter = (words[1..]).iter().peekable();

    let name = match words_iter.next() {
        Some(n) => n,
        None => {
            return Err(String::from("Syntax error - function name not found"));
        }
    };

    let mut args = vec![];
    if words_iter.peek().is_some() {
        loop {
            let arg = Expression::iter_parse(&mut words_iter, &[","])?;
            args.push(arg);

            let sep = match words_iter.next() {
                Some(s) => s,
                None => break,
            };

            match sep.as_str() {
                "," => {
                    continue;
                }
                _ => return Err(format!("Syntax error - unexpected sumbol '{}'", sep)),
            }
        }
    }

    // Ok(ASTNode::FnCall(FnCall::from_raw(&words[1..])?))
    Ok(ASTNode::FnCall {
        name: name.clone(),
        args,
    })
}

fn parse_variable(iter: &mut LinesIter) -> SubParserResult {
    let line = iter.next().unwrap();
    let name = try_get_word(1, line, "Syntax error - name of the variable not found")?;
    Ok(ASTNode::Variable {
        name,
        vtype: VarType::Word,
    })
}

fn parse_array(iter: &mut LinesIter) -> SubParserResult {
    let line = iter.next().unwrap();

    let name = try_get_word(1, line, "Syntax error - name of the array not found")?;
    let size = try_get_word(2, line, "Syntax error - name of the array not found")?;
    let size = match size.parse::<u16>() {
        Ok(n) => n,
        Err(e) => {
            return Err(format!(
                "Syntax error - array size parsing error. It must be unsigned number\n{}",
                e
            ));
        }
    };

    if size == 0 {
        return Err("Error - array size must be greater than zero".to_string());
    }

    Ok(ASTNode::Variable {
        name,
        vtype: VarType::Array(size),
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
    iter.next();
    Ok(ASTNode::Return)
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

    pub fn to_termtree(&self) -> termtree::Tree<String> {
        let str = match self {
            Self::Add => "+",
            Self::Sub => "-",
            Self::Mul => "*",
            Self::Div => "/",
            Self::Mod => "%",
            Self::Eq => "==",
            Self::NotEq => "!=",
            Self::Lt => "<",
            Self::Gt => ">",
            Self::LtEq => ">=",
            Self::GtEq => "<=",
            Self::And => "&&",
            Self::Or => "||",
        };

        termtree::Tree::new(str.to_string())
    }
}

#[derive(Debug, Clone)]
pub enum Expression {
    Number(i32),
    CString(usize),
    Variable(VariableUse),
    VariableAddr(String),
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

            let unit = if word == "REF" {
                ExprUnit::Operand(Self::VariableAddr(
                    iter.next()
                        .expect("Variable name to ref not specified")
                        .clone(),
                ))
            } else if word == "[" {
                ExprUnit::Operand(Self::Variable(VariableUse::iter_parse_arr_deref(iter)?))
            } else if word == "(" {
                let u =
                    ExprUnit::Operand(Self::Expr(Box::new(Expression::iter_parse(iter, &[")"])?)));
                iter.next(); // Skip ")"
                u
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

    pub fn to_termtree(&self) -> termtree::Tree<String> {
        match self {
            Self::Number(n) => termtree::Tree::new(format!("Number: {}", n)),
            Self::CString(c) => termtree::Tree::new(format!("String: {}", c)),
            Self::Variable(v) => v.to_termtree(),
            Self::VariableAddr(va) => termtree::Tree::new(format!("Address of Var: {}", va)),
            Self::Expr(e) => e.to_termtree(),
            Self::BinaryOp { op, left, right } => {
                let mut op = op.to_termtree();
                op.push(left.to_termtree());
                op.push(right.to_termtree());
                op
            }
        }
    }
}

/* ===============================> Other (shared) parsers and types <============================== */

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

    pub fn to_termtree(&self) -> termtree::Tree<String> {
        let mut root = termtree::Tree::new(format!("Variable: {}", self.name));
        if let Some(of) = &self.deref_offset {
            let mut r = termtree::Tree::new("Deref offset".to_string());
            r.push(of.to_termtree());
            root.push(r);
        }
        root
    }
}
