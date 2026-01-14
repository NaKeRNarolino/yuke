pub mod structs;

use crate::compile::structs::{Instruction, OpCode};
use crate::lexer::structs::OperatorType;
use crate::parser::structs::{ASTNode, ASTNodeValue};
use crate::static_analysis::StaticAnalysis;
use crate::store::AtomStorage;
use crate::util::Unbox;

pub struct Compiler {
    static_analysis: StaticAnalysis,
    res: Vec<u8>,
    const_floats: Vec<f64>,
    const_strs: Vec<String>,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            static_analysis: StaticAnalysis::new(),
            res: vec![],
            const_floats: vec![],
            const_strs: vec![],
        }
    }

    pub fn compile(&mut self, nodes: Vec<ASTNode>) -> Vec<u8> {
        // 1. Compile the instructions into a temporary buffer
        self.static_analysis.analyze(nodes.clone());

        let mut bytecode_body = vec![];
        let mut last = None;

        for node in nodes {
            if node.value.is_unit() {
                if last.is_some() {
                    self.push(Instruction::op(OpCode::POP));
                }
                continue;
            }

            last = self.compile_node(node);
        }

        bytecode_body = self.res.clone();

        let mut file = vec![];
        file.extend_from_slice(b"yuke");

        file.extend_from_slice(&(self.const_floats.len() as i32).to_le_bytes());

        for &f in &self.const_floats {
            file.extend_from_slice(&f.to_le_bytes());
        }

        file.extend_from_slice(&(self.const_strs.len() as i32).to_le_bytes());
        for s in &self.const_strs {
            file.extend_from_slice(&(s.len() as u32).to_le_bytes());
            file.extend_from_slice(s.as_bytes());
        }

        file.append(&mut bytecode_body);

        file
    }

    pub fn compile_node(&mut self, node: ASTNode) -> Option<()> {
        match node.value.clone() {
            ASTNodeValue::Number(v) => {
                self.const_floats.push(v);
                let idx = self.const_floats.len() - 1;

                self.push(Instruction::new(OpCode::PUSHN, idx as u32));

                Some(())
            }
            ASTNodeValue::String(v) => {
                self.const_strs
                    .push(AtomStorage::string(v).unwrap().clone());
                let idx = self.const_strs.len() - 1;

                self.push(Instruction::new(OpCode::PUSHS, idx as u32));

                Some(())
            }
            ASTNodeValue::BinaryExpression { left, right, op } => {
                self.compile_node(left.unbox());
                self.compile_node(right.unbox());

                match op {
                    OperatorType::Plus => {
                        self.push(Instruction::op(OpCode::ADD));
                    }
                    OperatorType::Minus => {
                        self.push(Instruction::op(OpCode::SUB));
                    }
                    OperatorType::Multiply => {
                        self.push(Instruction::op(OpCode::MUL));
                    }
                    OperatorType::Divide => {
                        self.push(Instruction::op(OpCode::DIV));
                    }
                    OperatorType::Modulo => {
                        self.push(Instruction::op(OpCode::MOD));
                    }
                    _ => todo!(),
                };
                Some(())
            }
            _ => todo!(),
        }
    }

    pub fn push(&mut self, instruction: Instruction) {
        self.res.append(&mut instruction.serialize());
    }
}

// {CONST STRS}
// STR
// STR
// STR
// INSTRS
