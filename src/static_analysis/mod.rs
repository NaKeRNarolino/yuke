use crate::interpret::structs::TypeData;
use crate::lexer::structs::{OperatorType, Span};
use crate::log::{Control, Log, LogOrigin};
use crate::parser::structs::{ASTNode, ASTNodeValue};
use crate::store::Atom;
use crate::typed::{DataType, DynamicType, NumTypes, TypeSig};
use crate::util::{Arw, Unbox, arw};
use std::any::Any;
use std::collections::HashMap;
use uuid::Uuid;

pub struct SALocalData {
    ty: DataType,
    struct_def: Option<HashMap<Atom, DataType>>,
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
            ASTNodeValue::Identifier(v) => match self.get_local(&v, node.span, true) {
                None => DataType::Null,
                Some(t) => t.r().ty.clone(),
            },
            ASTNodeValue::VariableDeclaration {
                name,
                data_type,
                immut,
                value,
            } => {
                let clone_value = value.clone();
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

                        self.scope().w().locals.insert(
                            name,
                            arw(SALocalData {
                                ty: DataType::Null,
                                struct_def: None,
                                immut,
                            }),
                        );

                        return DataType::Null;
                    }
                }

                self.scope().w().locals.insert(
                    name,
                    arw(SALocalData {
                        ty: data.unwrap_or(type_of_value),
                        struct_def: if clone_value.value.is_struct_definition() {
                            let (names, data_types) =
                                clone_value.value.as_struct_definition().unwrap();

                            let it = names.iter().zip(data_types.iter());

                            Some(
                                HashMap::from_iter(
                                    it.map(|(k, v)| (*k, self.type_of(v.clone())))
                                )
                            )
                        } else {
                            None
                        },
                        immut,
                    }),
                );

                DataType::Uni
            }
            ASTNodeValue::Assignment { prop, value } => {
                let vty = self.type_of(value.unbox());
                let tty = self.type_of(prop.clone().unbox());
                if !vty.matches(&tty) {
                    self.push_problem(
                        ProblemType::Err,
                        node.span,
                        format!("Type '{}' of value doesn't match type '{}'.", &vty,
                            &tty
                        )
                    )
                }
                match prop.unbox().value {
                    ASTNodeValue::Identifier(atom) => {
                        if let Some(local) = self.get_local(&atom, node.span, false) {
                            if local.r().immut {
                                self.push_problem(
                                    ProblemType::Err,
                                    node.span,
                                    format!("Variable '{}' is immutable.", atom)
                                );
                            }
                        }
                    }
                    _ => {}
                }

                DataType::Uni
            }
            ASTNodeValue::String(_) => DataType::Str,
            ASTNodeValue::Boolean(_) => DataType::Bln,
            ASTNodeValue::Block { contents } => {
                let mut last = DataType::Uni;

                for v in contents {
                    last = self.type_of(v.clone()); // so it actually checks everything for errs
                }

                last
            }
            ASTNodeValue::Type {
                dynamic,
                content,
                generics,
            } => {
                if dynamic {
                    let initial = content[0];

                    let local = self.get_local(&initial, node.span, true).unwrap();

                    let struct_def = local.r().struct_def.clone().unwrap();
                    if local.r().struct_def.is_none() {
                        self.push_problem(
                            ProblemType::Err,
                            node.span,
                            format!("The definition of {} is not a struct.", initial)
                        );
                        return DataType::Null
                    }

                    DataType::Dynamic {
                        name: initial.to_string(),
                        value: DynamicType::Struct(struct_def)
                    }
                } else {
                    DataType::from_atoms(content)
                }
            },
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
                                "The else block of the if statement evaluates to '{}' instead of expected '{}'.",
                                &ty, &ret
                            ),
                        )
                    }
                }

                ret
            }
            ASTNodeValue::When { ifs, or_else } => {
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
                            format!("The condition for the when statement evaluates to '{}' instead of 'Bln'.", &c_t)
                        )
                    }

                    if !b_t.matches(&ret) {
                        self.push_problem(
                            ProblemType::Err,
                            co.span,
                            format!(
                                "The block of the when statement evaluates to '{}' instead of expected '{}'.",
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
            },
            ASTNodeValue::Function { arg_names, arg_types, ret_type, body } => {
                let mut generics: Vec<DataType> = vec![];

                for a_t in arg_types {
                    generics.push(
                        self.type_of(a_t)
                    );
                }

                generics.push(
                    self.type_of(ret_type.unbox())
                );

                let body_last = self.type_of(body.unbox());

                if !body_last.matches(generics.last().unwrap()) {
                    self.push_problem(
                        ProblemType::Err,
                        node.span,
                        format!("Function return type '{}' and what the body returns('{}') don't match.", &generics.last().unwrap(), &body_last)
                    );
                }

                DataType::Fnc(generics)
            }
            ASTNodeValue::Call { on, args } => {
                let fnc_t = self.type_of(on.unbox());

                if !fnc_t.is_fnc() {
                    self.push_problem(
                        ProblemType::Err,
                        node.span,
                        "Not a function.".to_string()
                    );
                }

                let fnc_gen = match fnc_t {
                    DataType::Fnc(g) => g,
                    _ => unreachable!()
                };

                if args.len() != fnc_gen.len() - 1 {
                    self.push_problem(
                        ProblemType::Err,
                        node.span,
                        format!("Expected {} arguments, provided {} arguments.", fnc_gen.len() - 1, args.len())
                    );
                } else {
                    for (i, arg) in args.iter().enumerate() {
                        let arg_t = self.type_of(arg.clone());

                        if !arg_t.matches(&fnc_gen[i]) {
                            self.push_problem(
                                ProblemType::Err,
                                node.span,
                                format!("Expected type '{}' of argument with index {} doesn't match the provided type '{}'.",
                                        &fnc_gen[i], i, &arg_t
                                )
                            );
                        }
                    }
                }

                fnc_gen.last().unwrap().clone()
            }
            ASTNodeValue::StructDefinition { .. } => {
                DataType::Typ
            }
            ASTNodeValue::StructCreation { name, props } => {
                let local = self.get_local(&name, node.span, true).unwrap();

                if local.r().struct_def.is_none() {
                    self.push_problem(
                        ProblemType::Err,
                        node.span,
                        format!("The definition of {} is not a struct.", name)
                    );
                    return DataType::Null
                }

                let struct_def = local.r().struct_def.clone().unwrap();
                for (field_name, node) in props {
                    let ty = self.type_of(node.clone());

                    if !ty.matches(
                        &struct_def[&field_name]
                    ) {
                        self.push_problem(
                            ProblemType::Err,
                            node.span,
                            format!("The field '{}' of struct '{}' doesn't match the expected type (found '{}', expected '{}').",
                                    field_name, name, ty, struct_def[&field_name])
                        );
                        continue
                    }
                }

                return DataType::Dynamic {
                    name: name.to_string(),
                    value: DynamicType::Struct(struct_def)
                }
            },
            ASTNodeValue::ArrayDeclaration { values, ty } => {
                let type_of_first = match values.first() {
                    None => None,
                    Some(v) => Some(self.type_of(v.clone()))
                };
                let mut target_type = match ty {
                    None => None,
                    Some(v) => Some(self.type_of(v.unbox()))
                };

                match (&type_of_first, &target_type) {
                    (Some(t), Some(tt)) => {
                        if !t.matches(&tt) {
                            self.push_problem(
                                ProblemType::Err,
                                node.span,
                                format!("Type '{}' doesn't match '{}'.", &t, &tt)
                            );
                        }
                    }
                    _ => {}
                };

                if target_type.is_none() && type_of_first.is_none() {
                    self.push_problem(
                        ProblemType::Err,
                        node.span,
                        "Can't figure out the type of the array.".to_string()
                    );
                }

                if target_type.is_none() {
                    target_type = type_of_first;
                }

                DataType::Array(Box::new(target_type.unwrap()))
            }
            ASTNodeValue::ArrayAccess { on, index } => {
                let tt = self.type_of(on.unbox());

                if !tt.is_arr() {
                    self.push_problem(
                        ProblemType::Err,
                        node.span,
                        "Not an array.".to_string()
                    );
                }

                let arr_t = match tt { DataType::Array(t) => t.unbox(), _ => unreachable!() };

                arr_t
            }
            ASTNodeValue::Unit => DataType::Uni,
            ASTNodeValue::PropertyAccess { on, property } => {
                let on_type = self.type_of(on.unbox());

                if on_type.is_struct() {
                    match on_type {
                        DataType::Dynamic { value: DynamicType::Struct(v), name } => {
                            match v.get(&property) {
                                None => {
                                    self.push_problem(
                                        ProblemType::Err,
                                        node.span,
                                        format!("The field '{}' doesn't exist on type '{}'.", property, &name)
                                    );
                                    return DataType::Uni
                                }
                                Some(v) => {
                                    v.clone()
                                }
                            }
                        },
                        _ => return DataType::Uni,
                    }
                } else {
                    self.push_problem(
                        ProblemType::Err,
                        node.span,
                        format!("The type '{}' doesn't have fields.", on_type)
                    );
                    return DataType::Uni
                }
            }
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

    fn get_local(&mut self, atom: &Atom, trace: Span, cause_err: bool) -> Option<Arw<SALocalData>> {
        let mut idx = self.scopes.len() as i32 - 1;

        while idx != -1 {
            if let Some(v) = self.scopes[idx as usize].r().locals.get(atom) {
                return Some(v.clone());
            }
            idx -= 1;
        }

        if cause_err {
            self.push_problem(
                ProblemType::Err,
                trace,
                format!("Variable '{}' is not defined in this scope.", atom),
            );
        }

        None
    }
}
