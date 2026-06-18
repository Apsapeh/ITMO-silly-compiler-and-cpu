use isa::Mode::*;
use isa::Opcode::*;
use isa::Register::*;
use isa::*;
use std::collections::HashMap;

use crate::parser::ASTNode;
use crate::parser::Expression;
use crate::parser::Operator;
use crate::parser::VarType;
use crate::parser::VariableUse;

#[derive(Debug, Clone)]
pub enum Command {
    Word(u16),
    Label(String),
}

#[derive(Debug, Clone)]
pub struct Codegen {
    pub commands: Vec<Command>,
    pub labels: HashMap<String, usize>,
    label_counter: usize,

    //
    current_loop_id_stack: Vec<usize>,
    current_procedure: Option<(String, Vec<String>)>,
}

impl Codegen {
    pub fn new(ast: Vec<ASTNode>, string_register: Vec<String>) -> Self {
        let mut code = Self {
            commands: vec![],
            labels: HashMap::new(),
            label_counter: 0,
            current_loop_id_stack: vec![],
            current_procedure: None,
        };

        code.add_ast(ast, string_register);
        code
    }

    pub fn new_from_asm(src: String) -> Self {
        let mut code = Self {
            commands: vec![],
            labels: HashMap::new(),
            label_counter: 0,
            current_loop_id_stack: vec![],
            current_procedure: None,
        };

        code.add_asm_src(src);
        code
    }

    fn add_ast(&mut self, ast: Vec<ASTNode>, string_register: Vec<String>) {
        let mut global_variables = vec![];

        for node in ast {
            match node {
                ASTNode::Procedure { name, args, block } => {
                    self.gen_procedure(name, args, *block);
                }

                ASTNode::Variable { name, vtype } => {
                    // It makes no sense, but just collect all global variables at one place
                    global_variables.push((name, vtype));
                }
                _ => unreachable!(),
            }
        }

        for (name, vtype) in global_variables {
            self.set_label(&name);

            match vtype {
                VarType::Word => {
                    self.emit_word(0);
                }

                VarType::Array(size) => {
                    for _ in 0..size {
                        self.emit_word(0);
                    }
                }
            }
        }

        for (i, s) in string_register.iter().enumerate() {
            self.set_label(&format!(".str {}", i));
            for b in s.encode_utf16() {
                self.emit_word(b);
            }
            // null-terminator
            self.emit_word(0);
        }
    }

    pub fn add_asm_src(&mut self, src: String) {
        for (i, line) in src.lines().enumerate() {
            if line.is_empty() {
                continue;
            }

            let words = line.split_whitespace().collect::<Vec<_>>();

            // Comment
            if !words.is_empty() && words[0] == ";" {
                continue;
            }

            if words.len() >= 2 && words[0] == ":" {
                self.set_label(&words[1].to_uppercase());
            } else if words.len() >= 2 && words[0] == "#" {
                self.emit_word(words[1].parse::<i32>().unwrap() as u16);
            } else if words.len() >= 2 && words[0] == "@" {
                self.emit_label(&words[1].to_uppercase());
            } else if words.len() >= 4 {
                let opcode = Opcode::frow_raw_str(words[0])
                    .unwrap_or_else(|| panic!("Unexpected opcode at {} '{}'", i, words[0]));
                let mode = Mode::frow_raw_str(words[1])
                    .unwrap_or_else(|| panic!("Unexpected mode at {} '{}'", i, words[1]));
                let rd = Register::frow_raw_str(words[2])
                    .unwrap_or_else(|| panic!("Unexpected register rd at {} '{}'", i, words[2]));
                let rs = Register::frow_raw_str(words[3])
                    .unwrap_or_else(|| panic!("Unexpected register rs at {}  '{}'", i, words[3]));
                self.emit_instr(opcode, mode, rd, rs);
            }
        }
    }

    fn gen_block(
        &mut self,
        block: ASTNode,
        mut var_map: HashMap<String, u16>,
        stack_size: &mut u16,
    ) {
        let children = if let ASTNode::Block { children } = block {
            children
        } else {
            unreachable!("Codegen error: Expeced ASTNode::Block")
        };

        for ch in children {
            match ch {
                ASTNode::If {
                    expr,
                    block,
                    else_block,
                } => {
                    self.gen_if(expr, *block, else_block, var_map.clone(), stack_size);
                }

                ASTNode::Loop { block } => {
                    self.gen_loop(*block, var_map.clone(), stack_size);
                }

                ASTNode::FnCall { name, args } => {
                    self.gen_fncall(name, args, var_map.clone());
                }

                ASTNode::Variable { name, vtype } => {
                    let off = match &vtype {
                        VarType::Word => {
                            // Init var on stack with 0
                            let bp_off = 0u16.overflowing_sub(*stack_size + 1);
                            self.emit_instr(Mov, ImmToMemra, BP, BP);
                            self.emit_word(bp_off.0);
                            self.emit_word(0);
                            *stack_size
                        }

                        VarType::Array(s) => {
                            let bp_off = 0u16.overflowing_sub(*stack_size + s + 1);
                            self.emit_instr(Mov, RegToMemra, BP, BP);
                            self.emit_word(bp_off.0);
                            let bp_data_off = 0u16.overflowing_sub(*stack_size + s);
                            self.emit_instr(Sub, ImmToMemra, BP, BP);
                            self.emit_word(bp_off.0);
                            self.emit_word(bp_data_off.0);
                            *stack_size + s
                        }
                    };

                    var_map.insert(name, off);
                    *stack_size += vtype.get_size_in_words();
                }

                ASTNode::VariableSet { var, expr } => {
                    self.gen_variable_set(var, expr, var_map.clone());
                }

                ASTNode::Return => {
                    self.gen_return();
                }

                ASTNode::Stop => {
                    self.gen_stop();
                }

                ASTNode::Block { .. } | ASTNode::Procedure { .. } => {
                    unreachable!()
                }
            }
        }
    }

    fn gen_procedure(&mut self, name: String, args: Vec<String>, block: ASTNode) {
        self.current_procedure = Some((name.clone(), args.clone()));
        self.set_label(&name);

        self.emit_instr(Enter, ImmToReg, AL, AL);
        self.emit_word(0);
        let frame_size_addr = self.commands.len() - 1;

        // Store Callee-save registers
        self.emit_instr(Push, RegToReg, AL, AL);
        self.emit_instr(Push, RegToReg, AL, AH);
        self.emit_instr(Push, RegToReg, AL, BL);
        self.emit_instr(Push, RegToReg, AL, BH);
        self.emit_instr(Push, RegToReg, AL, CL);
        self.emit_instr(Push, RegToReg, AL, CH);

        let var_map = HashMap::<String, u16>::new();
        let mut stack_size = 0;

        self.gen_block(block, var_map, &mut stack_size);

        self.commands[frame_size_addr] = Command::Word(stack_size);

        self.set_label(&format!("{} .out", name));

        // Restore Callee-save registers
        self.emit_instr(Pop, RegToReg, CH, AL);
        self.emit_instr(Pop, RegToReg, CL, AL);
        self.emit_instr(Pop, RegToReg, BH, AL);
        self.emit_instr(Pop, RegToReg, BL, AL);
        self.emit_instr(Pop, RegToReg, AH, AL);
        self.emit_instr(Pop, RegToReg, AL, AL);

        self.emit_instr(Leave, RegToReg, AL, AL);

        if args.is_empty() {
            self.emit_instr(Ret, RegToReg, AL, AL);
        } else {
            self.emit_instr(Ret, ImmToReg, AL, AL);
            self.emit_word(args.len() as u16);
        }
    }

    fn gen_if(
        &mut self,
        expr: Expression,
        block: ASTNode,
        else_block: Option<Box<ASTNode>>,
        var_map: HashMap<String, u16>,
        stack_size: &mut u16,
    ) {
        let l_id = self.next_label_id();

        let else_lbl = format!(".if else {}", l_id);
        let end_lbl = format!(".if end {}", l_id);

        self.gen_expr(expr, var_map.clone());
        self.emit_instr(Cmp, ImmToReg, AL, AL);
        self.emit_word(0);

        self.emit_instr(Je, ImmToReg, AL, AL);
        self.emit_label(if else_block.is_some() {
            &else_lbl
        } else {
            &end_lbl
        });

        self.gen_block(block, var_map.clone(), stack_size);

        if let Some(else_block) = else_block {
            self.emit_instr(Jmp, ImmToReg, AL, AL);
            self.emit_label(&end_lbl);

            self.set_label(&else_lbl);
            self.gen_block(*else_block, var_map, stack_size);
        }

        self.set_label(&end_lbl);
    }

    fn gen_loop(&mut self, block: ASTNode, var_map: HashMap<String, u16>, stack_size: &mut u16) {
        let l_id = self.next_label_id();
        let lbl_start = format!(".loop {}", l_id);
        self.set_label(&lbl_start);
        self.current_loop_id_stack.push(l_id);

        self.gen_block(block, var_map, stack_size);

        self.current_loop_id_stack.pop();
        self.emit_instr(Jmp, ImmToReg, AL, AL);
        self.emit_label(&lbl_start);
        self.set_label(&format!(".loop {} out", l_id));
    }

    fn gen_fncall(&mut self, name: String, args: Vec<Expression>, var_map: HashMap<String, u16>) {
        for a in args {
            self.gen_expr(a, var_map.clone());
            self.emit_instr(Push, RegToReg, AL, AL);
        }

        self.emit_instr(Call, ImmToReg, AL, AL);
        self.emit_label(&name);
    }

    fn gen_variable_set(
        &mut self,
        var: VariableUse,
        expr: Expression,
        var_map: HashMap<String, u16>,
    ) {
        let (_, args) = self.current_procedure.clone().unwrap();

        if let Some(off) = var.deref_offset {
            // off -> AL; Push(AL)
            self.gen_expr(*off, var_map.clone());
            self.emit_instr(Push, RegToReg, AL, AL);

            // Stack
            if let Some(&offset) = var_map.get(&var.name) {
                self.emit_instr(Mov, MemraToReg, AL, BP);
                self.emit_word(0u16.overflowing_sub(offset + 1).0);
            // Arg
            } else if let Some(index) = args.iter().position(|arg| arg == &var.name) {
                self.emit_instr(Mov, MemraToReg, AL, BP);
                self.emit_word((args.len() - index + 1) as u16);
            // Global
            } else {
                self.emit_instr(Mov, MemaToReg, AL, AL);
                self.emit_label(&var.name);
            }

            self.emit_instr(Pop, RegToReg, BL, AL);
            self.emit_instr(Add, RegToReg, AL, BL);
            self.emit_instr(Push, RegToReg, AL, AL);
            self.gen_expr(expr, var_map);
            self.emit_instr(Pop, RegToReg, BL, AL);
            self.emit_instr(Mov, RegToMemr, BL, AL);
        } else {
            self.gen_expr(expr, var_map.clone());

            // Local
            if let Some(&offset) = var_map.get(&var.name) {
                self.emit_instr(Mov, RegToMemra, BP, AL);
                self.emit_word(0u16.overflowing_sub(offset + 1).0);
            // Arg
            } else if let Some(index) = args.iter().position(|arg| arg == &var.name) {
                self.emit_instr(Mov, RegToMemra, BP, AL);
                self.emit_word((args.len() - index + 1) as u16);
            // Global
            } else {
                self.emit_instr(Mov, RegToMema, AL, AL);
                self.emit_label(&var.name);
            }
        }
    }

    fn gen_stop(&mut self) {
        let l_id = *self
            .get_current_loop()
            .expect("\"STOP\" keyword must be in the loop body");
        self.emit_instr(Jmp, ImmToReg, AL, AL);
        self.emit_label(&format!(".loop {} out", l_id));
    }

    fn gen_return(&mut self) {
        let (name, _) = self
            .current_procedure
            .clone()
            .expect("\"RETURN\" keyword must be in the procedure body");

        self.emit_instr(Jmp, ImmToReg, AL, AL);
        self.emit_label(&format!("{} .out", name));
    }

    fn gen_expr(&mut self, expr: Expression, var_map: HashMap<String, u16>) {
        let (_, args) = self.current_procedure.clone().unwrap();

        match expr {
            Expression::Number(n) => {
                self.emit_instr(Mov, ImmToReg, AL, AL);
                self.emit_word(n as u16);
            }

            Expression::CString(s) => {
                self.emit_instr(Mov, ImmToReg, AL, AL);
                self.emit_label(&format!(".str {}", s));
            }

            Expression::Variable(vu) => {
                // Local variable (stack) ; BP - (offset + 1) -> AL
                if let Some(&offset) = var_map.get(&vu.name) {
                    self.emit_instr(Mov, MemraToReg, AL, BP);
                    self.emit_word(0u16.overflowing_sub(offset + 1).0);
                // Procedure argument
                } else if let Some(index) = args.iter().position(|arg| arg == &vu.name) {
                    self.emit_instr(Mov, MemraToReg, AL, BP);
                    self.emit_word((args.len() - index + 1) as u16);
                // Global variable or procedure
                } else {
                    self.emit_instr(Mov, MemaToReg, AL, AL);
                    self.emit_label(&vu.name);
                }

                if let Some(off) = vu.deref_offset {
                    self.emit_instr(Push, RegToReg, AL, AL);
                    self.gen_expr(*off, var_map);
                    self.emit_instr(Pop, RegToReg, BL, AL);
                    self.emit_instr(Add, RegToReg, AL, BL);
                    self.emit_instr(Mov, MemrToReg, AL, AL);
                }
            }

            Expression::VariableAddr(name) => {
                // Local variable (stack)
                if let Some(&offset) = var_map.get(&name) {
                    self.emit_instr(Mov, RegToReg, AL, BP);
                    self.emit_instr(Sub, ImmToReg, AL, AL);
                    self.emit_word(offset + 1);
                // Procedure argument
                } else if let Some(index) = args.iter().position(|arg| arg == &name) {
                    self.emit_instr(Mov, RegToReg, AL, BP);
                    self.emit_instr(Add, ImmToReg, AL, AL);
                    self.emit_word((args.len() - index + 1) as u16);
                // Global variable or procedure
                } else {
                    self.emit_instr(Mov, ImmToReg, AL, AL);
                    self.emit_label(&name);
                }
            }

            Expression::Expr(e) => {
                self.gen_expr(*e, var_map);
            }

            Expression::BinaryOp { op, left, right } => {
                // Right expr -> AL; Push(AL)
                self.gen_expr(*right, var_map.clone());
                self.emit_instr(Push, RegToReg, AL, AL);

                // Left -> AL;
                self.gen_expr(*left, var_map);

                // Pop(right) -> BL
                self.emit_instr(Pop, RegToReg, BL, AL);

                match op {
                    Operator::Add | Operator::Sub | Operator::Mul | Operator::Div => {
                        let opcode = match op {
                            Operator::Add => Opcode::Add,
                            Operator::Sub => Opcode::Sub,
                            Operator::Mul => Opcode::Mul,
                            Operator::Div => Opcode::Div,
                            _ => unreachable!(),
                        };
                        self.emit_instr(opcode, RegToReg, AL, BL);
                    }

                    Operator::Mod => {
                        self.emit_instr(Div, RegToReg, AL, BL);
                        self.emit_instr(Mov, RegToReg, AL, AH);
                    }

                    Operator::Eq
                    | Operator::NotEq
                    | Operator::Lt
                    | Operator::Gt
                    | Operator::LtEq
                    | Operator::GtEq => {
                        let id = self.next_label_id();
                        let true_label = format!(".cmp true {}", id);
                        let end_label = format!(".cmp end {}", id);

                        self.emit_instr(Cmp, RegToReg, AL, BL);

                        let jmp_opcode = match op {
                            Operator::Eq => Opcode::Je,
                            Operator::NotEq => Opcode::Jne,
                            Operator::Lt => Opcode::Jl,
                            Operator::Gt => Opcode::Jg,
                            Operator::LtEq => Opcode::Jle,
                            Operator::GtEq => Opcode::Jge,
                            _ => unreachable!(),
                        };

                        self.emit_instr(jmp_opcode, ImmToReg, AL, AL);
                        self.emit_label(&true_label);

                        // False branch
                        self.emit_instr(Mov, ImmToReg, AL, AL);
                        self.emit_word(0);
                        self.emit_instr(Jmp, ImmToReg, AL, AL);
                        self.emit_label(&end_label);

                        // True branch
                        self.set_label(&true_label);
                        self.emit_instr(Mov, ImmToReg, AL, AL);
                        self.emit_word(1);

                        self.set_label(&end_label);
                    }

                    Operator::And => {
                        let l_id = self.next_label_id();
                        let false_label = format!(".and false {}", l_id);
                        let end_label = format!(".and end {}", l_id);

                        // AL == 0 -> false
                        self.emit_instr(Cmp, ImmToReg, AL, AL);
                        self.emit_word(0);
                        self.emit_instr(Opcode::Je, ImmToReg, AL, AL);
                        self.emit_label(&false_label);

                        // BL == 0 -> false
                        self.emit_instr(Cmp, ImmToReg, BL, BL);
                        self.emit_word(0);
                        self.emit_instr(Opcode::Je, ImmToReg, AL, AL);
                        self.emit_label(&false_label);

                        // BL != 0 && AL && 0 -> true
                        self.emit_instr(Mov, ImmToReg, AL, AL);
                        self.emit_word(1);
                        self.emit_instr(Jmp, ImmToReg, AL, AL);
                        self.emit_label(&end_label);

                        // false
                        self.set_label(&false_label);
                        self.emit_instr(Mov, ImmToReg, AL, AL);
                        self.emit_word(0);

                        self.set_label(&end_label);
                    }

                    Operator::Or => {
                        let l_id = self.next_label_id();
                        let true_label = format!("_or_true_{}", l_id);
                        let end_label = format!("_or_end_{}", l_id);

                        // AL != 0 -> true
                        self.emit_instr(Cmp, ImmToReg, AL, AL);
                        self.emit_word(0);
                        self.emit_instr(Opcode::Jne, ImmToReg, AL, AL);
                        self.emit_label(&true_label);

                        // BL != 0 -> true
                        self.emit_instr(Cmp, ImmToReg, BL, BL);
                        self.emit_word(0);
                        self.emit_instr(Opcode::Jne, ImmToReg, AL, AL);
                        self.emit_label(&true_label);

                        // AL == 0 && BL == 0 -> false
                        self.emit_instr(Mov, ImmToReg, AL, AL);
                        self.emit_word(0);
                        self.emit_instr(Jmp, ImmToReg, AL, AL);
                        self.emit_label(&end_label);

                        // ture
                        self.set_label(&true_label);
                        self.emit_instr(Mov, ImmToReg, AL, AL);
                        self.emit_word(1);

                        self.set_label(&end_label);
                    }
                };
            }
        };
    }

    fn set_label(&mut self, label: &str) {
        self.labels.insert(label.to_string(), self.commands.len());
    }

    fn next_label_id(&mut self) -> usize {
        let id = self.label_counter;
        self.label_counter += 1;
        id
    }

    pub fn add_labels_offset(&mut self, offset: i32) {
        for (_, v) in self.labels.iter_mut() {
            *v += offset as usize;
        }
    }

    fn get_current_loop(&self) -> Option<&usize> {
        self.current_loop_id_stack.last()
    }

    fn emit_instr(&mut self, opcode: Opcode, mode: Mode, rd: Register, rs: Register) {
        let instr =
            ((opcode as u16) << 10) | ((mode as u16) << 6) | ((rd as u16) << 3) | (rs as u16);
        self.emit_word(instr);
    }

    fn emit_word(&mut self, word: u16) {
        self.commands.push(Command::Word(word));
    }

    fn emit_label(&mut self, label: &str) {
        self.commands.push(Command::Label(label.to_owned()))
    }
}
