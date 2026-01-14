use crate::compile::Compiler;
use crate::interpret::structs::RuntimeValue;
use crate::interpret::{Interpreter, RuntimeScope};
use crate::lexer::tokenize;
use crate::log::{Log, LogOrigin};
use crate::parser::Parser;
use crate::static_analysis::StaticAnalysis;
use crate::store::AtomStorage;
use crate::typed::DataTypeKind;
use crate::typed::TypeSig;
use crate::typed::{DataTypeSignature, Types};
use crate::util::arw;
use crate::vm::VM;
use proc_macro::{type_signature, yuke_type};
use std::collections::HashMap;
use std::fs;
use std::sync::Arc;

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
    use crate::Arc;
    use crate::interpret::structs::RuntimeValue;
    use crate::store::AtomStorage;
    use crate::typed::{DataTypeKind, DataTypeSignature, Types};
    use proc_macro::type_signature;
    use std::collections::HashMap;

    pub fn types(types: &Types) {
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
        let fnc = type_signature! {
            Fnc {
                match |t, v| { matches!(v, RuntimeValue::Function(_) )},
                kind BuiltIn,
                finalized |t, v| {
                    matches!(v, RuntimeValue::Function(fd) if fd.matches_generics(&t.generics))
                }
            }
        };
        let unit = type_signature! {
            Uni {
                match |t, v| { matches!(v, RuntimeValue::Unit) },
                kind BuiltIn
            }
        };
        let typ = type_signature! {
            Typ {
                match |t, v| { matches!(v, RuntimeValue::Type(_)) },
                kind BuiltIn
            }
        };

        let arr = type_signature! {
            Arr {
                match |t, v| { matches!(v, RuntimeValue::Array(_) )},
                kind BuiltIn,
                finalized |t, v| {
                    matches!(v, RuntimeValue::Array(arr) if arr.ty == t.generics[0])
                }
            }
        };

        types.add_type(unit);
        types.add_type(numerics);
        types.add_type(string);
        types.add_type(bool);
        types.add_type(fnc);
        types.add_type(typ);
        types.add_type(arr);
    }
}

fn main() {
    let file = fs::read_to_string("./dev/main.yk").unwrap();

    dbg!(&file);

    let tk = tokenize("main.yk".to_string(), file.to_string());

    dbg!(&tk);

    // let numerics = yuke_type! {
    //     Num {
    //         kind BuiltIn,
    //         children {
    //             Int {
    //                 kind BuiltIn
    //             },
    //             Flt {
    //                 kind BuiltIn
    //             }
    //         }
    //     }
    // };
    //
    // let string = yuke_type! {
    //     Str {
    //         kind BuiltIn
    //     }
    // };
    //
    // let bool = yuke_type! {
    //     Bln {
    //         kind BuiltIn
    //     }
    // };

    //
    // GlobalTypes::add_type(numerics);
    // GlobalTypes::add_type(string);
    // GlobalTypes::add_type(bool);
    // GlobalYukeTypes::add_type(fnc);
    // GlobalYukeTypes::add_type(unit);
    // GlobalYukeTypes::add_type(typ);
    // GlobalYukeTypes::add_type(arr);

    let ast = Parser { tokens: tk }.ast();

    let mut analysis = StaticAnalysis::new();

    analysis.analyze(ast.value.clone().into_block().unwrap());

    dbg!(&ast);

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
