use std::collections::HashMap;
use collection_literals::hash;
use enum_as_inner::EnumAsInner;
use lazy_static::lazy_static;
use crate::store::{Atom, AtomStorage};

#[derive(Debug, Clone, Copy)]
pub struct Location {
    pub file_name: Atom,
    pub line: usize, pub column: usize
}

#[derive(Debug, Clone, Copy)]
pub struct OnlyLocation {
    pub line: usize, pub column: usize
}

impl Location {
    pub fn only(line: usize, column: usize) -> OnlyLocation {
        OnlyLocation { line, column }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Span {
    pub(crate) file_name: Atom,
    pub(crate) start: OnlyLocation,
    pub(crate) end: OnlyLocation
}

#[derive(Debug, Clone)]
pub struct Token {
    pub span: Span,
    pub value: TokenValue,
}

#[derive(Debug, Clone, PartialEq)]
#[derive(EnumAsInner)]
pub enum TokenValue {
    Number(f64),
    String(Atom),
    Boolean(bool),
    Identifier(Atom),
    Keyword(KeywordType),
    Operator(OperatorType),
    Sign(SignType),
    /// note: should not be used outside lexing process
    Skip,
    End,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum KeywordType {
    Use,
    Pub,
    Immut,
    Struct,
    Fn,
    Def,
    Method,
    Feat,
    Let,
    For,
    While,
    If,
    Else,
    When
}


impl TokenValue {
    pub fn is_any_assignment_operator(&self) -> bool {
        self == &TokenValue::Operator(OperatorType::Assign) ||
            self == &TokenValue::Operator(OperatorType::PlusAssign) ||
            self == &TokenValue::Operator(OperatorType::MinusAssign) ||
            self == &TokenValue::Operator(OperatorType::DivideAssign) ||
            self == &TokenValue::Operator(OperatorType::ModuloAssign)
    }

    pub fn is_any_relation_operator(&self) -> bool {
        self == &TokenValue::Operator(OperatorType::Equality) ||
            self == &TokenValue::Operator(OperatorType::Inequality) ||
            self == &TokenValue::Operator(OperatorType::Bigger) ||
            self == &TokenValue::Operator(OperatorType::Smaller) ||
            self == &TokenValue::Operator(OperatorType::BiggerEqual) ||
            self == &TokenValue::Operator(OperatorType::SmallerEqual)
    }
}

lazy_static! {
    pub static ref RESERVED_KEYWORDS: HashMap<Atom, KeywordType> = hash! {
        map of &'static str => KeywordType {
            "use" => KeywordType::Use,
            "pub" => KeywordType::Pub,
            "immut" => KeywordType::Immut,
            "struct" => KeywordType::Struct,
            "fn" => KeywordType::Fn,
            "def" => KeywordType::Def,
            "method" => KeywordType::Method,
            "feat" => KeywordType::Feat,
            "let" => KeywordType::Let,
            "for" => KeywordType::For,
            "while" => KeywordType::While,
            "if" => KeywordType::If,
            "else" => KeywordType::Else,
            "when" => KeywordType::When
        }
    }.into_iter().map(|(k, v)| (AtomStorage::atom(k.to_string()), v)).collect();

    pub static ref SIMPLE_OPERATORS: HashMap<&'static str, OperatorType> = hash! {
        map of &'static str => OperatorType {
            "+" => OperatorType::Plus,
            "-" => OperatorType::Minus,
            "*" => OperatorType::Multiply,
            "/" => OperatorType::Divide,
            "%" => OperatorType::Modulo,
            ">" => OperatorType::Bigger,
            "<" => OperatorType::Smaller,
            "=" => OperatorType::Assign
        }
    };

    pub static ref SIMPLE_SIGNS: HashMap<char, SignType> = hash! {
        map of char => SignType {
            ',' => SignType::Comma,
            '.' => SignType::Dot,
            '_' => SignType::Underscore,
            ';' => SignType::Semicolon,
            '?' => SignType::QuestionMk,
            ':' => SignType::Colon,
            '!' => SignType::ExclamationMk,
            '#' => SignType::HashSign,
            '^' => SignType::Caret,
            '@' => SignType::At,
            '$' => SignType::DollarSign,
            '~' => SignType::Tilde,
            '&' => SignType::Ampersand,
            '|' => SignType::Pipe
        }
    };

    pub static ref SIGN_CONVERSIONS: Vec<TwoElementSignsConversion> = vec![
        TwoElementSignsConversion {
            first: TokenValue::Operator(OperatorType::Plus),
            second: TokenValue::Operator(OperatorType::Plus),
            result: TokenValue::Operator(OperatorType::Increment),
        }, // ++
        TwoElementSignsConversion {
            first: TokenValue::Operator(OperatorType::Minus),
            second: TokenValue::Operator(OperatorType::Minus),
            result: TokenValue::Operator(OperatorType::Decrement),
        }, // --
        TwoElementSignsConversion {
            first: TokenValue::Operator(OperatorType::Minus),
            second: TokenValue::Operator(OperatorType::Bigger),
            result: TokenValue::Sign(SignType::Arrow),
        }, // ->
        TwoElementSignsConversion {
            first: TokenValue::Operator(OperatorType::Smaller),
            second: TokenValue::Operator(OperatorType::Minus),
            result: TokenValue::Sign(SignType::BackwardArrow),
        }, // <-
        TwoElementSignsConversion {
            first: TokenValue::Operator(OperatorType::Assign),
            second: TokenValue::Operator(OperatorType::Bigger),
            result: TokenValue::Sign(SignType::EqArrow),
        }, // =>
        TwoElementSignsConversion {
            first: TokenValue::Operator(OperatorType::Bigger),
            second: TokenValue::Operator(OperatorType::Assign),
            result: TokenValue::Operator(OperatorType::BiggerEqual),
        }, // >=
        TwoElementSignsConversion {
            first: TokenValue::Operator(OperatorType::Smaller),
            second: TokenValue::Operator(OperatorType::Assign),
            result: TokenValue::Operator(OperatorType::SmallerEqual),
        }, // <=
        TwoElementSignsConversion {
            first: TokenValue::Sign(SignType::Arrow),
            second: TokenValue::Operator(OperatorType::Bigger),
            result: TokenValue::Sign(SignType::DoubleArrow),
        }, // ->>
        TwoElementSignsConversion {
            first: TokenValue::Operator(OperatorType::Assign),
            second: TokenValue::Operator(OperatorType::Assign),
            result: TokenValue::Operator(OperatorType::Equality),
        }, // ==
        TwoElementSignsConversion {
            first: TokenValue::Sign(SignType::ExclamationMk),
            second: TokenValue::Operator(OperatorType::Assign),
            result: TokenValue::Operator(OperatorType::Inequality),
        }, // !=
        TwoElementSignsConversion {
            first: TokenValue::Operator(OperatorType::Divide),
            second: TokenValue::Operator(OperatorType::Divide),
            result: TokenValue::Sign(SignType::Comment),
        }, // //
        TwoElementSignsConversion {
            first: TokenValue::Operator(OperatorType::Plus),
            second: TokenValue::Operator(OperatorType::Assign),
            result: TokenValue::Operator(OperatorType::PlusAssign),
        }, // +=
        TwoElementSignsConversion {
            first: TokenValue::Operator(OperatorType::Minus),
            second: TokenValue::Operator(OperatorType::Assign),
            result: TokenValue::Operator(OperatorType::MinusAssign),
        }, // -=
        TwoElementSignsConversion {
            first: TokenValue::Operator(OperatorType::Multiply),
            second: TokenValue::Operator(OperatorType::Assign),
            result: TokenValue::Operator(OperatorType::MultiplyAssign),
        }, // *=
        TwoElementSignsConversion {
            first: TokenValue::Operator(OperatorType::Divide),
            second: TokenValue::Operator(OperatorType::Assign),
            result: TokenValue::Operator(OperatorType::DivideAssign),
        }, // /=
        TwoElementSignsConversion {
            first: TokenValue::Operator(OperatorType::Modulo),
            second: TokenValue::Operator(OperatorType::Assign),
            result: TokenValue::Operator(OperatorType::ModuloAssign),
        }, // %=
        TwoElementSignsConversion {
            first: TokenValue::Sign(SignType::Dot),
            second: TokenValue::Sign(SignType::Dot),
            result: TokenValue::Sign(SignType::DoubleDot),
        }, // ..
        TwoElementSignsConversion {
            first: TokenValue::Operator(OperatorType::Divide),
            second: TokenValue::Operator(OperatorType::Bigger),
            result: TokenValue::Sign(SignType::SlashArrow),
        }, // />
        TwoElementSignsConversion {
            first: TokenValue::Sign(SignType::Tilde),
            second: TokenValue::Operator(OperatorType::Bigger),
            result: TokenValue::Sign(SignType::TildeArrow),
        }, // ~>
        TwoElementSignsConversion {
            first: TokenValue::Sign(SignType::Colon),
            second: TokenValue::Sign(SignType::Colon),
            result: TokenValue::Sign(SignType::DoubleColon),
        }, // ::
        TwoElementSignsConversion {
            first: TokenValue::Sign(SignType::Ampersand),
            second: TokenValue::Sign(SignType::Ampersand),
            result: TokenValue::Operator(OperatorType::LogicalAnd),
        }, // &&
        TwoElementSignsConversion {
            first: TokenValue::Sign(SignType::Pipe),
            second: TokenValue::Sign(SignType::Pipe),
            result: TokenValue::Operator(OperatorType::LogicalOr),
        }, // ||
    ];
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum OperatorType {
    Plus,
    Minus,
    Multiply,
    Divide,
    Modulo,
    Increment,
    Decrement,
    Bigger,
    Smaller,
    BiggerEqual,
    SmallerEqual,
    Equality,
    Inequality,
    Assign,
    PlusAssign,
    MinusAssign,
    MultiplyAssign,
    DivideAssign,
    ModuloAssign,
    LogicalAnd,
    LogicalOr
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignType {
    Semicolon,             // ;
    Colon,                 // :
    Comma,                 // ,
    Dot,                   // .
    Underscore,            // _
    Arrow,                 // ->
    BackwardArrow,         // <-
    ExclamationMk,         // !
    QuestionMk,            // ?
    Paren(Direction),      // ( )
    Brace(Direction),      // [ ]
    CurlyBrace(Direction), // { }
    EqArrow,               // =>
    DoubleArrow,           // ->>
    Comment,               // //
    HashSign,              // #
    Caret,                 // ^
    DoubleDot,             // ..
    SlashArrow,            // />
    At,                    // @
    Tilde,                 // ~
    TildeArrow,            // ~>
    DoubleColon,           // ::
    DollarSign,            // $
    Ampersand,             // &
    Pipe,                  // |
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Direction {
    Open,
    Close,
}


pub struct TwoElementSignsConversion {
    pub first: TokenValue,
    pub second: TokenValue,
    pub result: TokenValue,
}