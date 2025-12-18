use std::collections::VecDeque;

use crate::lexer::structs::{Direction, Location, SignType, Token, TokenValue, RESERVED_KEYWORDS, SIGN_CONVERSIONS, SIMPLE_OPERATORS, SIMPLE_SIGNS, OnlyLocation, Span};
use regex::RegexBuilder;
use crate::store::{AtomStorage, AtomStorageError};
use crate::store::sourcemap::SourceMaps;

pub mod structs;

fn is_skippable(input: char) -> bool {
    input == ' ' || input == '\n' || input == '\r'
}

fn resolve_string_to_token(input: String) -> TokenValue {
    let atom_true = AtomStorage::atom("true".to_string());
    let atom_false = AtomStorage::atom("false".to_string());
    let atom_input= AtomStorage::atom(input);
    if let Some(token) = RESERVED_KEYWORDS.get(&atom_input).cloned() {
        TokenValue::Keyword(token)
    } else if atom_input == atom_true || atom_input == atom_false {
        TokenValue::Boolean(if atom_input == atom_true {
            true
        } else {
            false
        })
    } else {
        TokenValue::Identifier(atom_input)
    }
}

fn advance_cursor(line: &mut usize, column: &mut usize, c: char) {
    if c == '\n' {
        *line += 1;
        *column = 0;
    } else {
        *column += 1;
    }
}

pub fn tokenize(file_name: String, raw_input: String) -> VecDeque<Token> {
    let mut tokens: Vec<Token> = vec![];

    let input = RegexBuilder::new(r"(\/\/).*")
        .build()
        .unwrap()
        .replace_all(&raw_input, "")
        .to_string();

    let file_name_atom = AtomStorage::atom(file_name);

    SourceMaps::push(
        file_name_atom, input.clone().split("\n").map(|x| x.to_string()).collect()
    );

    let mut input_chars: VecDeque<char> = input.chars().collect();
    let mut making_string: bool = false;
    let mut string = String::new();
    let mut prev_char: char = '\r';
    let mut line = 1;
    let mut column = 0;
    let mut string_start_location: Option<OnlyLocation> = None;

    while let Some(char) = input_chars.pop_front() {
        let start_location = Location::only(line, column + 1);

        advance_cursor(&mut line, &mut column, char);
        let end_location = Location::only(line, column);

        if char == '"' && prev_char != '\\' {
            prev_char = char;
            making_string = !making_string;
            if making_string == false {
                tokens.push(Token {
                    value: TokenValue::String(
                        AtomStorage::atom(string.clone().replace("\\\"", "\""))),
                    span: Span {
                        file_name: file_name_atom,
                        start: string_start_location.unwrap(),
                        end: end_location,
                    }
                });
                string = String::from("");
            }
            continue;
        }
        prev_char = char;

        if making_string {
            string_start_location = Some(
                Location::only(line, column - 1)
            );
            string.push(char);
            continue;
        }

        if is_skippable(char) {
            tokens.push(Token {
                value: TokenValue::Skip,
                span: Span {
                    file_name: file_name_atom,
                    start: start_location,
                    end: end_location,
                }
            });
            continue;
        }

        let single_char_token_value = match char {
            '(' => Some(TokenValue::Sign(SignType::Paren(Direction::Open))),
            ')' => Some(TokenValue::Sign(SignType::Paren(Direction::Close))),
            '[' => Some(TokenValue::Sign(SignType::Brace(Direction::Open))),
            ']' => Some(TokenValue::Sign(SignType::Brace(Direction::Close))),
            '{' => Some(TokenValue::Sign(SignType::CurlyBrace(Direction::Open))),
            '}' => Some(TokenValue::Sign(SignType::CurlyBrace(Direction::Close))),
            _ => None,
        };

        if let Some(value) = single_char_token_value {
            tokens.push(Token {
                value,
                span: Span {
                    file_name: file_name_atom,
                    start: start_location,
                    end: end_location,
                }
            });
            continue;
        }

        let current_token_value = if let Some(sign_type) = SIMPLE_SIGNS.get(&char).cloned() {
            Some(TokenValue::Sign(sign_type))
        } else if let Some(op) = SIMPLE_OPERATORS
            .get(format!("{}", char).as_str())
            .cloned()
        {
            Some(TokenValue::Operator(op))
        } else {
            None
        };

        if let Some(current_value) = current_token_value {
            let mut found_conversion = false;

            if let Some(last_token) = tokens.last_mut() {
                for conversion in SIGN_CONVERSIONS.iter() {
                    if conversion.first == last_token.value
                        && conversion.second == current_value
                    {
                        found_conversion = true;

                        let new_span = Span {
                            file_name: file_name_atom,
                            start: last_token.span.start,
                            end: end_location,
                        };

                        last_token.value = conversion.result.clone();
                        last_token.span = new_span;
                        break;
                    }
                }
            }

            if !found_conversion {
                tokens.push(Token {
                    value: current_value,
                    span: Span {
                        file_name: file_name_atom,
                        start: start_location,
                        end: end_location,
                    }
                });
            }
            continue;
        }

        if char.is_alphabetic() && !char.is_numeric() || char == '_' {
            let mut identifier_string = String::from(char);
            let mut current_end_location = end_location;

            while !input_chars.is_empty() {
                let next_char = input_chars[0];

                if next_char.is_alphanumeric() || next_char == '_' {
                    let consumed_char = input_chars.pop_front().unwrap();
                    advance_cursor(&mut line, &mut column, consumed_char);
                    identifier_string.push(consumed_char);
                    current_end_location = Location::only(line, column);
                } else {
                    break;
                }
            }

            tokens.push(Token {
                value: resolve_string_to_token(identifier_string),
                span: Span {
                    file_name: file_name_atom,
                    start: start_location,
                    end: current_end_location,
                }
            });
        }

        else if char.is_numeric() {
            let mut number_str = String::from(char);
            let mut current_end_location = end_location;

            while !input_chars.is_empty() {
                let next_char = input_chars[0];

                if next_char == '.' && input_chars.get(1).map_or(false, |c| *c == '.') {
                    break;
                }

                if next_char.is_numeric() || (next_char == '.' && !number_str.contains('.')) {
                    let consumed_char = input_chars.pop_front().unwrap();
                    advance_cursor(&mut line, &mut column, consumed_char);
                    number_str.push(consumed_char);
                    current_end_location = Location::only(line, column);
                } else if is_skippable(next_char) {
                    break;
                } else {
                    break;
                }
            }

            if !number_str.contains('.') {
                number_str.push_str(".0");
            }

            tokens.push(Token {
                value: TokenValue::Number(number_str.parse::<f64>().unwrap()),
                span: Span {
                    file_name: file_name_atom,
                    start: start_location,
                    end: current_end_location,
                }
            });
        }
    }

    tokens = tokens
        .into_iter()
        .filter(|x| x.value != TokenValue::Skip)
        .collect();

    let eof_location = Location::only(line, column);
    tokens.push(Token {
        value: TokenValue::End,
        span: Span {
            file_name: file_name_atom,
            start: eof_location,
            end: eof_location,
        }
    });

    tokens.into()
}