use crate::interpret::structs::{
    ArrayData, BinExpLogicals, BinExpRelations, ComplexData, ComplexStruct, FunctionData,
    StructData, TypeData,
};
pub mod structs;

use crate::interpret::structs::{
    AssignmentProperty, BinExpAdd, BinExpDiv, BinExpMul, BinExpRem, BinExpSub, RuntimeValue,
    Variable,
};
use crate::lexer::structs::{OperatorType, Span};
use crate::log::{Control, Log, LogOrigin};
use crate::parser::structs::{ASTNode, ASTNodeValue};
use crate::store::{Atom, AtomStorage};
use crate::typed::{CallMatches, DataTypeSignature, FinalizedDataType, Types, try_match};
use crate::util::{Arw, Rw, Unbox, arw};
use std::collections::{HashMap, VecDeque};
use std::fmt::format;
use std::ops::Deref;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Interpreter {
    pub(crate) global_types: Arc<Types>,
}

#[derive(Debug)]
pub struct RuntimeScope {
    parent: Option<Arw<RuntimeScope>>,
    variables: Rw<HashMap<Atom, Arw<Variable>>>,
    types: Rw<HashMap<Atom, Arc<DataTypeSignature>>>,
    interpreter: Arc<Interpreter>,
}

impl RuntimeScope {
    fn new_internal(parent: Option<Arw<RuntimeScope>>, interpreter: Arc<Interpreter>) -> Self {
        RuntimeScope {
            parent,
            variables: Rw::new(HashMap::new()),
            types: Rw::new(HashMap::new()),
            interpreter: interpreter.clone(),
        }
    }

    pub fn parented(parent: Arw<RuntimeScope>, interpreter: Arc<Interpreter>) -> Self {
        Self::new_internal(Some(parent), interpreter)
    }

    pub fn new(interpreter: Arc<Interpreter>) -> Self {
        Self::new_internal(None, interpreter)
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
                    format!(
                        "Variable '{}' is not defined in this scope.",
                        AtomStorage::string(atom).unwrap()
                    ),
                    LogOrigin::Interpret,
                );
                if let Some(span) = trace_span {
                    Log::trace_span(span);
                }
                Control::exit();
            }
            Some(v) => v,
        }
    }

    pub fn declare_variable(
        &self,
        name: Atom,
        value: RuntimeValue,
        is_immut: bool,
        trace: Option<Span>,
        ty: Option<FinalizedDataType>,
    ) {
        if let Some(_) = self.get_variable_condition(name) {
            Log::err(
                format!(
                    "Variable '{}' cannot be redeclared.",
                    AtomStorage::string(name).unwrap()
                ),
                LogOrigin::Interpret,
            );
            if let Some(span) = trace {
                Log::trace_span(span);
            }
            Control::exit();
        }

        if let Some(t) = ty.clone() {
            if !(t.matches)(t.clone(), &value) {
                Log::err(
                    format!(
                        "Variable '{}' cannot be declared, as the provided value of type '{}' doesn't match the provided type of '{}'.",
                        AtomStorage::string(name).unwrap(),
                        self.try_match(&value).unwrap().vis(),
                        &t.vis()
                    ),
                    LogOrigin::Interpret,
                );
                if let Some(span) = trace {
                    Log::trace_span(span);
                }
                Control::exit();
            }
        }

        let ty = match ty {
            None => self.try_match(&value).unwrap(),
            Some(v) => v,
        };

        self.variables.w().insert(
            name,
            arw(Variable {
                name,
                value: Rw::new(value),
                is_immut,
                ty,
            }),
        );
    }

    pub fn eval_assignment(&self, prop: AssignmentProperty, value: RuntimeValue, trace: Span) {
        match prop {
            AssignmentProperty::VariableOrFunction(v_name) => {
                let v = self.get_variable(v_name, Some(trace));

                if v.r().is_immut {
                    Log::err(
                        format!(
                            "Cannot assign a new value to the immutable '{}'.",
                            AtomStorage::string(v_name).unwrap()
                        ),
                        LogOrigin::Interpret,
                    );
                    Log::trace_span(trace);
                    Control::exit();
                }

                if !v.r().ty.call_matches(&value) {
                    Log::err(
                        format!(
                            "Cannot assign a new value to the variable '{}' as the value of type '{}' doesn't match variable's type of '{}'.",
                            AtomStorage::string(v_name).unwrap(),
                            self.try_match(&value).unwrap().visual_name,
                            &v.r().ty.vis()
                        ),
                        LogOrigin::Interpret,
                    );
                    Log::trace_span(trace);
                    Control::exit();
                }

                *v.w().value.w() = value;
            }
        }
    }

    pub fn try_match_complex(&self, value: &ComplexData) -> Option<FinalizedDataType> {
        match value {
            ComplexData::Struct(v) => Some(self.find_dynamic_type(v.name, None)),
        }
    }

    pub fn try_match(&self, value: &RuntimeValue) -> Option<FinalizedDataType> {
        if let RuntimeValue::Complex(c) = value {
            return self.try_match_complex(c);
        }

        if let Some(sig) = try_match(&self.types.r(), value) {
            return Some(sig);
        }

        if let Some(parent_scope) = &self.parent {
            if let Some(sig) = parent_scope.r().try_match(value) {
                return Some(sig);
            }
        }

        try_match(&self.interpreter.global_types.types.r(), value)
    }

    fn add_type(&mut self, types: (Atom, Arc<DataTypeSignature>)) {
        self.types.w().insert(types.0, types.1);
    }

    pub fn find_dynamic_type(&self, type_name: Atom, trace: Option<Span>) -> FinalizedDataType {
        let variable = self.get_variable(type_name, trace);

        let v_r = variable.r();
        let v = v_r.value.r();

        let val = v.as_type().unwrap().clone();

        let uuid: Uuid;

        let matches: Arc<dyn Fn(FinalizedDataType, &RuntimeValue) -> bool> = Arc::new(match val {
            TypeData::Struct(stct) => {
                uuid = stct.uuid;
                move |t, v| {
                    if let RuntimeValue::Complex(ComplexData::Struct(str)) = v {
                        str.prop_types == stct.prop_types && str.prop_names == stct.prop_names
                    } else {
                        false
                    }
                }
            }
        });

        FinalizedDataType {
            name: AtomStorage::string(type_name).unwrap().clone(),
            visual_name: format!("?{}", AtomStorage::string(type_name).unwrap().clone()),
            generics: Vec::new(),
            matches,
            uuid,
        }
    }

    pub(crate) fn find_type(
        &self,
        mut target_type: VecDeque<Atom>,
        generics: Vec<ASTNode>,
        dynamic: bool,
        trace: Option<Span>,
        name: Option<String>,
    ) -> FinalizedDataType {
        let initial = *target_type.front().unwrap();

        if dynamic {
            return self.find_dynamic_type(initial, trace);
        }

        if self.interpreter.global_types.has_type(initial) {
            return self
                .interpreter
                .global_types
                .find_type(target_type, generics, trace, &self);
        }

        let name = if let Some(n) = name {
            n
        } else {
            Vec::from(target_type.clone())
                .iter()
                .map(|x| AtomStorage::string(*x).unwrap().as_str())
                .collect::<Vec<&str>>()
                .join(".")
        };

        if self.types.r().contains_key(&initial) {
            target_type.pop_front();
            let tp = self.types.r();
            let mut kv = tp.get(&initial).unwrap();

            while !target_type.is_empty() {
                let next = target_type.pop_front().unwrap();
                kv = match kv.children.get(&next) {
                    None => {
                        Log::err(
                            format!("Type '{}' couldn't be found.", &name),
                            LogOrigin::Interpret,
                        );
                        if let Some(tr) = trace {
                            Log::trace_span(tr);
                        }
                        Control::exit();
                    }
                    Some(v) => v,
                }
            }

            let t = kv.clone();

            FinalizedDataType::from(t).apply(
                generics
                    .iter()
                    .map(|x| {
                        let (dy, tp, tg) = x.value.clone().into_type().unwrap();

                        self.find_type(tp.into(), tg, dy, Some(x.span), None)
                    })
                    .collect(),
            )
        } else {
            if let Some(par) = &self.parent {
                par.r()
                    .find_type(target_type, generics, dynamic, trace, Some(name))
            } else {
                Log::err(
                    format!("Type '{}' couldn't be found.", &name),
                    LogOrigin::Interpret,
                );
                if let Some(tr) = trace {
                    Log::trace_span(tr);
                }
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
            ASTNodeValue::String(v) => {
                RuntimeValue::String(AtomStorage::string(v).unwrap().clone())
            }
            ASTNodeValue::Boolean(v) => RuntimeValue::Boolean(v),
            ASTNodeValue::Block { contents } => self.eval_block(
                contents,
                arw(RuntimeScope::parented(scope, Arc::new(self.clone()))),
            ),
            ASTNodeValue::If { .. } => self.eval_if(node, scope),
            ASTNodeValue::Type { .. } => {
                unimplemented!("This kind of ASTNodes should be unevaluatable.")
            }
            ASTNodeValue::When { .. } => self.eval_when(node, scope),
            ASTNodeValue::Function { .. } => self.eval_function(node, scope),
            ASTNodeValue::Call { .. } => self.eval_call(node, scope),
            ASTNodeValue::StructDefinition { .. } => self.eval_struct_def(node, scope),
            ASTNodeValue::StructCreation { .. } => self.eval_struct_creation(node, scope),
            ASTNodeValue::PropertyAccess { .. } => self.eval_struct_property(node, scope),
            ASTNodeValue::ArrayDeclaration { .. } => self.eval_array_decl(node, scope),
            ASTNodeValue::ArrayAccess { .. } => self.eval_array_access(node, scope),
        }
    }

    fn eval_variable_declaration(&self, node: ASTNode, scope: Arw<RuntimeScope>) -> RuntimeValue {
        let (name, value, is_immut, ty) = node.value.into_variable_declaration().unwrap();

        let ev = self.eval_node((*value).clone(), scope.clone());

        let ty_res = match ty {
            None => None,
            Some(v) => {
                let (dy, tn, tg) = v.value.into_type().unwrap();
                Some(scope.r().find_type(tn.into(), tg, dy, Some(v.span), None))
            }
        };

        scope
            .w()
            .declare_variable(name, ev, is_immut, Some(node.span), ty_res);

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
            _ => unreachable!(),
        }
    }

    fn eval_identifier(&self, node: ASTNode, scope: Arw<RuntimeScope>) -> RuntimeValue {
        let var = node.value.into_identifier().unwrap();

        let g = scope.r().get_variable(var, Some(node.span));

        g.r().value.r().deref().clone()
    }

    fn figure_out_assignment_property(prop: ASTNode) -> AssignmentProperty {
        if prop.value.is_identifier() {
            AssignmentProperty::VariableOrFunction(prop.value.into_identifier().unwrap())
        } else {
            Log::err(
                format!("Cannot create an assignment property from {:?}.", &prop),
                LogOrigin::Interpret,
            );
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

        scope.w().eval_assignment(prop, ev_val, trace);

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
        let (ifs, or_else) = node.value.into_when().unwrap();

        // let ev_v = self.eval_node(value.unbox(), scope.clone());

        let mut ret = RuntimeValue::Unit;

        let mut should_run_else = true;

        for stmt in ifs {
            let sp = stmt.condition.span;
            let ev_condition = self.eval_node(stmt.condition.unbox(), scope.clone());

            if matches!(
                ev_condition,
                RuntimeValue::Boolean(true)
            ) {
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

    fn find_ty(&self, node: ASTNode, scope: Arw<RuntimeScope>) -> FinalizedDataType {
        let (dy, ty, tg) = node.value.into_type().unwrap();
        scope
            .r()
            .find_type(ty.into(), tg, dy, Some(node.span), None)
    }

    fn eval_function(&self, node: ASTNode, scope: Arw<RuntimeScope>) -> RuntimeValue {
        let (arg_names, arg_types, ret_type, body) = node.value.into_function().unwrap();

        let types: Vec<FinalizedDataType> = arg_types
            .iter()
            .map(|x| {
                let (dy, ty, tg) = x.value.clone().into_type().unwrap();
                scope.r().find_type(ty.into(), tg, dy, Some(x.span), None)
            })
            .collect();
        let ret_type = {
            let (dy, ty, tg) = ret_type.value.into_type().unwrap();
            scope
                .r()
                .find_type(ty.into(), tg, dy, Some(ret_type.span), None)
        };

        let func_scope = RuntimeScope::parented(scope.clone(), Arc::new(self.clone()));

        RuntimeValue::Function(FunctionData {
            arg_names,
            arg_types: types,
            ret_type,
            function_body: body,
            scope: arw(func_scope),
        })
    }

    fn eval_call(&self, node: ASTNode, scope: Arw<RuntimeScope>) -> RuntimeValue {
        let (on, args) = node.value.into_call().unwrap();

        let o_span = on.span;
        let fun = self.eval_node(on.unbox(), scope.clone());

        match fun {
            RuntimeValue::Function(fd) => {
                let fn_scope = arw(RuntimeScope::parented(
                    fd.scope.clone(),
                    Arc::new(self.clone()),
                ));

                if args.len() != fd.arg_names.len() {
                    Log::err(
                        format!(
                            "Expected {} arguments, found {}.",
                            fd.arg_names.len(),
                            args.len()
                        ),
                        LogOrigin::Interpret,
                    );
                    Log::trace_span(node.span);
                    Control::exit();
                }

                let args_ev: Vec<RuntimeValue> = args
                    .into_iter()
                    .map(|x| self.eval_node(x, scope.clone()))
                    .collect();

                for i in 0..(fd.arg_types.len()) {
                    let ty = &fd.arg_types[i];
                    let nm = &fd.arg_names[i];

                    if !ty.call_matches(&args_ev[i]) {
                        let t1 = scope.r().try_match(&args_ev[i]).unwrap();
                        let t2 = &fd.arg_types[i];
                        Log::err(
                            format!(
                                "The type of the provided argument [{}] {} does not match the expected type {} {}.",
                                i,
                                t1.vis(),
                                t2.vis(),
                                if t1.vis() == t2.vis() {
                                    format!("(UUIDs {} and {} respectfully)", t1.uuid, t2.uuid)
                                } else {
                                    "".to_string()
                                }
                            ),
                            LogOrigin::Interpret,
                        );
                        Log::trace_span(node.span);
                        Control::exit();
                    }

                    fn_scope.w().declare_variable(
                        *nm,
                        args_ev[i].clone(),
                        true,
                        None,
                        Some(fd.arg_types[i].clone()),
                    );
                }

                let e = self.eval_node(fd.function_body.unbox(), fn_scope);

                if !fd.ret_type.call_matches(&e) {
                    Log::err(
                        format!(
                            "The returned value of type {} does not match the expected type {}.",
                            scope.r().try_match(&e).unwrap().visual_name,
                            fd.ret_type.vis()
                        ),
                        LogOrigin::Interpret,
                    );
                    Log::trace_span(node.span);
                    Control::exit();
                }

                e
            }
            _ => {
                Log::err("Cannot get a callable.".to_string(), LogOrigin::Interpret);
                Log::trace_span(o_span);
                Control::exit();
            }
        }
    }

    fn eval_struct_def(&self, node: ASTNode, scope: Arw<RuntimeScope>) -> RuntimeValue {
        let (names, types) = node.value.into_struct_definition().unwrap();

        RuntimeValue::Type(TypeData::Struct(StructData {
            prop_names: names,
            prop_types: self.map_types(types, scope),
            uuid: Uuid::new_v4(),
        }))
    }

    fn map_types(&self, types: Vec<ASTNode>, scope: Arw<RuntimeScope>) -> Vec<FinalizedDataType> {
        types
            .iter()
            .map(|x| {
                let (dy, ty, tg) = x.value.clone().into_type().unwrap();
                scope.r().find_type(ty.into(), tg, dy, Some(x.span), None)
            })
            .collect()
    }

    fn eval_struct_creation(&self, node: ASTNode, scope: Arw<RuntimeScope>) -> RuntimeValue {
        let (name, props) = node.value.into_struct_creation().unwrap();

        let ty = scope.r().find_dynamic_type(name, Some(node.span));

        let type_var = scope.r().get_variable(name, Some(node.span));

        let type_data = match type_var.r().value.r().clone().into_type() {
            Ok(v) => match v {
                TypeData::Struct(v) => v,
            },
            Err(_) => {
                Log::err(
                    format!(
                        "The variable {} is not Typ.",
                        AtomStorage::string(name).unwrap()
                    ),
                    LogOrigin::Interpret,
                );
                Log::trace_span(node.span);
                Control::exit();
            }
        };

        if props.len() != type_data.prop_names.len() {
            Log::err("Not all keys provided.".to_string(), LogOrigin::Interpret);
            Log::trace_span(node.span);
            Control::exit();
        }

        let props_ev: HashMap<Atom, RuntimeValue> = props
            .into_iter()
            .map(|(k, v)| (k, self.eval_node(v, scope.clone())))
            .collect();
        let map_kvs: HashMap<Atom, FinalizedDataType> = {
            let mut m = HashMap::new();

            for i in 0..type_data.prop_types.len() {
                m.insert(type_data.prop_names[i], type_data.prop_types[i].clone());
            }

            m
        };

        for (k, v) in &props_ev {
            let ty = map_kvs.get(k).unwrap();

            if !ty.call_matches(v) {
                Log::err(
                    format!(
                        "Provided value for key {} does not match its type of {}.",
                        AtomStorage::string(*k).unwrap(),
                        ty.vis()
                    ),
                    LogOrigin::Interpret,
                );
                Log::trace_span(node.span);
                Control::exit();
            }
        }

        RuntimeValue::Complex(ComplexData::Struct(ComplexStruct {
            name,
            prop_names: type_data.prop_names,
            prop_types: type_data.prop_types,
            data: props_ev,
        }))
    }

    fn eval_struct_property(&self, node: ASTNode, scope: Arw<RuntimeScope>) -> RuntimeValue {
        let (on, name) = node.value.into_property_access().unwrap();

        let o_sp = on.span;
        let o_ev = self.eval_node(on.unbox(), scope);

        match o_ev {
            RuntimeValue::Complex(complex) => match complex {
                ComplexData::Struct(str) => match str.data.get(&name) {
                    None => {
                        Log::err(
                            format!(
                                "Property '{}' is not defined for type {}.",
                                AtomStorage::string(name).unwrap(),
                                AtomStorage::string(str.name).unwrap()
                            ),
                            LogOrigin::Interpret,
                        );
                        Log::trace_span(o_sp);
                        Control::exit();
                    }
                    Some(v) => v.clone(),
                },
            },
            _ => {
                Log::err("Not a complex.".to_string(), LogOrigin::Interpret);
                Log::trace_span(o_sp);
                Control::exit();
            }
        }
    }

    fn eval_array_decl(&self, node: ASTNode, scope: Arw<RuntimeScope>) -> RuntimeValue {
        let (values, ty) = node.value.into_array_declaration().unwrap();

        let ev: Vec<RuntimeValue> = values
            .into_iter()
            .map(|x| self.eval_node(x, scope.clone()))
            .collect();

        let f_ty = match ty {
            None => {
                if ev.is_empty() {
                    Log::err("Cannot determine a type of the empty array. Try specifying the type or fill it.".to_string(), LogOrigin::Interpret);
                    Log::trace_span(node.span);
                    Control::exit();
                } else {
                    scope.r().try_match(&ev[0]).unwrap()
                }
            }
            Some(v) => self.find_ty(v.unbox(), scope),
        };

        for (i, v) in ev.iter().enumerate() {
            if !f_ty.call_matches(v) {
                Log::err(
                    format!(
                        "Array index [{}] does not match array's type {}.",
                        i,
                        f_ty.vis()
                    ),
                    LogOrigin::Interpret,
                );
                Log::trace_span(node.span);
                Control::exit();
            }
        }

        RuntimeValue::Array(ArrayData {
            ty: f_ty,
            values: ev,
        })
    }

    fn eval_array_access(&self, node: ASTNode, scope: Arw<RuntimeScope>) -> RuntimeValue {
        let (on, index) = node.value.into_array_access().unwrap();

        let on_ev = self.eval_node(on.unbox(), scope.clone());
        let index = self.eval_node(index.unbox(), scope.clone());

        on_ev.index(index, node.span)
    }

    pub fn scope_ref(&self) -> Arc<Interpreter> {
        Arc::new(self.clone())
    }
}
