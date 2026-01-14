use crate::interpret::structs::TypeData;
use crate::lexer::structs::{OperatorType, Span};
use crate::log::{Control, Log, LogOrigin};
use crate::parser::structs::{ASTNode, ASTNodeValue};
use crate::store::Atom;
use crate::typed::{DataType, NumTypes, TypeSig};
use crate::util::{Arw, Unbox, arw};
use std::any::Any;
use std::collections::HashMap;
use uuid::Uuid;

pub struct SALocalData {
    ty: DataType,
    immut: bool,
}

pub struct SAScope {
    locals: HashMap<Atom, Arw<SALocalData>>,
}

impl SAScope {
    pub fn new() -> SAScope {
        SAScope {
            locals: HashMap::new(),
        }
    }
}

pub struct StaticAnalysis {
    scopes: Vec<Arw<SAScope>>,
    problems: Vec<Problem>,
    node_types: HashMap<Uuid, DataType>,
}

pub enum ProblemType {
    Err,
}

pub struct Problem {
    pub ty: ProblemType,
    pub sp: Span,
    pub mg: String,
}

impl StaticAnalysis {
    pub fn new() -> StaticAnalysis {
        StaticAnalysis {
            problems: Vec::new(),
            scopes: vec![arw(SAScope::new())],
            node_types: HashMap::new(),
        }
    }

    pub fn analyze(&mut self, nodes: Vec<ASTNode>) -> bool {
        for node in nodes {
            self.type_of(node);
        }

        if self.problems.is_empty() {
            return true;
        }

        for p in &self.problems {
            match p.ty {
                ProblemType::Err => Log::err(p.mg.to_string(), LogOrigin::StaticAnalysis),
            }
            Log::trace_span(p.sp);
        }

        if !self.problems.is_empty() {
            Control::exit();
        }

        false
    }

    pub fn type_of(&mut self, node: ASTNode) -> DataType {
        if self.node_types.contains_key(&node.id) {
            return self.node_types[&node.id].clone();
        }

        let t = match node.value {
            ASTNodeValue::BinaryExpression { left, right, op } => {
                let left_type = self.type_of(left.unbox());
                let right_type = self.type_of(right.unbox());

                match op {
                    OperatorType::Plus => {
                        if left_type.matches(&DataType::Num(NumTypes::Int))
                            && right_type.matches(&DataType::Num(NumTypes::Int))
                        {
                            return DataType::Num(NumTypes::Int);
                        }
                        if left_type.matches(&DataType::Num(NumTypes::Flt))
                            && right_type.matches(&DataType::Num(NumTypes::Flt))
                        {
                            return DataType::Num(NumTypes::Flt);
                        }
                        if left_type.matches(&DataType::Num(NumTypes::Gen))
                            && right_type.matches(&DataType::Num(NumTypes::Gen))
                        {
                            return DataType::Num(NumTypes::Gen);
                        }

                        DataType::Null
                    }
                    OperatorType::Minus => {
                        if left_type.matches(&DataType::Num(NumTypes::Int))
                            && right_type.matches(&DataType::Num(NumTypes::Int))
                        {
                            return DataType::Num(NumTypes::Int);
                        }
                        if left_type.matches(&DataType::Num(NumTypes::Flt))
                            && right_type.matches(&DataType::Num(NumTypes::Flt))
                        {
                            return DataType::Num(NumTypes::Flt);
                        }
                        if left_type.matches(&DataType::Num(NumTypes::Gen))
                            && right_type.matches(&DataType::Num(NumTypes::Gen))
                        {
                            return DataType::Num(NumTypes::Gen);
                        }
                        DataType::Null
                    }
                    OperatorType::Multiply => {
                        if left_type.matches(&DataType::Num(NumTypes::Int))
                            && right_type.matches(&DataType::Num(NumTypes::Int))
                        {
                            return DataType::Num(NumTypes::Int);
                        }
                        if left_type.matches(&DataType::Num(NumTypes::Flt))
                            && right_type.matches(&DataType::Num(NumTypes::Flt))
                        {
                            return DataType::Num(NumTypes::Flt);
                        }
                        if left_type.matches(&DataType::Num(NumTypes::Gen))
                            && right_type.matches(&DataType::Num(NumTypes::Gen))
                        {
                            return DataType::Num(NumTypes::Gen);
                        }
                        if left_type.matches(&DataType::Str) && right_type.matches(&DataType::num())
                        {
                            return DataType::Str;
                        }

                        DataType::Null
                    }
                    OperatorType::Divide => {
                        if left_type.matches(&DataType::Num(NumTypes::Int))
                            && right_type.matches(&DataType::Num(NumTypes::Int))
                        {
                            return DataType::Num(NumTypes::Gen);
                        }
                        if left_type.matches(&DataType::Num(NumTypes::Flt))
                            && right_type.matches(&DataType::Num(NumTypes::Flt))
                        {
                            return DataType::Num(NumTypes::Flt);
                        }
                        if left_type.matches(&DataType::Num(NumTypes::Gen))
                            && right_type.matches(&DataType::Num(NumTypes::Gen))
                        {
                            return DataType::Num(NumTypes::Gen);
                        }

                        DataType::Null
                    }
                    OperatorType::Modulo => {
                        if left_type.matches(&DataType::Num(NumTypes::Int))
                            && right_type.matches(&DataType::Num(NumTypes::Int))
                        {
                            return DataType::Num(NumTypes::Int);
                        }
                        if left_type.matches(&DataType::Num(NumTypes::Flt))
                            && right_type.matches(&DataType::Num(NumTypes::Flt))
                        {
                            return DataType::Num(NumTypes::Flt);
                        }
                        if left_type.matches(&DataType::Num(NumTypes::Gen))
                            && right_type.matches(&DataType::Num(NumTypes::Gen))
                        {
                            return DataType::Num(NumTypes::Gen);
                        }
                        DataType::Null
                    }
                    OperatorType::Equality => DataType::Bln,
                    OperatorType::Inequality => DataType::Bln,
                    _ => DataType::Null,
                }
            }
            ASTNodeValue::Number(v) => {
                if v.floor() == v {
                    DataType::Num(NumTypes::Int)
                } else {
                    DataType::Num(NumTypes::Flt)
                }
            }
            ASTNodeValue::Identifier(v) => match self.get_local(&v, node.span) {
                None => DataType::Null,
                Some(t) => t.r().ty.clone(),
            },
            ASTNodeValue::VariableDeclaration {
                name,
                data_type,
                immut,
                value,
            } => {
                let type_of_value = self.type_of(value.unbox());
                let data = data_type.map(|v| self.type_of(v.unbox()));

                if let Some(v) = &data {
                    if !type_of_value.matches(v) {
                        self.push_problem(
                            ProblemType::Err,
                            node.span,
                            format!("The expected type for variable '{}' '{}' doesn't match the value's type '{}'.",
                                    &name, v, type_of_value),
                        );

                        return DataType::Null;
                    }
                }

                self.scope().w().locals.insert(
                    name,
                    arw(SALocalData {
                        ty: data.unwrap_or(type_of_value),
                        immut,
                    }),
                );

                DataType::Uni
            }
            // ASTNodeValue::Assignment { .. } => {}
            ASTNodeValue::String(_) => DataType::Str,
            ASTNodeValue::Boolean(_) => DataType::Bln,
            // ASTNodeValue::Block { .. } => {}
            // ASTNodeValue::If { .. } => {}
            ASTNodeValue::Type {
                dynamic,
                content,
                generics,
            } => DataType::from_atoms(content),
            ASTNodeValue::If { ifs, or_else } => {
                let ret = self.type_of(ifs.first().unwrap().block.clone().unbox());

                for i in ifs {
                    let co = i.condition.unbox();
                    let bo = i.block.unbox();
                    let c_t = self.type_of(co.clone());
                    let b_t = self.type_of(bo.clone());

                    if c_t != DataType::Bln {
                        self.push_problem(
                            ProblemType::Err,
                            co.span,
                            format!("The condition for the if statement evaluates to '{}' instead of 'Bln'.", &c_t)
                        )
                    }

                    if !b_t.matches(&ret) {
                        self.push_problem(
                            ProblemType::Err,
                            co.span,
                            format!(
                                "The block of if statement to '{}' instead of expected '{}'.",
                                &b_t, &ret
                            ),
                        )
                    }
                }

                if let Some(else_bl) = or_else {
                    let sp = else_bl.span;
                    let ty = self.type_of(else_bl.unbox());

                    if !ty.matches(&ret) {
                        self.push_problem(
                            ProblemType::Err,
                            sp,
                            format!(
                                "The else block of if statement to '{}' instead of expected '{}'.",
                                &ty, &ret
                            ),
                        )
                    }
                }

                ret
            }
            // ASTNodeValue::When { .. } => {}
            // ASTNodeValue::Function { .. } => {}
            // ASTNodeValue::Call { .. } => {}
            // ASTNodeValue::StructDefinition { .. } => {}
            // ASTNodeValue::StructCreation { .. } => {}
            // ASTNodeValue::PropertyAccess { .. } => {}
            // ASTNodeValue::ArrayDeclaration { .. } => {}
            // ASTNodeValue::ArrayAccess { .. } => {}
            ASTNodeValue::Unit => DataType::Uni,
            ASTNodeValue::Block { contents } => {
                let last = contents.last();

                if let Some(v) = last {
                    self.type_of(v.clone())
                } else {
                    DataType::Uni
                }
            }
            ASTNodeValue::Assignment { .. } => DataType::Uni,
            _ => todo!("{:?}", node.value),
        };
        self.node_types.insert(node.id, t.clone());
        t
    }

    fn push_problem(&mut self, ty: ProblemType, sp: Span, mg: String) {
        self.problems.push(Problem { ty, sp, mg })
    }

    fn push_scope(&mut self) {
        self.scopes.push(arw(SAScope::new()))
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn scope(&mut self) -> Arw<SAScope> {
        self.scopes.last().unwrap().clone()
    }

    fn get_local(&mut self, atom: &Atom, trace: Span) -> Option<Arw<SALocalData>> {
        let mut idx = self.scopes.len() as i32 - 1;

        while idx != -1 {
            if let Some(v) = self.scopes[idx as usize].r().locals.get(atom) {
                return Some(v.clone());
            }
            idx -= 1;
        }

        self.push_problem(
            ProblemType::Err,
            trace,
            format!("Variable '{}' is not defined in this scope.", atom),
        );

        None
    }
}
