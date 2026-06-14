use isa::Mode::*;
use isa::Opcode::*;
use isa::Register::*;
use isa::*;
use std::collections::HashMap;

use crate::parser::ASTNode;
use crate::parser::Argument;

pub enum Command {
    Word(u16),
    Label(String),
}

pub struct Codegen {
    commands: Vec<Command>,
    labels: HashMap<String, usize>,
    label_counter: usize,

    //
    current_loop_lbl_stack: Vec<String>,
}

impl Codegen {
    pub fn new(ast: Vec<ASTNode>) -> Self {
        let mut code = Self {
            commands: vec![],
            labels: HashMap::new(),
            label_counter: 0,
            current_loop_lbl_stack: vec![],
        };

        code
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
                ASTNode::Loop { block } => {
                    self.gen_loop(block, var_map.clone(), stack_size);
                }

                ASTNode::Variable { name, vtype } => {
                    var_map.insert(name, *stack_size);
                    *stack_size += vtype.get_size_in_words();
                }

                ASTNode::Stop => {
                    self.gen_stop();
                }

                ASTNode::Block { .. } | ASTNode::Procedure { .. } => {
                    unreachable!()
                }

                _ => unimplemented!(),
            }
        }
    }

    fn gen_procedure(&mut self, name: String, args: Vec<Argument>, block: Box<ASTNode>) {
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

        self.gen_block(*block, var_map, &mut stack_size);

        self.commands[frame_size_addr] = Command::Word(stack_size);

        // Restore Callee-save registers
        self.emit_instr(Pop, RegToReg, CH, AL);
        self.emit_instr(Pop, RegToReg, CL, AL);
        self.emit_instr(Pop, RegToReg, BH, AL);
        self.emit_instr(Pop, RegToReg, BL, AL);
        self.emit_instr(Pop, RegToReg, AH, AL);
        self.emit_instr(Pop, RegToReg, AL, AL);

        self.emit_instr(Leave, RegToReg, AL, AL);
        self.emit_instr(Ret, ImmToReg, AL, AL);
        self.emit_word(args.len() as u16);
    }

    fn gen_loop(
        &mut self,
        block: Box<ASTNode>,
        var_map: HashMap<String, u16>,
        stack_size: &mut u16,
    ) {
        let (lbl_str, lc) = self.set_label_with_counter("__loop", None, None);
        self.current_loop_lbl_stack.push(lbl_str.clone());

        self.gen_block(*block, var_map, stack_size);

        self.current_loop_lbl_stack.pop();
        self.emit_instr(Jmp, ImmToReg, AL, AL);
        self.emit_label(&lbl_str);
        self.set_label_with_counter("__loop", Some(lc), Some("out"));
    }

    fn gen_stop(&mut self) {
        let mut curr_loop = self
            .get_current_loop()
            .expect("\"STOP\" keyword must be in the loop body")
            .clone();
        curr_loop += "_out";
        self.emit_instr(Jmp, ImmToReg, AL, AL);
        self.emit_label(&curr_loop);
    }

    fn set_label_with_counter(
        &mut self,
        prefix: &str,
        num: Option<usize>,
        postfix: Option<&str>,
    ) -> (String, usize) {
        let lc = match num {
            Some(n) => n,
            None => {
                let lbl = self.label_counter;
                self.label_counter += 1;
                lbl
            }
        };

        let lbl_str = match postfix {
            Some(postfix) => format!("{}_{}_{}", prefix, lc, postfix),
            None => format!("{}_{}", prefix, lc),
        };
        self.labels.insert(lbl_str.clone(), self.commands.len());
        (lbl_str, lc)
    }

    fn set_label(&mut self, label: &str) {
        self.labels.insert(label.to_string(), self.commands.len());
    }

    fn get_current_loop(&self) -> Option<&String> {
        self.current_loop_lbl_stack.last()
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
