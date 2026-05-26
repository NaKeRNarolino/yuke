use crate::interpret::RuntimeScope;
use crate::lexer::structs::Span;
use crate::log::{Control, Log, LogOrigin};
use crate::parser::structs::ASTNode;
use crate::store::Atom;
use crate::typed::{TypeSignature, BuiltType};
use crate::util::{Arw, Rw};
use enum_as_inner::EnumAsInner;
use std::cmp::PartialEq;
use std::collections::HashMap;
use std::ops::{Add, Div, Mul, Rem, Sub};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct RuntimeValue {
    pub type_ref: BuiltType,
    pub val: RuntimeValueType
}

impl RuntimeValue {
    pub fn new(type_ref: BuiltType, val: RuntimeValueType) -> Self {
        Self {
            type_ref, val
        }
    }
}

#[derive(Debug, Clone, EnumAsInner)]
pub enum RuntimeValueType {
    Number(f64),
    String(String),
    Boolean(bool),
    Function(FunctionData),
    Complex(ComplexData),
    Array(ArrayData),
    Unit,
}

pub trait BinExpAdd {
    type Output;

    fn add(self, rhs: Self, trace: Span) -> Self::Output;
}
pub trait BinExpMul {
    type Output;

    fn mul(self, rhs: Self, trace: Span) -> Self::Output;
}
pub trait BinExpSub {
    type Output;

    fn sub(self, rhs: Self, trace: Span) -> Self::Output;
}
pub trait BinExpDiv {
    type Output;

    fn div(self, rhs: Self, trace: Span) -> Self::Output;
}
pub trait BinExpRem {
    type Output;

    fn rem(self, rhs: Self, trace: Span) -> Self::Output;
}
pub trait BinExpRelations {
    type Output;

    fn big(self, rhs: Self, trace: Span) -> Self::Output;
    fn sml(self, rhs: Self, trace: Span) -> Self::Output;
    fn beq(self, rhs: Self, trace: Span) -> Self::Output;
    fn seq(self, rhs: Self, trace: Span) -> Self::Output;
    fn eq(self, rhs: Self, trace: Span) -> Self::Output;
    fn ieq(self, rhs: Self, trace: Span) -> Self::Output;
}

pub trait BinExpLogicals {
    type Output;

    fn l_and(self, rhs: Self, trace: Span) -> Self::Output;
    fn l_or(self, rhs: Self, trace: Span) -> Self::Output;
}

impl BinExpAdd for RuntimeValueType {
    type Output = RuntimeValueType;

    fn add(self, rhs: Self, trace: Span) -> Self::Output {
        match (self, rhs) {
            (RuntimeValueType::Number(x), RuntimeValueType::Number(y)) => RuntimeValueType::Number(x + y),
            (RuntimeValueType::Boolean(x), RuntimeValueType::Boolean(y)) => RuntimeValueType::Boolean(x || y),
            (f, s) => {
                Log::err(
                    format!("Operation '+' is not implemented for {:?} and {:?}.", f, s),
                    LogOrigin::Interpret,
                );
                Log::trace_span(trace);
                Control::exit();
            }
        }
    }
}

impl BinExpMul for RuntimeValueType {
    type Output = RuntimeValueType;

    fn mul(self, rhs: Self, trace: Span) -> Self::Output {
        match (self, rhs) {
            (RuntimeValueType::Number(x), RuntimeValueType::Number(y)) => RuntimeValueType::Number(x * y),
            (RuntimeValueType::String(x), RuntimeValueType::Number(y)) => {
                RuntimeValueType::String(x.repeat(y.floor() as usize))
            }
            (RuntimeValueType::Boolean(x), RuntimeValueType::Boolean(y)) => RuntimeValueType::Boolean(x && y),
            (f, s) => {
                Log::err(
                    format!("Operation '*' is not implemented for {:?} and {:?}.", f, s),
                    LogOrigin::Interpret,
                );
                Log::trace_span(trace);
                Control::exit();
            }
        }
    }
}

impl BinExpDiv for RuntimeValueType {
    type Output = RuntimeValueType;

    fn div(self, rhs: Self, trace: Span) -> Self::Output {
        match (self, rhs) {
            (RuntimeValueType::Number(x), RuntimeValueType::Number(y)) => RuntimeValueType::Number(x / y),
            (f, s) => {
                Log::err(
                    format!("Operation '/' is not implemented for {:?} and {:?}.", f, s),
                    LogOrigin::Interpret,
                );
                Log::trace_span(trace);
                Control::exit();
            }
        }
    }
}

impl BinExpSub for RuntimeValueType {
    type Output = RuntimeValueType;

    fn sub(self, rhs: Self, trace: Span) -> Self::Output {
        match (self, rhs) {
            (RuntimeValueType::Number(x), RuntimeValueType::Number(y)) => RuntimeValueType::Number(x - y),
            (f, s) => {
                Log::err(
                    format!("Operation '-' is not implemented for {:?} and {:?}.", f, s),
                    LogOrigin::Interpret,
                );
                Log::trace_span(trace);
                Control::exit();
            }
        }
    }
}

impl BinExpRem for RuntimeValueType {
    type Output = RuntimeValueType;

    fn rem(self, rhs: Self, trace: Span) -> Self::Output {
        match (self, rhs) {
            (RuntimeValueType::Number(x), RuntimeValueType::Number(y)) => RuntimeValueType::Number(x % y),
            (f, s) => {
                Log::err(
                    format!("Operation '%' is not implemented for {:?} and {:?}.", f, s),
                    LogOrigin::Interpret,
                );
                Log::trace_span(trace);
                Control::exit();
            }
        }
    }
}

impl BinExpRelations for RuntimeValueType {
    type Output = RuntimeValueType;

    fn big(self, rhs: Self, trace: Span) -> Self::Output {
        match (self, rhs) {
            (RuntimeValueType::Number(x), RuntimeValueType::Number(y)) => RuntimeValueType::Boolean(x > y),
            (f, s) => {
                Log::err(
                    format!("Operation '>' is not implemented for {:?} and {:?}.", f, s),
                    LogOrigin::Interpret,
                );
                Log::trace_span(trace);
                Control::exit();
            }
        }
    }

    fn sml(self, rhs: Self, trace: Span) -> Self::Output {
        match (self, rhs) {
            (RuntimeValueType::Number(x), RuntimeValueType::Number(y)) => RuntimeValueType::Boolean(x < y),
            (f, s) => {
                Log::err(
                    format!("Operation '<' is not implemented for {:?} and {:?}.", f, s),
                    LogOrigin::Interpret,
                );
                Log::trace_span(trace);
                Control::exit();
            }
        }
    }

    fn beq(self, rhs: Self, trace: Span) -> Self::Output {
        match (self, rhs) {
            (RuntimeValueType::Number(x), RuntimeValueType::Number(y)) => RuntimeValueType::Boolean(x >= y),
            (f, s) => {
                Log::err(
                    format!("Operation '>=' is not implemented for {:?} and {:?}.", f, s),
                    LogOrigin::Interpret,
                );
                Log::trace_span(trace);
                Control::exit();
            }
        }
    }

    fn seq(self, rhs: Self, trace: Span) -> Self::Output {
        match (self, rhs) {
            (RuntimeValueType::Number(x), RuntimeValueType::Number(y)) => RuntimeValueType::Boolean(x <= y),
            (f, s) => {
                Log::err(
                    format!("Operation '<=' is not implemented for {:?} and {:?}.", f, s),
                    LogOrigin::Interpret,
                );
                Log::trace_span(trace);
                Control::exit();
            }
        }
    }

    fn eq(self, rhs: Self, trace: Span) -> Self::Output {
        match (self, rhs) {
            (RuntimeValueType::Number(x), RuntimeValueType::Number(y)) => RuntimeValueType::Boolean(x == y),
            (RuntimeValueType::String(x), RuntimeValueType::String(y)) => RuntimeValueType::Boolean(x == y),
            (RuntimeValueType::Boolean(x), RuntimeValueType::Boolean(y)) => RuntimeValueType::Boolean(x == y),
            (f, s) => {
                Log::err(
                    format!("Operation '==' is not implemented for {:?} and {:?}.", f, s),
                    LogOrigin::Interpret,
                );
                Log::trace_span(trace);
                Control::exit();
            }
        }
    }

    fn ieq(self, rhs: Self, trace: Span) -> Self::Output {
        match (self, rhs) {
            (RuntimeValueType::Number(x), RuntimeValueType::Number(y)) => RuntimeValueType::Boolean(x != y),
            (RuntimeValueType::String(x), RuntimeValueType::String(y)) => RuntimeValueType::Boolean(x != y),
            (RuntimeValueType::Boolean(x), RuntimeValueType::Boolean(y)) => RuntimeValueType::Boolean(x != y),
            (f, s) => {
                Log::err(
                    format!("Operation '!=' is not implemented for {:?} and {:?}.", f, s),
                    LogOrigin::Interpret,
                );
                Log::trace_span(trace);
                Control::exit();
            }
        }
    }
}

impl BinExpLogicals for RuntimeValueType {
    type Output = RuntimeValueType;

    fn l_and(self, rhs: Self, trace: Span) -> Self::Output {
        match (self, rhs) {
            (RuntimeValueType::Boolean(x), RuntimeValueType::Boolean(y)) => RuntimeValueType::Boolean(x && y),
            (f, s) => {
                Log::err(
                    format!("Operation '&&' is not implemented for {:?} and {:?}.", f, s),
                    LogOrigin::Interpret,
                );
                Log::trace_span(trace);
                Control::exit();
            }
        }
    }

    fn l_or(self, rhs: Self, trace: Span) -> Self::Output {
        match (self, rhs) {
            (RuntimeValueType::Boolean(x), RuntimeValueType::Boolean(y)) => RuntimeValueType::Boolean(x || y),
            (f, s) => {
                Log::err(
                    format!("Operation '||' is not implemented for {:?} and {:?}.", f, s),
                    LogOrigin::Interpret,
                );
                Log::trace_span(trace);
                Control::exit();
            }
        }
    }
}

impl RuntimeValueType {
    pub fn index(self, rhs: Self, trace: Span) -> RuntimeValue {
        match (self, rhs) {
            (RuntimeValueType::Array(ad), RuntimeValueType::Number(v)) => {
                ad.values[v.floor() as usize].clone()
            }
            (f, s) => {
                Log::err(
                    format!(
                        "Operation '[?]' is not implemented for {:?} and {:?}.",
                        f, s
                    ),
                    LogOrigin::Interpret,
                );
                Log::trace_span(trace);
                Control::exit();
            }
        }
    }
}

#[derive(Debug)]
pub struct Variable {
    pub(crate) name: Atom,
    pub(crate) value: Rw<RuntimeValue>,
    pub(crate) is_immut: bool,
    pub ty: BuiltType,
}

pub enum AssignmentProperty {
    VariableOrFunction(Atom),
}

#[derive(Clone, Debug)]
pub struct FunctionData {
    pub arg_names: Vec<Atom>,
    pub arg_types: Vec<BuiltType>,
    pub ret_type: BuiltType,
    pub function_body: Box<ASTNode>,
    pub scope: Arw<RuntimeScope>,
}

impl FunctionData {
    pub fn matches_generics(&self, generics: &Vec<BuiltType>) -> bool {
        if generics.len() != self.arg_types.len() + 1 {
            return false;
        }

        if generics.last().unwrap() != &self.ret_type {
            return false;
        }

        for i in 0..(generics.len() - 1) {
            if generics[i] != self.arg_types[i] {
                return false;
            }
        }

        true
    }
}

#[derive(Clone, Debug)]
pub enum TypeData {
    Struct(StructData),
}

#[derive(Clone, Debug)]
pub struct StructData {
    pub prop_names: Vec<Atom>,
    pub prop_types: Vec<BuiltType>,
    pub uuid: Uuid,
}

#[derive(Clone, Debug)]
pub enum ComplexData {
    Struct(ComplexStruct),
}

#[derive(Clone, Debug)]
pub struct ComplexStruct {
    pub name: Atom,
    pub prop_names: Vec<Atom>,
    pub prop_types: Vec<BuiltType>,
    pub data: HashMap<Atom, RuntimeValue>,
}

#[derive(Clone, Debug)]
pub struct ArrayData {
    pub ty: BuiltType,
    pub values: Vec<RuntimeValue>,
}
