use std::fmt::format;

#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub enum OpCode {
    PUSHN = 0x01,
    ADD = 0x02,
    SUB = 0x03,
    MUL = 0x04,
    DIV = 0x05,
    MOD = 0x06,
    LOAD = 0x07,
    STORE = 0x08,
    POP = 0x09,
    PUSHS = 0x0A,
    PUSHB = 0x0B,
}

pub struct Instruction {
    pub content: Option<u32>,
    pub op: OpCode,
}

impl Instruction {
    pub fn new(op: OpCode, content: u32) -> Self {
        Instruction {
            op,
            content: Some(content),
        }
    }

    pub fn op(op: OpCode) -> Self {
        Instruction { op, content: None }
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.push(self.op as u8);

        if let Some(c) = self.content {
            bytes.extend_from_slice(&c.to_le_bytes());
        }

        bytes
    }
}
