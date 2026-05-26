use crate::compile::Compiler;
use crate::interpret::structs::RuntimeValueType;
use crate::interpret::{Interpreter, RuntimeScope};
use crate::lexer::tokenize;
use crate::log::{Log, LogOrigin};
use crate::parser::Parser;
use crate::static_analysis::StaticAnalysis;
use crate::store::AtomStorage;
use crate::typed::DataTypeKind;
use crate::typed::{TypeSignature, Types};
use crate::util::arw;
use crate::vm::VM;
use proc_macro::{type_signature};
use std::collections::HashMap;
use std::fs;
use std::sync::Arc;
use std::time::Instant;

mod compile;
pub mod interpret;
pub mod lexer;
mod log;
pub mod parser;
mod static_analysis;
pub mod store;
mod typed;
mod util;
mod vm;

mod interpret_types {
    use crate::typed::TypeStructure;
    use crate::util::Rw;
    use crate::Arc;
    use crate::interpret::structs::RuntimeValueType;
    use crate::store::AtomStorage;
    use crate::typed::{DataTypeKind, TypeSignature, UnderlyingType, Types};
    use proc_macro::type_signature;
    use std::collections::HashMap;

    pub fn types(types: &Types) {
        let flt = type_signature! {
            Flt {
                struct UnderlyingType::new(
                    TypeStructure::Primitive("Flt".to_string())
                ),
                match |t, v| { matches!(v, RuntimeValueType::Number(x) if x.floor() != *x) },
                kind BuiltIn
            }
        };
        let int = type_signature! {
            Int {
                struct UnderlyingType::new(
                    TypeStructure::Primitive("Int".to_string())
                ),
                match |t, v| { matches!(v, RuntimeValueType::Number(x) if x.floor() == *x) },
                kind BuiltIn
            }
        };
        let string = type_signature! {
            Str {
                struct UnderlyingType::new(
                    TypeStructure::Primitive("Str".to_string())
                ),
                match |t, v| { matches!(v, RuntimeValueType::String(_)) },
                kind BuiltIn
            }
        };
        let bool = type_signature! {
            Bln {
                struct UnderlyingType::new(
                    TypeStructure::Primitive("Bln".to_string())
                ),
                match |t, v| { matches!(v, RuntimeValueType::Boolean(_)) },
                kind BuiltIn
            }
        };
        let fnc = type_signature! {
            Fnc {
                struct UnderlyingType::new(
                    TypeStructure::Primitive("Fnc".to_string())
                ),
                match |t, v| { matches!(v, RuntimeValueType::Function(_) )},
                kind BuiltIn,
                built |t, v| {
                    matches!(v, RuntimeValueType::Function(fd) if fd.matches_generics(&t.generics))
                }
            }
        };
        let unit = type_signature! {
            Uni {
                struct UnderlyingType::new(
                    TypeStructure::Primitive("Uni".to_string())
                ),
                match |t, v| { matches!(v, RuntimeValueType::Unit) },
                kind BuiltIn
            }
        };
        let any = type_signature! {
            Any {
                struct UnderlyingType::new(
                    TypeStructure::Primitive("Any".to_string())
                ),
                match |_, _| { true },
                kind BuiltIn
            }
        };
        let arr = type_signature! {
            Arr {
                struct UnderlyingType::new(
                    TypeStructure::Primitive("Arr".to_string())
                ),
                match |t, v| { matches!(v, RuntimeValueType::Array(_) )},
                kind BuiltIn,
                built |t, v| {
                    matches!(v, RuntimeValueType::Array(arr) if arr.ty == t.generics[0] || t.generics[0].type_ref.name == AtomStorage::atom("Any"))
                    // todo! WORKAROUND, NEEDS TO BE REFACTORED TO HANDLE TYPE CASTS   ^^^^^^^^^^^^^^^^^^^^^^^^^^^
                }
            }
        };

        types.add_type(flt);
        types.add_type(int);
        types.add_type(unit);
        types.add_type(string);
        types.add_type(bool);
        types.add_type(fnc);
        types.add_type(arr);
        types.add_type(any);
    }
}

fn main() {
    let file = fs::read_to_string("./dev/main.yk").unwrap();

    dbg!(&file);

    let tk = tokenize("main.yk".to_string(), file.to_string());

    let instant = Instant::now();

    let ast = Parser { tokens: tk }.ast();

    dbg!(&ast);
    println!("Execution took {}ms", instant.elapsed().as_millis());

    let mut analysis = StaticAnalysis::new();

    analysis.analyze(ast.value.clone().into_block().unwrap());

    let global_types = Types::new();

    interpret_types::types(&global_types);

    let i = Interpreter {
        global_types: Arc::new(global_types),
    };

    let scope = RuntimeScope::new(i.scope_ref());

    let arw = arw(scope);
    let ev = i.eval_node(ast, arw.clone());

    dbg!(&ev);
}
