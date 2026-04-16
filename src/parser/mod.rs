pub mod structs;

use crate::lexer::structs::{
    Direction, KeywordType, Location, OperatorType, SignType, Span, Token, TokenValue,
};
use crate::log::{Control, Log, LogOrigin};
use crate::parser::structs::{ASTNode, ASTNodeValue, IfContent};
use crate::store::Atom;
use std::collections::{HashMap, VecDeque};
use std::env::args;
use std::fmt::format;
use std::ptr::eq;
use thiserror::Error;

pub struct Parser {
    pub tokens: VecDeque<Token>,
}

#[derive(Error, Debug)]
pub enum ParserError {
    #[error("Expected token {val:?} not found.")]
    ExpectedTokenNotFound { val: TokenValue },
}

impl Parser {
    pub fn ast(&mut self) -> ASTNode {
        let mut ast = Vec::new();
        let file_name = self.tokens.front().unwrap().span.file_name;

        while !self.tokens.is_empty() && self.curr().value != TokenValue::End {
            ast.push(self.parse());

            if !self.tokens.is_empty() && self.curr().value == TokenValue::Sign(SignType::Semicolon)
            {
                ast.push(ASTNode::new(self.curr().span, ASTNodeValue::Unit));
                self.go();
            }
        }

        let span = if !ast.is_empty() {
            Span {
                file_name: ast.first().unwrap().span.file_name,
                start: ast.first().unwrap().span.start,
                end: ast.last().unwrap().span.end,
            }
        } else {
            Span {
                file_name,
                start: Location::only(0, 0),
                end: Location::only(0, 0),
            }
        };

        ASTNode::new(span, ASTNodeValue::Block { contents: ast })
    }

    fn curr(&self) -> &Token {
        &self.tokens[0]
    }

    fn go(&mut self) -> Token {
        self.tokens.pop_front().unwrap()
    }

    fn peek(&self) -> &Token {
        &self.tokens[1]
    }

    fn parse(&mut self) -> ASTNode {
        let v = match &self.curr().value {
            // TokenValue::Number(_) => {}
            // TokenValue::String(_) => {}
            // TokenValue::Boolean(_) => {}
            // TokenValue::Identifier(_) => {}
            TokenValue::Keyword(kw) => match kw {
                KeywordType::Let => self.parse_variable_declaration(false),
                KeywordType::Immut => self.parse_variable_declaration(true),
                KeywordType::If => self.parse_if_expression(),
                KeywordType::When => self.parse_when(),
                KeywordType::Fn => self.parse_function(),
                KeywordType::Struct => self.parse_struct_def(),
                _ => self.parse_starting_point(),
            },
            // TokenValue::Operator(_) => {}
            // TokenValue::Sign(_) => {}
            // TokenValue::Skip => {}
            // TokenValue::End => {}
            _ => self.parse_starting_point(),
        };
        //
        // self.try_parse_struct_property(v)
        v
    }

    fn parse_starting_point(&mut self) -> ASTNode {
        self.parse_logical_or()
    }

    fn parse_assignment(&mut self) -> ASTNode {
        let x = self.parse_add_expr();
        let mut left = self.try_parse_struct_property_or_method(x);
        let start = left.span;

        while self.curr().value.is_any_assignment_operator() {
            if !self.curr().value.is_any_assignment_operator() {
                break;
            }

            let op = self.go().value.into_operator().unwrap();

            let mut right = self.parse();

            right.value = match op {
                OperatorType::Assign => right.value.clone(),
                OperatorType::PlusAssign => ASTNodeValue::BinaryExpression {
                    left: Box::new(left.clone()),
                    right: Box::new(right.clone()),
                    op: OperatorType::Plus,
                },
                OperatorType::MinusAssign => ASTNodeValue::BinaryExpression {
                    left: Box::new(left.clone()),
                    right: Box::new(right.clone()),
                    op: OperatorType::Minus,
                },
                OperatorType::MultiplyAssign => ASTNodeValue::BinaryExpression {
                    left: Box::new(left.clone()),
                    right: Box::new(right.clone()),
                    op: OperatorType::Multiply,
                },
                OperatorType::DivideAssign => ASTNodeValue::BinaryExpression {
                    left: Box::new(left.clone()),
                    right: Box::new(right.clone()),
                    op: OperatorType::Divide,
                },
                OperatorType::ModuloAssign => ASTNodeValue::BinaryExpression {
                    left: Box::new(left.clone()),
                    right: Box::new(right.clone()),
                    op: OperatorType::Modulo,
                },
                _ => unreachable!(),
            };

            left = ASTNode::new(
                Span {
                    file_name: start.file_name,
                    start: start.start,
                    end: right.span.end,
                },
                ASTNodeValue::Assignment {
                    prop: Box::new(left),
                    value: Box::new(right),
                },
            )
        }

        left
    }

    fn parse_multiply_expr(&mut self) -> ASTNode {
        let mut left = self.parse_atom();
        let start = left.span;

        while self.curr().value == TokenValue::Operator(OperatorType::Multiply)
            || self.curr().value == TokenValue::Operator(OperatorType::Divide)
            || self.curr().value == TokenValue::Operator(OperatorType::Modulo)
        {
            if !(self.curr().value == TokenValue::Operator(OperatorType::Multiply)
                || self.curr().value == TokenValue::Operator(OperatorType::Divide)
                || self.curr().value == TokenValue::Operator(OperatorType::Modulo))
            {
                break;
            }

            let op = self.go().value.into_operator().unwrap();

            let right = self.parse_atom();

            left = ASTNode::new(
                Span {
                    file_name: start.file_name,
                    start: start.start,
                    end: right.span.end,
                },
                ASTNodeValue::BinaryExpression {
                    left: Box::new(left),
                    right: Box::new(right),
                    op,
                },
            )
        }

        left
    }

    fn parse_add_expr(&mut self) -> ASTNode {
        let mut left = self.parse_multiply_expr();
        let start = left.span;

        while self.curr().value == TokenValue::Operator(OperatorType::Plus)
            || self.curr().value == TokenValue::Operator(OperatorType::Minus)
        {
            if !(self.curr().value == TokenValue::Operator(OperatorType::Plus)
                || self.curr().value == TokenValue::Operator(OperatorType::Minus))
            {
                break;
            }

            let op = self.go().value.into_operator().unwrap();

            let right = self.parse_multiply_expr();

            left = ASTNode::new(
                Span {
                    file_name: start.file_name,
                    start: start.start,
                    end: right.span.end,
                },
                ASTNodeValue::BinaryExpression {
                    left: Box::new(left),
                    right: Box::new(right),
                    op,
                },
            )
        }

        left
    }

    fn parse_atom(&mut self) -> ASTNode {
        let tk = self.go();
        let node = match tk.value {
            TokenValue::Number(v) => ASTNode::new(tk.span, ASTNodeValue::Number(v)),
            TokenValue::String(v) => ASTNode::new(tk.span, ASTNodeValue::String(v)),
            TokenValue::Boolean(v) => ASTNode::new(tk.span, ASTNodeValue::Boolean(v)),
            TokenValue::Identifier(v) => ASTNode::new(tk.span, ASTNodeValue::Identifier(v)),
            TokenValue::Sign(SignType::Paren(Direction::Open)) => {
                let expr = self.parse();

                self.expected(
                    |x| matches!(x, TokenValue::Sign(SignType::Paren(Direction::Close))),
                    "Expected a ')', found %s.",
                )
                .unwrap();

                expr
            }
            TokenValue::Sign(SignType::Brace(Direction::Open)) => self.parse_array(tk),
            _ => {
                Log::err(
                    format!("Token {:?} couldn't be parsed into an atom.", &tk.value),
                    LogOrigin::Parse,
                );
                Log::trace_span(tk.span);
                Control::exit();
            }
        };

        self.parse_postfix(node)
    }

    fn parse_variable_declaration(&mut self, is_immut: bool) -> ASTNode {
        let start = self.go(); // let / immut
        let ident_tk = self
            .expected(
                |v| matches!(v, TokenValue::Identifier(_)),
                "Expected an Identifier, found %s.",
            )
            .unwrap();

        let ident = ident_tk.value.into_identifier().unwrap();

        let mut data_type: Option<Box<ASTNode>> = None;

        if self.curr().value == TokenValue::Sign(SignType::Colon) {
            self.go();

            data_type = Some(Box::new(self.parse_data_type()));
        }

        _ = self.expected(
            |v| matches!(v, TokenValue::Operator(OperatorType::Assign)),
            "Expected '=', found %s.",
        );

        let value = self.parse();

        ASTNode::new(
            Span {
                file_name: start.span.file_name,
                start: start.span.start,
                end: value.span.end,
            },
            ASTNodeValue::VariableDeclaration {
                name: ident,
                value: Box::new(value),
                immut: is_immut,
                data_type,
            },
        )
    }

    fn expected(
        &mut self,
        matches: impl Fn(&TokenValue) -> bool,
        reason: impl Into<String>,
    ) -> Result<Token, String> {
        if !matches(&self.curr().value) {
            Log::err(
                reason
                    .into()
                    .replace("%s", &format!("{:?}", &self.curr().value)),
                LogOrigin::Parse,
            );
            Log::trace_span(self.curr().span);
            Control::exit();
        } else {
            Ok(self.go())
        }
    }

    fn parse_logical_or(&mut self) -> ASTNode {
        let mut left = self.parse_logical_and();
        let start = left.span;

        while self.curr().value == TokenValue::Operator(OperatorType::LogicalOr) {
            if self.curr().value != TokenValue::Operator(OperatorType::LogicalOr) {
                break;
            }

            let op = self.go().value.into_operator().unwrap();

            let right = self.parse_logical_and();

            left = ASTNode::new(
                Span {
                    file_name: start.file_name,
                    start: start.start,
                    end: right.span.end,
                },
                ASTNodeValue::BinaryExpression {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                },
            )
        }

        left
    }

    fn parse_logical_and(&mut self) -> ASTNode {
        let mut left = self.parse_relations();
        let start = left.span;

        while self.curr().value == TokenValue::Operator(OperatorType::LogicalAnd) {
            if self.curr().value != TokenValue::Operator(OperatorType::LogicalAnd) {
                break;
            }

            let op = self.go().value.into_operator().unwrap();

            let right = self.parse_relations();

            left = ASTNode::new(
                Span {
                    file_name: start.file_name,
                    start: start.start,
                    end: right.span.end,
                },
                ASTNodeValue::BinaryExpression {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                },
            )
        }

        left
    }

    fn parse_relations(&mut self) -> ASTNode {
        let mut left = self.parse_assignment();
        let start = left.span;

        while self.curr().value.is_any_relation_operator() {
            if !self.curr().value.is_any_relation_operator() {
                break;
            }

            let op = self.go().value.into_operator().unwrap();

            let right = self.parse_assignment();

            left = ASTNode::new(
                Span {
                    file_name: start.file_name,
                    start: start.start,
                    end: right.span.end,
                },
                ASTNodeValue::BinaryExpression {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                },
            )
        }

        left
    }

    fn parse_code_block(&mut self) -> ASTNode {
        let _ = self
            .expected(
                |v| v == &TokenValue::Sign(SignType::CurlyBrace(Direction::Open)),
                "Expected an '{', found %s.",
            )
            .unwrap();

        let mut content = Vec::new();
        let file_name = self.tokens.front().unwrap().span.file_name;

        while self.curr().value != TokenValue::Sign(SignType::CurlyBrace(Direction::Close))
            && self.curr().value != TokenValue::End
        {
            content.push(self.parse());

            if self.curr().value != TokenValue::Sign(SignType::CurlyBrace(Direction::Close))
                && self.curr().value == TokenValue::Sign(SignType::Semicolon)
            {
                content.push(ASTNode::new(self.curr().span, ASTNodeValue::Unit));
                self.go();
            }
        }

        let mut span = if !content.is_empty() {
            Span {
                file_name: content.first().unwrap().span.file_name,
                start: content.first().unwrap().span.start,
                end: content.last().unwrap().span.end,
            }
        } else {
            Span {
                file_name,
                start: Location::only(0, 0),
                end: Location::only(0, 0),
            }
        };

        let lst = self
            .expected(
                |v| v == &TokenValue::Sign(SignType::CurlyBrace(Direction::Close)),
                "Expected an '}', found %s.",
            )
            .unwrap();

        span.end = lst.span.end;

        ASTNode::new(span, ASTNodeValue::Block { contents: content })
    }

    fn parse_if_expression(&mut self) -> ASTNode {
        let f = self.go(); // `if`
        let mut f_span = f.span;

        let condition = self.parse();

        let block = self.parse_code_block();

        let mut ifs: Vec<IfContent> = Vec::new();

        ifs.push(IfContent {
            condition: Box::new(condition),
            block: Box::new(block),
        });

        while self.curr().value == TokenValue::Keyword(KeywordType::Else)
            && self.peek().value == TokenValue::Keyword(KeywordType::If)
        {
            self.go();
            self.go(); // `else` `if`

            let c = self.parse();
            let b = self.parse_code_block();

            f_span.end = b.span.end;

            ifs.push(IfContent {
                condition: Box::new(c),
                block: Box::new(b),
            });
        }

        let mut or_else: Option<Box<ASTNode>> = None;

        if self.curr().value == TokenValue::Keyword(KeywordType::Else) {
            self.go();
            let b = self.parse_code_block();
            f_span.end = b.span.end;
            or_else = Some(Box::new(b));
        }

        ASTNode::new(f_span, ASTNodeValue::If { ifs, or_else })
    }

    fn parse_data_type(&mut self) -> ASTNode {
        let mut dynamic = false;
        if self.curr().value == TokenValue::Sign(SignType::QuestionMk) {
            self.go();
            dynamic = true;
        }

        let ident_tk = self
            .expected(
                |v| matches!(v, TokenValue::Identifier(_)),
                "Expected an Identifier, found %s.",
            )
            .unwrap();

        let ident = ident_tk.value.into_identifier().unwrap();

        let mut span = ident_tk.span;

        let mut res: Vec<Atom> = vec![ident];

        while self.curr().value == TokenValue::Sign(SignType::Dot) {
            self.go();

            let i_tk = self
                .expected(
                    |v| matches!(v, TokenValue::Identifier(_)),
                    "Expected an Identifier, found %s.",
                )
                .unwrap();

            span.end = i_tk.span.end;
            let i = i_tk.value.into_identifier().unwrap();

            res.push(i);
        }

        let mut generics: Vec<ASTNode> = Vec::new();

        if self.curr().value == TokenValue::Operator(OperatorType::Smaller) {
            self.go();

            generics.push(self.parse_data_type());

            while self.curr().value == TokenValue::Sign(SignType::Comma)
                && self.not_end()
                && self.curr().value != TokenValue::Operator(OperatorType::Bigger)
            {
                self.go();

                let tp = self.parse_data_type();

                span.end = tp.span.end;

                generics.push(tp)
            }

            self.expected(
                |v| matches!(v, TokenValue::Operator(OperatorType::Bigger)),
                "Expected '>', found %s.",
            )
            .unwrap();
        }

        ASTNode::new(
            span,
            ASTNodeValue::Type {
                dynamic,
                content: res,
                generics,
            },
        )
    }

    fn parse_when(&mut self) -> ASTNode {
        let wn = self.go(); // `when`

        // let value = self.parse();

        let _ = self
            .expected(
                |v| v == &TokenValue::Sign(SignType::CurlyBrace(Direction::Open)),
                "Expected an '{', found %s.",
            )
            .unwrap();

        let mut ifs: Vec<IfContent> = Vec::new();

        while self.curr().value != TokenValue::Sign(SignType::CurlyBrace(Direction::Close))
            && self.curr().value != TokenValue::End
        {
            let eq_to = self.parse();

            let block = self.parse_code_block();

            ifs.push(IfContent {
                block: Box::new(block),
                condition: Box::new(eq_to),
            })
        }

        let mut last = self
            .expected(
                |v| v == &TokenValue::Sign(SignType::CurlyBrace(Direction::Close)),
                "Expected an '}', found %s.",
            )
            .unwrap();

        let mut or_else: Option<Box<ASTNode>> = None;

        if self.curr().value == TokenValue::Keyword(KeywordType::Else) {
            self.go(); // `else`

            let bl = self.parse_code_block();
            last.span.end = bl.span.end;
            or_else = Some(Box::new(bl));
        }

        ASTNode::new(
            Span {
                file_name: wn.span.file_name,
                start: wn.span.start,
                end: last.span.end,
            },
            ASTNodeValue::When {
                // value: Box::new(value),
                ifs,
                or_else,
            },
        )
    }

    fn parse_function(&mut self) -> ASTNode {
        let f = self.go(); // `fn`
        //

        if matches!(self.curr().value, TokenValue::Sign(SignType::At)) {
            self.go();

            return self.parse_method(f);
        }

        let mut named: Option<Atom> = None;

        if matches!(self.curr().value, TokenValue::Identifier(_)) {
            named = Some(self.go().value.into_identifier().unwrap());
        }

        self.expected(
            |v| matches!(v, TokenValue::Sign(SignType::Paren(Direction::Open))),
            "Expected an '(', found %s.",
        )
        .unwrap();

        let mut args_name: Vec<Atom> = Vec::new();

        let mut args_type: Vec<ASTNode> = Vec::new();

        if self.curr().value != TokenValue::Sign(SignType::Paren(Direction::Close)) {
            let ident_tk = self
                .expected(
                    |v| matches!(v, TokenValue::Identifier(_)),
                    "Expected an Identifier, found %s.",
                )
                .unwrap();
            self.expected(
                |v| matches!(v, TokenValue::Sign(SignType::Colon)),
                "Expected a ':', found %s.",
            )
            .unwrap();
            let ty = self.parse_data_type();

            args_name.push(ident_tk.value.into_identifier().unwrap());
            args_type.push(ty);

            while self.not_end()
                && self.curr().value == TokenValue::Sign(SignType::Comma)
                && self.curr().value != TokenValue::Sign(SignType::Paren(Direction::Close))
            {
                self.go();

                let ident_tk = self
                    .expected(
                        |v| matches!(v, TokenValue::Identifier(_)),
                        "Expected an Identifier, found %s.",
                    )
                    .unwrap();
                self.expected(
                    |v| matches!(v, TokenValue::Sign(SignType::Colon)),
                    "Expected a ':', found %s.",
                )
                .unwrap();
                let ty = self.parse_data_type();

                args_name.push(ident_tk.value.into_identifier().unwrap());
                args_type.push(ty);
            }
        }

        self.expected(
            |v| matches!(v, TokenValue::Sign(SignType::Paren(Direction::Close))),
            "Expected an ')', found %s.",
        )
        .unwrap();

        self.expected(
            |v| matches!(v, TokenValue::Sign(SignType::Arrow)),
            "Expected an '->', found %s.",
        )
        .unwrap();

        let ret = self.parse_data_type();

        let body = self.parse_code_block();

        let fun = ASTNode::new(
            Span {
                file_name: f.span.file_name,
                start: f.span.start,
                end: f.span.end,
            },
            ASTNodeValue::Function {
                arg_names: args_name,
                arg_types: args_type,
                body: Box::new(body),
                ret_type: Box::new(ret),
            },
        );
        if let Some(name) = named {
            ASTNode::new(
                fun.span,
                ASTNodeValue::VariableDeclaration {
                    name,
                    immut: true,
                    value: Box::new(fun),
                    data_type: None,
                },
            )
        } else {
            fun
        }
    }

    fn not_end(&self) -> bool {
        self.curr().value != TokenValue::End
    }

    fn try_parse_call(&mut self, call: ASTNode) -> ASTNode {
        if self.curr().value == TokenValue::Sign(SignType::Paren(Direction::Open)) {
            self.go(); // `(`

            if self.curr().value == TokenValue::Sign(SignType::Paren(Direction::Close)) {
                let l = self.go();

                let mut sp = call.span;
                sp.end = l.span.end;
                return ASTNode::new(
                    sp,
                    ASTNodeValue::Call {
                        args: Vec::new(),
                        on: Box::new(call),
                    },
                );
            }

            let mut args = Vec::new();

            args.push(self.parse());

            while self.not_end()
                && self.curr().value == TokenValue::Sign(SignType::Comma)
                && self.curr().value != TokenValue::Sign(SignType::Paren(Direction::Close))
            {
                self.go(); // `,`

                args.push(self.parse());
            }

            let l = self
                .expected(
                    |v| matches!(v, TokenValue::Sign(SignType::Paren(Direction::Close))),
                    "Expected an ')', found %s.",
                )
                .unwrap();

            let mut sp = call.span;
            sp.end = l.span.end;

            ASTNode::new(
                sp,
                ASTNodeValue::Call {
                    args,
                    on: Box::new(call),
                },
            )
        } else {
            call
        }
    }

    fn parse_struct_def(&mut self) -> ASTNode {
        let f = self.go(); // `struct`

        self.expected(
            |v| matches!(v, TokenValue::Sign(SignType::CurlyBrace(Direction::Open))),
            "Expected an '{', found %s.",
        )
        .unwrap();

        let mut args_name: Vec<Atom> = Vec::new();

        let mut args_type: Vec<ASTNode> = Vec::new();

        if self.curr().value != TokenValue::Sign(SignType::CurlyBrace(Direction::Close)) {
            let ident_tk = self
                .expected(
                    |v| matches!(v, TokenValue::Identifier(_)),
                    "Expected an Identifier, found %s.",
                )
                .unwrap();
            self.expected(
                |v| matches!(v, TokenValue::Sign(SignType::Colon)),
                "Expected a ':', found %s.",
            )
            .unwrap();
            let ty = self.parse_data_type();

            args_name.push(ident_tk.value.into_identifier().unwrap());
            args_type.push(ty);

            while self.not_end()
                && self.curr().value == TokenValue::Sign(SignType::Comma)
                && self.curr().value != TokenValue::Sign(SignType::CurlyBrace(Direction::Close))
            {
                self.go();

                let ident_tk = self
                    .expected(
                        |v| matches!(v, TokenValue::Identifier(_)),
                        "Expected an Identifier, found %s.",
                    )
                    .unwrap();
                self.expected(
                    |v| matches!(v, TokenValue::Sign(SignType::Colon)),
                    "Expected a ':', found %s.",
                )
                .unwrap();
                let ty = self.parse_data_type();

                args_name.push(ident_tk.value.into_identifier().unwrap());
                args_type.push(ty);
            }
        }

        let l = self
            .expected(
                |v| matches!(v, TokenValue::Sign(SignType::CurlyBrace(Direction::Close))),
                "Expected an '}', found %s.",
            )
            .unwrap();

        ASTNode::new(
            Span {
                file_name: f.span.file_name,
                start: f.span.start,
                end: l.span.end,
            },
            ASTNodeValue::StructDefinition {
                prop_names: args_name,
                prop_types: args_type,
            },
        )
    }

    fn try_parse_struct_creation(&mut self, name: ASTNode) -> ASTNode {
        if self.curr().value == TokenValue::Sign(SignType::Dot)
            && self.peek().value == TokenValue::Sign(SignType::CurlyBrace(Direction::Open))
        {
            self.go();
            self.go();

            let mut values: HashMap<Atom, ASTNode> = HashMap::new();

            if self.curr().value != TokenValue::Sign(SignType::CurlyBrace(Direction::Close)) {
                let ident_tk = self
                    .expected(
                        |v| matches!(v, TokenValue::Identifier(_)),
                        "Expected an Identifier, found %s.",
                    )
                    .unwrap();
                self.expected(
                    |v| matches!(v, TokenValue::Sign(SignType::Colon)),
                    "Expected a ':', found %s.",
                )
                .unwrap();
                let ty = self.parse();

                values.insert(ident_tk.value.into_identifier().unwrap(), ty);

                while self.not_end()
                    && self.curr().value == TokenValue::Sign(SignType::Comma)
                    && self.curr().value != TokenValue::Sign(SignType::CurlyBrace(Direction::Close))
                {
                    self.go();

                    let ident_tk = self
                        .expected(
                            |v| matches!(v, TokenValue::Identifier(_)),
                            "Expected an Identifier, found %s.",
                        )
                        .unwrap();
                    self.expected(
                        |v| matches!(v, TokenValue::Sign(SignType::Colon)),
                        "Expected a ':', found %s.",
                    )
                    .unwrap();
                    let ty = self.parse();

                    values.insert(ident_tk.value.into_identifier().unwrap(), ty);
                }
            }

            let l = self
                .expected(
                    |v| matches!(v, TokenValue::Sign(SignType::CurlyBrace(Direction::Close))),
                    "Expected an '}', found %s.",
                )
                .unwrap();

            ASTNode::new(
                Span::between(name.span, l.span),
                ASTNodeValue::StructCreation {
                    name: name.value.into_identifier().unwrap(),
                    props: values,
                },
            )
        } else {
            name
        }
    }

    fn try_parse_struct_property_or_method(&mut self, on: ASTNode) -> ASTNode {
        if self.curr().value == TokenValue::Sign(SignType::Dot)
            && matches!(self.peek().value, TokenValue::Identifier(_))
        {
            self.go();
            let name_tk = self.go();
            let name = name_tk.value.into_identifier().unwrap();

            let o_span = on.span;

            if matches!(self.curr().value, TokenValue::Sign(SignType::Paren(Direction::Open))) {
                self.go(); // `(`

                let mut args = Vec::new();

                args.push(on.clone());

                if self.curr().value == TokenValue::Sign(SignType::Paren(Direction::Close)) {
                    let l = self.go();

                    let mut sp = on.span;
                    sp.end = l.span.end;
                    return ASTNode::new(
                        sp,
                        ASTNodeValue::Call {
                            args,
                            on: Box::new(
                                ASTNode::new(
                                    name_tk.span,
                                    ASTNodeValue::Identifier(
                                        name
                                    )
                                )
                            ),
                        },
                    );
                }

                args.push(self.parse());

                while self.not_end()
                    && self.curr().value == TokenValue::Sign(SignType::Comma)
                    && self.curr().value != TokenValue::Sign(SignType::Paren(Direction::Close))
                {
                    self.go(); // `,`

                    args.push(self.parse());
                }

                let l = self
                    .expected(
                        |v| matches!(v, TokenValue::Sign(SignType::Paren(Direction::Close))),
                        "Expected an ')', found %s.",
                    )
                    .unwrap();

                let mut sp = on.span;
                sp.end = l.span.end;

                return ASTNode::new(
                    sp,
                    ASTNodeValue::Call {
                        args,
                        on: Box::new(
                            ASTNode::new(
                                name_tk.span,
                                ASTNodeValue::Identifier(
                                    name
                                )
                            )
                        ),
                    },
                )
            }

            ASTNode::new(
                Span::between(o_span, name_tk.span),
                ASTNodeValue::PropertyAccess {
                    on: Box::new(on),
                    property: name,
                },
            )
        } else {
            on
        }
    }

    fn parse_array(&mut self, tk: Token) -> ASTNode {
        let mut ty: Option<Box<ASTNode>> = None;

        if self.curr().value == TokenValue::Operator(OperatorType::Smaller) {
            self.go();
            ty = Some(Box::new(self.parse_data_type()));

            self.expected(
                |v| matches!(v, TokenValue::Operator(OperatorType::Bigger)),
                "Expected a '>', found %s.",
            )
            .unwrap();
        }

        if self.curr().value == TokenValue::Sign(SignType::Brace(Direction::Close)) {
            let l = self.go();

            let mut sp = tk.span;
            sp.end = l.span.end;
            return ASTNode::new(
                sp,
                ASTNodeValue::ArrayDeclaration {
                    ty,
                    values: Vec::new(),
                },
            );
        }

        let mut values = Vec::new();

        values.push(self.parse());

        while self.not_end()
            && self.curr().value == TokenValue::Sign(SignType::Comma)
            && self.curr().value != TokenValue::Sign(SignType::Brace(Direction::Close))
        {
            self.go(); // `,`

            values.push(self.parse());
        }

        let l = self
            .expected(
                |v| matches!(v, TokenValue::Sign(SignType::Brace(Direction::Close))),
                "Expected an ']', found %s.",
            )
            .unwrap();

        let mut sp = tk.span;
        sp.end = l.span.end;

        ASTNode::new(sp, ASTNodeValue::ArrayDeclaration { values, ty })
    }

    fn parse_postfix(&mut self, mut node: ASTNode) -> ASTNode {
        loop {
            match &self.curr().value {
                // a.b | A.{ }
                TokenValue::Sign(SignType::Dot) => {
                    if matches!(
                        self.peek().value,
                        TokenValue::Sign(SignType::CurlyBrace(Direction::Open))
                    ) {
                        node = self.try_parse_struct_creation(node);
                    } else {
                        node = self.try_parse_struct_property_or_method(node);
                    }
                }
                // a()
                TokenValue::Sign(SignType::Paren(Direction::Open)) => {
                    node = self.try_parse_call(node);
                }
                // a[..]
                TokenValue::Sign(SignType::Brace(Direction::Open)) => {
                    node = self.parse_array_access(node);
                }
                _ => break,
            }
        }
        node
    }

    fn parse_array_access(&mut self, node: ASTNode) -> ASTNode {
        let s = self.go();
        let idx = self.parse();
        let l = self
            .expected(
                |x| matches!(x, TokenValue::Sign(SignType::Brace(Direction::Close))),
                "Expected a ']', found %s.",
            )
            .unwrap();

        ASTNode::new(
            Span::between(s.span, l.span),
            ASTNodeValue::ArrayAccess {
                on: Box::new(node),
                index: Box::new(idx),
            },
        )
    }

    fn parse_method(&mut self, fn_token: Token /* for span */) -> ASTNode {
        let data_type = self.parse_data_type();

        self.expected(|v| matches!(v, TokenValue::Sign(SignType::Colon)), "Expected a ':', found %s.").unwrap();

        let ident = self.expected(|v| v.is_identifier(), "Expected an Identifier, found %s.").unwrap();

        let arg_name = ident.value.as_identifier().unwrap();

        let fn_name_tk = self.expected(|v| v.is_identifier(), "Expected an Identifier, found %s.").unwrap();

        let fn_name = fn_name_tk.value.as_identifier().unwrap();

        self.expected(
            |v| matches!(v, TokenValue::Sign(SignType::Paren(Direction::Open))),
            "Expected an '(', found %s.",
        )
            .unwrap();

        let mut args_name: Vec<Atom> = Vec::new();

        args_name.push(*arg_name);

        let mut args_type: Vec<ASTNode> = Vec::new();

        args_type.push(data_type.clone());

        if self.curr().value != TokenValue::Sign(SignType::Paren(Direction::Close)) {
            let ident_tk = self
                .expected(
                    |v| matches!(v, TokenValue::Identifier(_)),
                    "Expected an Identifier, found %s.",
                )
                .unwrap();
            self.expected(
                |v| matches!(v, TokenValue::Sign(SignType::Colon)),
                "Expected a ':', found %s.",
            )
                .unwrap();
            let ty = self.parse_data_type();

            args_name.push(ident_tk.value.into_identifier().unwrap());
            args_type.push(ty);

            while self.not_end()
                && self.curr().value == TokenValue::Sign(SignType::Comma)
                && self.curr().value != TokenValue::Sign(SignType::Paren(Direction::Close))
            {
                self.go();

                let ident_tk = self
                    .expected(
                        |v| matches!(v, TokenValue::Identifier(_)),
                        "Expected an Identifier, found %s.",
                    )
                    .unwrap();
                self.expected(
                    |v| matches!(v, TokenValue::Sign(SignType::Colon)),
                    "Expected a ':', found %s.",
                )
                    .unwrap();
                let ty = self.parse_data_type();

                args_name.push(ident_tk.value.into_identifier().unwrap());
                args_type.push(ty);
            }
        }

        self.expected(
            |v| matches!(v, TokenValue::Sign(SignType::Paren(Direction::Close))),
            "Expected an ')', found %s.",
        )
            .unwrap();

        self.expected(
            |v| matches!(v, TokenValue::Sign(SignType::Arrow)),
            "Expected an '->', found %s.",
        )
            .unwrap();

        let ret = self.parse_data_type();

        let body = self.parse_code_block();

        ASTNode::new(
            Span::between(fn_token.span, (&body).span),
            ASTNodeValue::Method {
                name: *fn_name,
                data_type: Box::new(data_type),
                fn_ast: Box::new(
                    ASTNode::new(
                        Span::between(fn_token.span, (&body).span),
                        ASTNodeValue::Function {
                            ret_type: Box::new(ret),
                            arg_names: args_name,
                            arg_types: args_type,
                            body: Box::new(body)
                        }
                    )
                )
            }
        )
    }
}
