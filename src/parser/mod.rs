pub mod structs;

use std::collections::VecDeque;
use std::fmt::format;
use thiserror::Error;
use crate::lexer::structs::{Direction, KeywordType, Location, OperatorType, SignType, Span, Token, TokenValue};
use crate::log::{Control, Log, LogOrigin};
use crate::parser::structs::{ASTNode, ASTNodeValue, IfContent};
use crate::store::Atom;

pub struct Parser {
    pub tokens: VecDeque<Token>
}

#[derive(Error, Debug)]
pub enum ParserError {
    #[error("Expected token {val:?} not found.")]
    ExpectedTokenNotFound {
        val: TokenValue
    }
}

impl Parser {
    pub fn ast(&mut self) -> ASTNode {
        let mut ast = Vec::new();
        let file_name = self.tokens.front().unwrap().span.file_name;

        while !self.tokens.is_empty() && self.curr().value != TokenValue::End {
            ast.push(self.parse());

            if !self.tokens.is_empty() && self.curr().value == TokenValue::Sign(SignType::Semicolon) {
                ast.push(ASTNode {
                    span: self.curr().span,
                    value: ASTNodeValue::Unit
                });
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

        ASTNode {
            span,
            value: ASTNodeValue::Block {
                contents: ast
            }
        }
    }

    fn curr(&self) -> &Token { &self.tokens[0] }

    fn go(&mut self) -> Token { self.tokens.pop_front().unwrap() }

    fn peek(&self) -> &Token { &self.tokens[1] }

    fn parse(&mut self) -> ASTNode {
        match &self.curr().value {
            // TokenValue::Number(_) => {}
            // TokenValue::String(_) => {}
            // TokenValue::Boolean(_) => {}
            // TokenValue::Identifier(_) => {}
            TokenValue::Keyword(kw) => match kw {
                KeywordType::Let => self.parse_variable_declaration(false),
                KeywordType::Immut => self.parse_variable_declaration(true),
                KeywordType::If => self.parse_if_expression(),
                _ => self.parse_starting_point()
            },
            // TokenValue::Operator(_) => {}
            // TokenValue::Sign(_) => {}
            // TokenValue::Skip => {}
            // TokenValue::End => {}
            _ => self.parse_starting_point()
        }
    }

    fn parse_starting_point(&mut self) -> ASTNode {
        self.parse_logical_or()
    }

    fn parse_assignment(&mut self) -> ASTNode {
        let mut left = self.parse_add_expr();
        let start = left.span;

        while self.curr().value.is_any_assignment_operator() {
            if !self.curr().value.is_any_assignment_operator() {
                break
            }

            let op = self.go().value.into_operator().unwrap();

            let mut right = self.parse_add_expr();

            right.value = match op {
                OperatorType::Assign => right.value.clone(),
                OperatorType::PlusAssign => ASTNodeValue::BinaryExpression {
                    left: Box::new(left.clone()),
                    right: Box::new(right.clone()),
                    op: OperatorType::Plus
                },
                OperatorType::MinusAssign => ASTNodeValue::BinaryExpression {
                    left: Box::new(left.clone()),
                    right: Box::new(right.clone()),
                    op: OperatorType::Minus
                },
                OperatorType::MultiplyAssign => ASTNodeValue::BinaryExpression {
                    left: Box::new(left.clone()),
                    right: Box::new(right.clone()),
                    op: OperatorType::Multiply
                },
                OperatorType::DivideAssign => ASTNodeValue::BinaryExpression {
                    left: Box::new(left.clone()),
                    right: Box::new(right.clone()),
                    op: OperatorType::Divide
                },
                OperatorType::ModuloAssign => ASTNodeValue::BinaryExpression {
                    left: Box::new(left.clone()),
                    right: Box::new(right.clone()),
                    op: OperatorType::Modulo
                },
                _ => unreachable!()
            };

            left = ASTNode {
                span: Span {
                    file_name: start.file_name,
                    start: start.start,
                    end: right.span.end
                },
                value: ASTNodeValue::Assignment {
                    prop: Box::new(left),
                    value: Box::new(right)
                }
            }
        }

        left
    }

    fn parse_multiply_expr(&mut self) -> ASTNode {
        let mut left = self.parse_atom();
        let start = left.span;

        while self.curr().value == TokenValue::Operator(OperatorType::Multiply) ||
            self.curr().value == TokenValue::Operator(OperatorType::Divide) ||
            self.curr().value == TokenValue::Operator(OperatorType::Modulo) {
            if !(self.curr().value == TokenValue::Operator(OperatorType::Multiply) ||
                self.curr().value == TokenValue::Operator(OperatorType::Divide) ||
                self.curr().value == TokenValue::Operator(OperatorType::Modulo)) {
                break
            }

            let op = self.go().value.into_operator().unwrap();

            let right = self.parse_atom();

            left = ASTNode {
                span: Span {
                    file_name: start.file_name,
                    start: start.start,
                    end: right.span.end
                },
                value: ASTNodeValue::BinaryExpression {
                    left: Box::new(left),
                    right: Box::new(right),
                    op
                }
            }
        }

        left
    }

    fn parse_add_expr(&mut self) -> ASTNode {
        let mut left = self.parse_multiply_expr();
        let start = left.span;

        while self.curr().value == TokenValue::Operator(OperatorType::Plus) || self.curr().value == TokenValue::Operator(OperatorType::Minus) {
            if !(self.curr().value == TokenValue::Operator(OperatorType::Plus) || self.curr().value == TokenValue::Operator(OperatorType::Minus)) {
                break
            }

            let op = self.go().value.into_operator().unwrap();

            let right = self.parse_multiply_expr();

            left = ASTNode {
                span: Span {
                    file_name: start.file_name,
                    start: start.start,
                    end: right.span.end
                },
                value: ASTNodeValue::BinaryExpression {
                    left: Box::new(left),
                    right: Box::new(right),
                    op
                }
            }
        }

        left
    }

    fn parse_atom(&mut self) -> ASTNode {
        let tk = self.go();
        match tk.value {
            TokenValue::Number(v) => ASTNode {
                span: tk.span,
                value: ASTNodeValue::Number(v)
            },
            TokenValue::String(v) => ASTNode {
                span: tk.span,
                value: ASTNodeValue::String(v)
            },
            TokenValue::Boolean(v) => ASTNode {
                span: tk.span,
                value: ASTNodeValue::Boolean(v)
            },
            TokenValue::Identifier(v) => ASTNode {
                span: tk.span,
                value: ASTNodeValue::Identifier(v)
            },
            _ => {
                Log::err(format!("Token {:?} couldn't be parsed into an atom.", &tk.value), LogOrigin::Parse);
                Log::trace_span(tk.span);
                Control::exit();
            }
        }
    }

    fn parse_variable_declaration(&mut self, is_immut: bool) -> ASTNode {
        let start = self.go(); // let / immut
        let ident_tk = self.expected(
            |v| matches!(v, TokenValue::Identifier(_)),
            "Expected an Identifier, found %s."
        ).unwrap();

        let ident = ident_tk.value.into_identifier().unwrap();

        _ =self.expected(
            |v| matches!(v, TokenValue::Operator(OperatorType::Assign)),
            "Expected '=', found %s."
        );

        let value = self.parse();

        ASTNode {
            span: Span { file_name: start.span.file_name, start: start.span.start, end: value.span.end },
            value: ASTNodeValue::VariableDeclaration {
                name: ident,
                value: Box::new(value),
                immut: is_immut
            }
        }
    }

    fn expected(&mut self, matches: impl Fn(&TokenValue) -> bool, reason: impl Into<String>) -> Result<Token, String> {
        if !matches(&self.curr().value) {
            Log::err(reason.into().replace("%s", &format!("{:?}", &self.curr().value)), LogOrigin::Parse);
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
                break
            }

            let op = self.go().value.into_operator().unwrap();

            let right = self.parse_logical_and();

            left = ASTNode {
                span: Span {
                    file_name: start.file_name,
                    start: start.start,
                    end: right.span.end
                },
                value: ASTNodeValue::BinaryExpression {
                    op,
                    left: Box::new(left),
                    right: Box::new(right)
                }
            }
        }

        left
    }

    fn parse_logical_and(&mut self) -> ASTNode {
        let mut left = self.parse_relations();
        let start = left.span;

        while self.curr().value == TokenValue::Operator(OperatorType::LogicalAnd) {
            if self.curr().value != TokenValue::Operator(OperatorType::LogicalAnd) {
                break
            }

            let op = self.go().value.into_operator().unwrap();

            let right = self.parse_relations();

            left = ASTNode {
                span: Span {
                    file_name: start.file_name,
                    start: start.start,
                    end: right.span.end
                },
                value: ASTNodeValue::BinaryExpression {
                    op,
                    left: Box::new(left),
                    right: Box::new(right)
                }
            }
        }

        left
    }

    fn parse_relations(&mut self) -> ASTNode {
        let mut left = self.parse_assignment();
        let start = left.span;

        while self.curr().value.is_any_relation_operator() {
            if !self.curr().value.is_any_relation_operator() {
                break
            }

            let op = self.go().value.into_operator().unwrap();

            let right = self.parse_assignment();

            left = ASTNode {
                span: Span {
                    file_name: start.file_name,
                    start: start.start,
                    end: right.span.end
                },
                value: ASTNodeValue::BinaryExpression {
                    op,
                    left: Box::new(left),
                    right: Box::new(right)
                }
            }
        }

        left
    }

    fn parse_code_block(&mut self) -> ASTNode {
        let _ =self.expected(
            |v| v == &TokenValue::Sign(SignType::CurlyBrace(Direction::Open)),
            "Expected an '{', found %s."
        );

        let mut content = Vec::new();
        let file_name = self.tokens.front().unwrap().span.file_name;

        while self.curr().value != TokenValue::Sign(SignType::CurlyBrace(Direction::Close)) && self.curr().value != TokenValue::End {
            content.push(self.parse());

            if self.curr().value != TokenValue::Sign(SignType::CurlyBrace(Direction::Close)) && self.curr().value == TokenValue::Sign(SignType::Semicolon) {
                content.push(ASTNode {
                    span: self.curr().span,
                    value: ASTNodeValue::Unit
                });
                self.go();
            }
        }

        let span = if !content.is_empty() {
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

        let _ =self.expected(
            |v| v == &TokenValue::Sign(SignType::CurlyBrace(Direction::Close)),
            "Expected an '}', found %s."
        );

        ASTNode {
            span,
            value: ASTNodeValue::Block {
                contents: content
            }
        }
    }

    fn parse_if_expression(&mut self) -> ASTNode {
        let f = self.go(); // `if`
        let mut f_span = f.span;

        let condition = self.parse();

        let block = self.parse_code_block();

        let mut ifs: Vec<IfContent> = Vec::new();

        ifs.push(IfContent {
            condition: Box::new(condition),
            block: Box::new(block)
        });

        while self.curr().value == TokenValue::Keyword(KeywordType::Else)
            && self.peek().value == TokenValue::Keyword(KeywordType::If) {
            self.go(); self.go(); // `else` `if`

            let c = self.parse();
            let b = self.parse_code_block();

            f_span.end = b.span.end;

            ifs.push(IfContent {
                condition: Box::new(c),
                block: Box::new(b)
            });
        }

        let mut or_else: Option<Box<ASTNode>> = None;

        if self.curr().value == TokenValue::Keyword(KeywordType::Else) {
            self.go();
            let b = self.parse_code_block();
            f_span.end = b.span.end;
            or_else = Some(Box::new(b));
        }

        ASTNode {
            span: f_span,
            value: ASTNodeValue::If {
                ifs,
                or_else
            }
        }
    }
}