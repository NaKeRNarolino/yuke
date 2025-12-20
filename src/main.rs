use std::collections::HashMap;
use std::sync::Arc;
use crate::typed::{DataTypeSignature, GlobalTypes};
use std::fs;
use proc_macro::type_signature;
use crate::interpret::{Interpreter, RuntimeScope};
use crate::interpret::structs::RuntimeValue;
use crate::lexer::tokenize;
use crate::log::{Log, LogOrigin};
use crate::parser::Parser;
use crate::store::AtomStorage;
use crate::typed::DataTypeKind;
use crate::util::arw;

pub mod lexer;
pub mod store;
pub mod parser;
pub mod interpret;
mod util;
mod log;
mod typed;

fn main() {
    let file = fs::read_to_string("./dev/main.yk").unwrap();

    dbg!(&file);

    let tk = tokenize("main.yk".to_string(), file.to_string());

    dbg!(&tk);


    let numerics = type_signature! {
        Num {
            match |t, v| { matches!(v, RuntimeValue::Number(_)) },
            kind BuiltIn,
            children {
                Int {
                    match |t, v| { matches!(v, RuntimeValue::Number(x) if &x.floor() == x) },
                    kind BuiltIn
                },
                Flt {
                    match |t, v| { matches!(v, RuntimeValue::Number(x) if &x.floor() != x) },
                    kind BuiltIn
                },
            }
        }
    };
    let string = type_signature! {
        Str {
            match |t, v| { matches!(v, RuntimeValue::String(_)) },
            kind BuiltIn
        }
    };
    let bool = type_signature! {
        Bln {
            match |t, v| { matches!(v, RuntimeValue::Boolean(_)) },
            kind BuiltIn
        }
    };

    GlobalTypes::add_type(numerics);
    GlobalTypes::add_type(string);
    GlobalTypes::add_type(bool);

    let ast = Parser { tokens: tk }.ast();

    dbg!(&ast);
    let i = Interpreter { };

    let scope = RuntimeScope::new();

    let arw = arw(scope);
    let ev = i.eval_node(
        ast, arw.clone()
    );

    dbg!(&ev);
    //
    // dbg!(&arw.r().get_variable(
    //     AtomStorage::atom("x".to_string())
    // ).r().value.r());
}
