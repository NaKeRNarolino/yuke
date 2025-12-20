use custom_derive::custom_derive;
use enum_as_inner::EnumAsInner;
use crate::lexer::structs::{OperatorType, Span};
use crate::store::Atom;

#[derive(Debug, Clone)]
pub struct ASTNode {
    pub(crate) value: ASTNodeValue,
    pub(crate) span: Span
}

#[derive(EnumAsInner, Debug, Clone)]
pub enum ASTNodeValue {
    BinaryExpression {
        left: Box<ASTNode>,
        right: Box<ASTNode>,
        op: OperatorType
    },
    Number(f64),
    Identifier(Atom),
    VariableDeclaration {
        name: Atom,
        value: Box<ASTNode>,
        immut: bool,
        data_type: Option<Box<ASTNode>>
    },
    Assignment {
        prop: Box<ASTNode>,
        value: Box<ASTNode>
    },
    String(Atom),
    Boolean(bool),
    Block {
        contents: Vec<ASTNode>
    },
    If {
        ifs: Vec<IfContent>,
        or_else: Option<Box<ASTNode>>
    },
    Type {
        content: Vec<Atom>
    },
    When {
        value: Box<ASTNode>,
        ifs: Vec<IfContent>,
        or_else: Option<Box<ASTNode>>
    },
    Unit
}

#[derive(Clone, Debug)]
pub struct IfContent {
    pub condition: Box<ASTNode>,
    pub block: Box<ASTNode>
}