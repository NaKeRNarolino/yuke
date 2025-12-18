use std::fs;
use crate::interpret::{Interpreter, RuntimeScope};
use crate::lexer::tokenize;
use crate::log::{Log, LogOrigin};
use crate::parser::Parser;
use crate::store::AtomStorage;
use crate::util::arw;

pub mod lexer;
pub mod store;
pub mod parser;
pub mod interpret;
mod util;
mod log;

fn main() {
    let file = fs::read_to_string("./dev/main.yk").unwrap();

    dbg!(&file);

    let tk = tokenize("main.yk".to_string(), file.to_string());

    dbg!(&tk);

    let ast = Parser { tokens: tk }.ast();

    dbg!(&ast);
    let i = Interpreter { };

    let scope = RuntimeScope::new();

    let arw = arw(scope);
    let ev = i.eval_node(
        ast, arw.clone()
    );


    //
    dbg!(&ev);
    //
    // dbg!(&arw.r().get_variable(
    //     AtomStorage::atom("x".to_string())
    // ).r().value.r());
}
