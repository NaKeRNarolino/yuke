use crate::interpret::structs::{BinExpLogicals, BinExpRelations};
pub mod structs;

use std::collections::{HashMap, VecDeque};
use std::fmt::format;
use std::ops::Deref;
use std::sync::Arc;
use crate::interpret::structs::{AssignmentProperty, BinExpAdd, BinExpDiv, BinExpMul, BinExpRem, BinExpSub, RuntimeValue, Variable};
use crate::lexer::structs::{OperatorType, Span};
use crate::log::{Control, Log, LogOrigin};
use crate::parser::structs::{ASTNode, ASTNodeValue};
use crate::store::{Atom, AtomStorage};
use crate::typed::{try_match, CallMatches, DataTypeSignature, GlobalTypes, GLOBAL_TYPES};
use crate::util::{arw, Arw, Rw, Unbox};

pub struct Interpreter {
}

pub struct RuntimeScope {
    parent: Option<Arw<RuntimeScope>>,
    variables: Rw<HashMap<Atom, Arw<Variable>>>,
    types: Rw<HashMap<Atom, Arc<DataTypeSignature>>>
}

impl RuntimeScope {
    fn new_internal(parent: Option<Arw<RuntimeScope>>) -> Self {
        RuntimeScope {
            parent,
            variables: Rw::new(HashMap::new()),
            types: Rw::new(HashMap::new())
        }
    }

    pub fn parented(parent: Arw<RuntimeScope>) -> Self {
        Self::new_internal(Some(parent))
    }

    pub fn new() -> Self {
        Self::new_internal(None)
    }

    pub fn get_variable_condition(&self, atom: Atom) -> Option<Arw<Variable>> {
        if let Some(var) = self.variables.r().get(&atom) {
            Some(var.clone())
        } else {
            if let Some(parent) = self.parent.clone() {
                parent.r().get_variable_condition(atom)
            } else {
                None
            }
        }
    }

    pub fn get_variable(&self, atom: Atom, trace_span: Option<Span>) -> Arw<Variable> {
        match self.get_variable_condition(atom) {
            None => {
                Log::err(
                    format!("Variable '{}' is not defined in this scope.", AtomStorage::string(atom).unwrap()),
                    LogOrigin::Interpret
                );
                if let Some(span) = trace_span {
                    Log::trace_span(span);
                }
                Control::exit();
            }
            Some(v) => v
        }
    }

    pub fn declare_variable(&self, name: Atom, value: RuntimeValue, is_immut: bool, trace: Option<Span>, ty: Option<Arc<DataTypeSignature>>) {
        if let Some(_) = self.get_variable_condition(name) {
            Log::err(
                format!("Variable '{}' cannot be redeclared.", AtomStorage::string(name).unwrap()),
                LogOrigin::Interpret
            );
            if let Some(span) = trace {
                Log::trace_span(span);
            }
            Control::exit();
        }

        if let Some(t) = ty.clone() {
            if !(t.matches)(t.clone(), &value) {
                Log::err(
                    format!("Variable '{}' cannot be declared, as the provided value of type '{}' doesn't match the provided type of '{}'.", AtomStorage::string(name).unwrap(), self.try_match(&value).unwrap().visual_name, &t.visual_name),
                    LogOrigin::Interpret
                );
                if let Some(span) = trace {
                    Log::trace_span(span);
                }
                Control::exit();
            }
        }

        let ty = match ty {
            None => self.try_match(&value).unwrap(),
            Some(v) => v
        };

        self.variables.w().insert(name, arw(Variable {
            name, value: Rw::new(value), is_immut, ty
        }));
    }

    pub fn eval_assignment(&self, prop: AssignmentProperty, value: RuntimeValue, trace: Span) {
        match prop {
            AssignmentProperty::VariableOrFunction(v_name) => {
                let v = self.get_variable(v_name, Some(trace));

                if v.r().is_immut {
                    Log::err(format!("Cannot assign a new value to the immutable '{}'.", AtomStorage::string(v_name).unwrap()), LogOrigin::Interpret);
                    Log::trace_span(trace);
                    Control::exit();
                }

                if !v.r().ty.call_matches(&value) {
                    Log::err(format!("Cannot assign a new value to the variable '{}' as the value of type '{}' doesn't match variable's type of '{}'.", AtomStorage::string(v_name).unwrap(), self.try_match(&value).unwrap().visual_name, &v.r().ty.visual_name), LogOrigin::Interpret);
                    Log::trace_span(trace);
                    Control::exit();
                }

                *v.w().value.w() = value;
            }
        }
    }

    pub fn try_match(&self, value: &RuntimeValue) -> Option<Arc<DataTypeSignature>> {
        if let Some(sig) = try_match(&self.types.r(), value) {
            return Some(sig);
        }

        if let Some(parent_scope) = &self.parent {
            if let Some(sig) = parent_scope.r().try_match(value) {
                return Some(sig);
            }
        }

        try_match(&GLOBAL_TYPES.r(), value)
    }

    fn add_type(&mut self, types: (Atom, Arc<DataTypeSignature>)) {
        self.types.w().insert(types.0, types.1);
    }

    fn find_type(&self, mut target_type: VecDeque<Atom>, trace: Option<Span>, name: Option<String>) -> Arc<DataTypeSignature> {
        let initial = *target_type.front().unwrap();

        if GlobalTypes::has_type(initial) {
            return GlobalTypes::find_type(target_type, trace);
        }

        let name = if let Some(n) = name { n } else { Vec::from(target_type.clone()).iter().map(|x| AtomStorage::string(*x).unwrap().as_str()).collect::<Vec<&str>>().join(".") };

        if self.types.r().contains_key(&initial) {
            target_type.pop_front();
            let tp = self.types.r();
            let mut kv = tp.get(&initial).unwrap();

            while !target_type.is_empty() {
                let next = target_type.pop_front().unwrap();
                kv = match kv.children.get(&next) {
                    None => {
                        Log::err(format!("Type '{}' couldn't be found.", &name), LogOrigin::Interpret);
                        if let Some(tr) = trace { Log::trace_span(tr); }
                        Control::exit();
                    }
                    Some(v) => v
                }
            }

            kv.clone()
        } else {
            if let Some(par) = &self.parent {
                par.r().find_type(target_type, trace, Some(name))
            } else {
                Log::err(format!("Type '{}' couldn't be found.", &name), LogOrigin::Interpret);
                if let Some(tr) = trace { Log::trace_span(tr); }
                Control::exit();
            }
        }
    }
}

impl Interpreter {
    fn eval_block(&self, ast: Vec<ASTNode>, scope: Arw<RuntimeScope>) -> RuntimeValue {
        let mut res = RuntimeValue::Unit;

        for node in ast {
            res = self.eval_node(node, scope.clone());
        }

        res
    }

    pub fn eval_node(&self, node: ASTNode, scope: Arw<RuntimeScope>) -> RuntimeValue {
        match node.value {
            ASTNodeValue::BinaryExpression { .. } => self.eval_binary_expr(node, scope),
            ASTNodeValue::Number(v) => RuntimeValue::Number(v),
            ASTNodeValue::Unit => RuntimeValue::Unit,
            ASTNodeValue::VariableDeclaration { .. } => self.eval_variable_declaration(node, scope),
            ASTNodeValue::Identifier(_) => self.eval_identifier(node, scope),
            ASTNodeValue::Assignment { .. } => self.eval_assignment(node, scope),
            ASTNodeValue::String(v) => RuntimeValue::String(
                AtomStorage::string(v).unwrap().clone()
            ),
            ASTNodeValue::Boolean(v) => RuntimeValue::Boolean(v),
            ASTNodeValue::Block { contents } => self.eval_block(contents, arw(RuntimeScope::parented(scope))),
            ASTNodeValue::If { .. } => self.eval_if(node, scope),
            ASTNodeValue::Type { .. } => unimplemented!("This kind of ASTNodes should be unevaluatable."),
            ASTNodeValue::When { .. } => self.eval_when(node, scope)
        }
    }

    fn eval_variable_declaration(&self, node: ASTNode, scope: Arw<RuntimeScope>) -> RuntimeValue {
        let (name, value, is_immut, ty) = node.value.into_variable_declaration().unwrap();

        let ev = self.eval_node((*value).clone(), scope.clone());

        let ty_res = match ty {
            None => None,
            Some(v) => Some(scope.r().find_type(
                v.value.into_type().unwrap().into(),
                Some(v.span),
                None
            ))
        };

        scope.w().declare_variable(name, ev, is_immut, Some(node.span), ty_res);

        RuntimeValue::Unit
    }

    fn eval_binary_expr(&self, node: ASTNode, scope: Arw<RuntimeScope>) -> RuntimeValue {
        let (left, right, op) = node.value.into_binary_expression().unwrap();

        let ev_l = self.eval_node((*left).clone(), scope.clone());
        let ev_r = self.eval_node((*right).clone(), scope);

        match op {
            OperatorType::Plus => ev_l.add(ev_r, node.span),
            OperatorType::Minus => ev_l.sub(ev_r, node.span),
            OperatorType::Multiply => ev_l.mul(ev_r, node.span),
            OperatorType::Divide => ev_l.div(ev_r, node.span),
            OperatorType::Modulo => ev_l.rem(ev_r, node.span),
            OperatorType::Equality => ev_l.eq(ev_r, node.span),
            OperatorType::Inequality => ev_l.ieq(ev_r, node.span),
            OperatorType::BiggerEqual => ev_l.beq(ev_r, node.span),
            OperatorType::SmallerEqual => ev_l.seq(ev_r, node.span),
            OperatorType::Bigger => ev_l.big(ev_r, node.span),
            OperatorType::Smaller => ev_l.sml(ev_r, node.span),
            OperatorType::LogicalAnd => ev_l.l_and(ev_r, node.span),
            OperatorType::LogicalOr => ev_l.l_or(ev_r, node.span),
            _ => unreachable!()
        }
    }

    fn eval_identifier(&self, node: ASTNode, scope: Arw<RuntimeScope>) -> RuntimeValue {
        let var = node.value.into_identifier().unwrap();

        let g = scope.r().get_variable(var, Some(node.span));

        g.r().value.r().deref().clone()
    }

    fn figure_out_assignment_property(
        prop: ASTNode
    ) -> AssignmentProperty {
        if prop.value.is_identifier() {
            AssignmentProperty::VariableOrFunction(
                prop.value.into_identifier().unwrap()
            )
        } else {
            Log::err(format!("Cannot create an assignment property from {:?}.", &prop), LogOrigin::Interpret);
            Log::trace_span(prop.span);
            Control::exit();
        }
    }

    fn eval_assignment(&self, node: ASTNode, scope: Arw<RuntimeScope>) -> RuntimeValue {
        let (prop, value) = node.value.into_assignment().unwrap();

        // let ev_prop = self.eval_node(prop.unbox(), scope.clone());
        let trace = node.span;

        let prop = Self::figure_out_assignment_property(prop.unbox());
        let ev_val = self.eval_node(value.unbox(), scope.clone());

        scope.w().eval_assignment(
            prop, ev_val, trace
        );

        RuntimeValue::Unit
    }

    fn eval_if(&self, node: ASTNode, scope: Arw<RuntimeScope>) -> RuntimeValue {
        let mut ret = RuntimeValue::Unit;

        let (ifs, or_else) = node.value.into_if().unwrap();

        let mut should_run_else = true;

        for stmt in ifs {
            let ev_condition = self.eval_node(stmt.condition.unbox(), scope.clone());

            if matches!(ev_condition, RuntimeValue::Boolean(true)) {
                should_run_else = false;
                ret = self.eval_node(stmt.block.unbox(), scope.clone());
                break;
            }
        }

        if should_run_else {
            if let Some(r) = or_else {
                ret = self.eval_node(r.unbox(), scope)
            }
        }

        ret
    }

    fn eval_when(&self, node: ASTNode, scope: Arw<RuntimeScope>) -> RuntimeValue {
        let (value, ifs, or_else) = node.value.into_when().unwrap();

        let ev_v = self.eval_node(value.unbox(), scope.clone());

        let mut ret = RuntimeValue::Unit;

        let mut should_run_else = true;

        for stmt in ifs {
            let sp = stmt.condition.span;
            let ev_condition = self.eval_node(stmt.condition.unbox(), scope.clone());

            if matches!(ev_condition.eq(ev_v.clone(), sp), RuntimeValue::Boolean(true)) {
                should_run_else = false;
                ret = self.eval_node(stmt.block.unbox(), scope.clone());
                break;
            }
        }

        if should_run_else {
            if let Some(r) = or_else {
                ret = self.eval_node(r.unbox(), scope)
            }
        }

        ret
    }
}

