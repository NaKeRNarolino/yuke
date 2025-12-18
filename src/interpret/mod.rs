use crate::interpret::structs::{BinExpLogicals, BinExpRelations};
mod structs;

use std::collections::HashMap;
use std::fmt::format;
use std::ops::Deref;
use crate::interpret::structs::{AssignmentProperty, BinExpAdd, BinExpDiv, BinExpMul, BinExpRem, BinExpSub, RuntimeValue, Variable};
use crate::lexer::structs::{OperatorType, Span};
use crate::log::{Control, Log, LogOrigin};
use crate::parser::structs::{ASTNode, ASTNodeValue};
use crate::store::{Atom, AtomStorage};
use crate::util::{arw, Arw, Rw, Unbox};

pub struct Interpreter {
}

pub struct RuntimeScope {
    parent: Option<Arw<RuntimeScope>>,
    variables: Rw<HashMap<Atom, Arw<Variable>>>
}

impl RuntimeScope {
    fn new_internal(parent: Option<Arw<RuntimeScope>>) -> Self {
        RuntimeScope {
            parent,
            variables: Rw::new(HashMap::new())
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

    pub fn declare_variable(&self, name: Atom, value: RuntimeValue, is_immut: bool, trace: Option<Span>) {
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

        self.variables.w().insert(name, arw(Variable {
            name, value: Rw::new(value), is_immut
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

                *v.w().value.w() = value;
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
            ASTNodeValue::If { .. } => self.eval_if(node, scope)
        }
    }

    fn eval_variable_declaration(&self, node: ASTNode, scope: Arw<RuntimeScope>) -> RuntimeValue {
        let (name, value, is_immut) = node.value.into_variable_declaration().unwrap();

        let ev = self.eval_node((*value).clone(), scope.clone());

        scope.w().declare_variable(name, ev, is_immut, Some(node.span));

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
        let trace = prop.span;

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
}

