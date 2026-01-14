mod structs;

use crate::compile::structs::OpCode;
use crate::vm::structs::RuntimeValue;
use std::collections::HashMap;

pub struct VMScope {
    locals: HashMap<u32, RuntimeValue>,
}

pub struct VM {
    stack: Vec<RuntimeValue>,
    constant_f64: Vec<f64>,
    constant_str: Vec<String>,
    bytecode: Vec<u8>,
    scopes: Vec<VMScope>,
    ip: usize,
}
//
// impl VM {
//     pub fn get_stack(&self) -> &Vec<RuntimeValue> {
//         &self.stack
//     }
//
//     pub fn new(binary: Vec<u8>) -> Self {
//         if &binary[0..4] != b"yuke" {
//             panic!("Invalid header for a compiled .yk");
//         }
//
//         let count = u32::from_le_bytes(binary[4..8].try_into().unwrap()) as usize;
//
//         let mut constants = Vec::new();
//         let mut cursor = 8;
//         for _ in 0..count {
//             let f = f64::from_le_bytes(binary[cursor..cursor + 8].try_into().unwrap());
//             constants.push(f);
//             cursor += 8;
//         }
//
//         let count_strs =
//             u32::from_le_bytes(binary[cursor..cursor + 4].try_into().unwrap()) as usize;
//         cursor += 4;
//
//         let mut constant_str = Vec::new();
//
//         for _ in 0..count_strs {
//             let str_len =
//                 u32::from_le_bytes(binary[cursor..cursor + 4].try_into().unwrap()) as usize;
//             cursor += 4;
//
//             let str_bytes = &binary[cursor..cursor + str_len];
//             constant_str.push(String::from_utf8(str_bytes.to_vec()).unwrap());
//             cursor += str_len;
//         }
//
//         let bytecode = binary[cursor..].to_vec();
//
//         Self {
//             stack: vec![],
//             constant_f64: constants,
//             bytecode,
//             constant_str,
//             ip: 0,
//             scopes: vec![VMScope::new()],
//         }
//     }
//
//     pub fn run(&mut self) {
//         while self.ip < self.bytecode.len() {
//             let op = self.bytecode[self.ip];
//             self.ip += 1;
//
//             let op: OpCode = unsafe { std::mem::transmute(op) };
//
//             match op {
//                 OpCode::PUSHN => {
//                     let idx =
//                         u32::from_le_bytes(self.bytecode[self.ip..self.ip + 4].try_into().unwrap())
//                             as usize;
//                     self.ip += 4;
//                     self.stack.push(self.constant_f64[idx].into());
//                 }
//                 OpCode::PUSHS => {
//                     let idx =
//                         u32::from_le_bytes(self.bytecode[self.ip..self.ip + 4].try_into().unwrap())
//                             as usize;
//                     self.ip += 4;
//                     self.stack.push(self.constant_str[idx].clone().into());
//                 }
//                 OpCode::ADD => {
//                     let b = self.stack.pop().unwrap();
//                     let a = self.stack.pop().unwrap();
//                     self.stack.push((a + b).into());
//                 }
//                 OpCode::SUB => {
//                     let b = self.stack.pop().unwrap();
//                     let a = self.stack.pop().unwrap();
//                     self.stack.push((a - b).into());
//                 }
//                 OpCode::MUL => {
//                     let b = self.stack.pop().unwrap();
//                     let a = self.stack.pop().unwrap();
//                     self.stack.push((a * b).into());
//                 }
//                 OpCode::DIV => {
//                     let b = self.stack.pop().unwrap();
//                     let a = self.stack.pop().unwrap();
//                     self.stack.push((a / b).into());
//                 }
//                 // ...
//                 _ => todo!(),
//             }
//         }
//     }
// }
