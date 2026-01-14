use crate::lexer::structs::{Span, Token};
use crate::store::AtomStorage;
use crate::store::sourcemap::SourceMaps;
use colored::Colorize;
use std::fmt::Display;
use std::process::exit;

pub struct Log;

pub enum LogOrigin {
    Parse,
    Interpret,
    StaticAnalysis,
    Unnamed,
}

impl Display for LogOrigin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                LogOrigin::Parse => "Parsing",
                LogOrigin::Interpret => "Interpreting",
                LogOrigin::StaticAnalysis => "Static Analysis",
                LogOrigin::Unnamed => "Unnamed",
            }
        )
    }
}

impl Log {
    pub fn info(s: impl Into<String>, origin: LogOrigin) {
        println!(
            "{} [{}] {}",
            "@info".green(),
            origin.to_string().green(),
            s.into()
        )
    }

    pub fn err(s: impl Into<String>, origin: LogOrigin) {
        println!(
            "{} [{}] {}",
            "@err".red(),
            origin.to_string().red(),
            s.into()
        )
    }

    pub fn dbg(s: impl Into<String>, origin: LogOrigin) {
        println!(
            "{} [{}] {}",
            "@dbg".yellow(),
            origin.to_string().yellow(),
            s.into()
        )
    }

    pub fn trace_span(span: Span) {
        let line = span.start.line;
        let source = SourceMaps::get(&span.file_name);

        let line = source.get(line - 1).unwrap();
        let c1 = span.start.column.clamp(0, line.len());
        let c2 = span.end.column.clamp(0, line.len());

        let mut s1 = (&line[0..c1 - 1]).to_string();
        let mut s2 = (&line[c1 - 1..c2]).to_string();
        let s3 = (&line[c2..line.len()]).to_string();

        if c1 == c2 {
            if c1 != 0 {
                s1 = (&line[0..c1 - 1]).to_string();
                let c = line.chars().collect::<Vec<char>>();
                let k = c.get(c1 - 1).unwrap().clone().to_string().clone();
                s2 = k.to_string();
            }
        }

        println!(
            "| {} at {}:{}{} :",
            AtomStorage::string(span.file_name).unwrap().green(),
            span.start.line.to_string().green(),
            (span.start.column + 1).to_string().green(),
            if c1 != c2 {
                format!(
                    " to {}:{}",
                    span.end.line.to_string().green(),
                    (span.end.column + 1).to_string().green()
                )
            } else {
                "".to_string()
            }
        );
        println!("> {}{}{}", s1, s2.green(), s3);
    }
}

pub struct Control;

impl Control {
    pub fn exit() -> ! {
        exit(0);
    }
}
