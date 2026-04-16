use crate::lexer::structs::{Location, OperatorType, Span};
use crate::store::Atom;
use custom_derive::custom_derive;
use enum_as_inner::EnumAsInner;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ASTNode {
    pub(crate) value: ASTNodeValue,
    pub(crate) span: Span,
    pub id: Uuid,
}

impl ASTNode {
    pub fn new(span: Span, value: ASTNodeValue) -> ASTNode {
        ASTNode {
            span,
            value,
            id: Uuid::new_v4(),
        }
    }

    pub fn unit() -> ASTNode {
        ASTNode::new(
            Span {
                file_name: Atom(0),
                start: Location::only(0, 0),
                end: Location::only(0, 0),
            },
            ASTNodeValue::Unit,
        )
    }
}

#[derive(EnumAsInner, Debug, Clone)]
pub enum ASTNodeValue {
    BinaryExpression {
        left: Box<ASTNode>,
        right: Box<ASTNode>,
        op: OperatorType,
    },
    Number(f64),
    Identifier(Atom),
    VariableDeclaration {
        name: Atom,
        value: Box<ASTNode>,
        immut: bool,
        data_type: Option<Box<ASTNode>>,
    },
    Assignment {
        prop: Box<ASTNode>,
        value: Box<ASTNode>,
    },
    String(Atom),
    Boolean(bool),
    Block {
        contents: Vec<ASTNode>,
    },
    If {
        ifs: Vec<IfContent>,
        or_else: Option<Box<ASTNode>>,
    },
    Type {
        dynamic: bool,
        content: Vec<Atom>,
        generics: Vec<ASTNode>,
    },
    When {
        // value: Box<ASTNode>,
        ifs: Vec<IfContent>,
        or_else: Option<Box<ASTNode>>,
    },
    Function {
        arg_names: Vec<Atom>,
        arg_types: Vec<ASTNode>,
        ret_type: Box<ASTNode>,
        body: Box<ASTNode>,
    },
    Call {
        on: Box<ASTNode>,
        args: Vec<ASTNode>,
    },
    StructDefinition {
        prop_names: Vec<Atom>,
        prop_types: Vec<ASTNode>,
    },
    StructCreation {
        name: Atom,
        props: HashMap<Atom, ASTNode>,
    },
    PropertyAccess {
        on: Box<ASTNode>,
        property: Atom,
    },
    ArrayDeclaration {
        values: Vec<ASTNode>,
        ty: Option<Box<ASTNode>>,
    },
    ArrayAccess {
        on: Box<ASTNode>,
        index: Box<ASTNode>,
    },
    Unit,
    Method {
        name: Atom,
        data_type: Box<ASTNode>,
        fn_ast: Box<ASTNode>
    }
}

#[derive(Clone, Debug)]
pub struct IfContent {
    pub condition: Box<ASTNode>,
    pub block: Box<ASTNode>,
}
