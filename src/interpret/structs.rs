use std::cmp::PartialEq;
use std::ops::{Add, Div, Mul, Rem, Sub};
use std::sync::Arc;
use crate::interpret::RuntimeScope;
use crate::lexer::structs::Span;
use crate::log::{Control, Log, LogOrigin};
use crate::parser::structs::ASTNode;
use crate::store::Atom;
use crate::typed::{DataTypeSignature, FinalizedDataType};
use crate::util::{Arw, Rw};

#[derive(Debug, Clone)]
pub enum RuntimeValue {
    Number(f64),
    String(String),
    Boolean(bool),
    Function(FunctionData),
    Unit
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



impl BinExpAdd for RuntimeValue {
    type Output = RuntimeValue;

    fn add(self, rhs: Self, trace: Span) -> Self::Output {
        match (self, rhs) {
            (RuntimeValue::Number(x), RuntimeValue::Number(y)) => RuntimeValue::Number(x + y),
            (RuntimeValue::Boolean(x), RuntimeValue::Boolean(y)) => RuntimeValue::Boolean(x || y),
            (f, s) => {
                Log::err(format!("Operation '+' is not implemented for {:?} and {:?}.", f, s), LogOrigin::Interpret);
                Log::trace_span(trace);
                Control::exit();
            }
        }
    }
}


impl BinExpMul for RuntimeValue {
    type Output = RuntimeValue;

    fn mul(self, rhs: Self, trace: Span) -> Self::Output {
        match (self, rhs) {
            (RuntimeValue::Number(x), RuntimeValue::Number(y)) => RuntimeValue::Number(x * y),
            (
                RuntimeValue::String(x), RuntimeValue::Number(y)
            ) => RuntimeValue::String(x.repeat(y.floor() as usize)),
            (RuntimeValue::Boolean(x), RuntimeValue::Boolean(y)) => RuntimeValue::Boolean(x && y),
            (f, s) => {
                Log::err(format!("Operation '*' is not implemented for {:?} and {:?}.", f, s), LogOrigin::Interpret);
                Log::trace_span(trace);
                Control::exit();
            }
        }
    }
}

impl BinExpDiv for RuntimeValue {
    type Output = RuntimeValue;

    fn div(self, rhs: Self, trace: Span) -> Self::Output {
        match (self, rhs) {
            (RuntimeValue::Number(x), RuntimeValue::Number(y)) => RuntimeValue::Number(x / y),
            (f, s) => {
                Log::err(format!("Operation '/' is not implemented for {:?} and {:?}.", f, s), LogOrigin::Interpret);
                Log::trace_span(trace);
                Control::exit();
            }
        }
    }
}

impl BinExpSub for RuntimeValue {
    type Output = RuntimeValue;

    fn sub(self, rhs: Self, trace: Span) -> Self::Output {
        match (self, rhs) {
            (RuntimeValue::Number(x), RuntimeValue::Number(y)) => RuntimeValue::Number(x - y),
            (f, s) => {
                Log::err(format!("Operation '-' is not implemented for {:?} and {:?}.", f, s), LogOrigin::Interpret);
                Log::trace_span(trace);
                Control::exit();
            }
        }
    }
}

impl BinExpRem for RuntimeValue {
    type Output = RuntimeValue;

    fn rem(self, rhs: Self, trace: Span) -> Self::Output {
        match (self, rhs) {
            (RuntimeValue::Number(x), RuntimeValue::Number(y)) => RuntimeValue::Number(x % y),
            (f, s) => {
                Log::err(format!("Operation '%' is not implemented for {:?} and {:?}.", f, s), LogOrigin::Interpret);
                Log::trace_span(trace);
                Control::exit();
            }
        }
    }
}

impl BinExpRelations for RuntimeValue {
    type Output = RuntimeValue;

    fn big(self, rhs: Self, trace: Span) -> Self::Output {
        match (self, rhs) {
            (RuntimeValue::Number(x), RuntimeValue::Number(y)) => RuntimeValue::Boolean(x > y),
            (f, s) => {
                Log::err(format!("Operation '>' is not implemented for {:?} and {:?}.", f, s), LogOrigin::Interpret);
                Log::trace_span(trace);
                Control::exit();
            }
        }
    }

    fn sml(self, rhs: Self, trace: Span) -> Self::Output {
        match (self, rhs) {
            (RuntimeValue::Number(x), RuntimeValue::Number(y)) => RuntimeValue::Boolean(x < y),
            (f, s) => {
                Log::err(format!("Operation '<' is not implemented for {:?} and {:?}.", f, s), LogOrigin::Interpret);
                Log::trace_span(trace);
                Control::exit();
            }
        }
    }

    fn beq(self, rhs: Self, trace: Span) -> Self::Output {
        match (self, rhs) {
            (RuntimeValue::Number(x), RuntimeValue::Number(y)) => RuntimeValue::Boolean(x >= y),
            (f, s) => {
                Log::err(format!("Operation '>=' is not implemented for {:?} and {:?}.", f, s), LogOrigin::Interpret);
                Log::trace_span(trace);
                Control::exit();
            }
        }
    }

    fn seq(self, rhs: Self, trace: Span) -> Self::Output {
        match (self, rhs) {
            (RuntimeValue::Number(x), RuntimeValue::Number(y)) => RuntimeValue::Boolean(x <= y),
            (f, s) => {
                Log::err(format!("Operation '<=' is not implemented for {:?} and {:?}.", f, s), LogOrigin::Interpret);
                Log::trace_span(trace);
                Control::exit();
            }
        }
    }

    fn eq(self, rhs: Self, trace: Span) -> Self::Output {
        match (self, rhs) {
            (RuntimeValue::Number(x), RuntimeValue::Number(y)) => RuntimeValue::Boolean(x == y),
            (RuntimeValue::String(x), RuntimeValue::String(y)) => RuntimeValue::Boolean(x == y),
            (RuntimeValue::Boolean(x), RuntimeValue::Boolean(y)) => RuntimeValue::Boolean(x == y),
            (f, s) => {
                Log::err(format!("Operation '==' is not implemented for {:?} and {:?}.", f, s), LogOrigin::Interpret);
                Log::trace_span(trace);
                Control::exit();
            }
        }
    }

    fn ieq(self, rhs: Self, trace: Span) -> Self::Output {
        match (self, rhs) {
            (RuntimeValue::Number(x), RuntimeValue::Number(y)) => RuntimeValue::Boolean(x != y),
            (RuntimeValue::String(x), RuntimeValue::String(y)) => RuntimeValue::Boolean(x != y),
            (RuntimeValue::Boolean(x), RuntimeValue::Boolean(y)) => RuntimeValue::Boolean(x != y),
            (f, s) => {
                Log::err(format!("Operation '!=' is not implemented for {:?} and {:?}.", f, s), LogOrigin::Interpret);
                Log::trace_span(trace);
                Control::exit();
            }
        }
    }
}

impl BinExpLogicals for RuntimeValue {
    type Output = RuntimeValue;

    fn l_and(self, rhs: Self, trace: Span) -> Self::Output {
        match (self, rhs) {
            (RuntimeValue::Boolean(x), RuntimeValue::Boolean(y)) => RuntimeValue::Boolean(x && y),
            (f, s) => {
                Log::err(format!("Operation '&&' is not implemented for {:?} and {:?}.", f, s), LogOrigin::Interpret);
                Log::trace_span(trace);
                Control::exit();
            }
        }
    }

    fn l_or(self, rhs: Self, trace: Span) -> Self::Output {
        match (self, rhs) {
            (RuntimeValue::Boolean(x), RuntimeValue::Boolean(y)) => RuntimeValue::Boolean(x || y),
            (f, s) => {
                Log::err(format!("Operation '||' is not implemented for {:?} and {:?}.", f, s), LogOrigin::Interpret);
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
    pub ty: FinalizedDataType
}

pub enum AssignmentProperty {
    VariableOrFunction(Atom)
}

#[derive(Clone, Debug)]
pub struct FunctionData {
    pub arg_names: Vec<Atom>,
    pub arg_types: Vec<FinalizedDataType>,
    pub ret_type: FinalizedDataType,
    pub function_body: Box<ASTNode>,
    pub scope: Arw<RuntimeScope>
}


impl FunctionData {
    pub fn matches_generics(&self, generics: &Vec<FinalizedDataType>) -> bool {
        if generics.len() != self.arg_types.len() + 1 {
            return false
        }

        if generics.last().unwrap() != &self.ret_type {
            return false
        }

        for i in 0..(generics.len() - 1) {
            if generics[i] != self.arg_types[i] {
                return false
            }
        }

        true
    }
}