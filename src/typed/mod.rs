use std::collections::{HashMap, VecDeque};
use std::fmt::{Debug, Formatter};
use std::sync::Arc;
use lazy_static::lazy_static;
use crate::interpret::RuntimeScope;
use crate::interpret::structs::RuntimeValue;
use crate::lexer::structs::Span;
use crate::log::{Control, Log, LogOrigin};
use crate::parser::structs::ASTNode;
use crate::store::{Atom, AtomStorage};
use crate::util::Rw;

#[derive(Clone)]
pub struct DataTypeSignature {
    pub name: String,
    pub visual_name: String,
    pub kind: DataTypeKind,
    pub matches: Arc<fn(Arc<DataTypeSignature>, &RuntimeValue) -> bool>,
    pub matches_finalized: Arc<fn(FinalizedDataType, &RuntimeValue) -> bool>,
    pub children: HashMap<Atom, Arc<DataTypeSignature>>
}

#[derive(Clone)]
pub struct FinalizedDataType {
    pub name: String,
    pub visual_name: String,
    pub matches: Arc<fn(FinalizedDataType, &RuntimeValue) -> bool>,
    pub generics: Vec<FinalizedDataType>
}

impl PartialEq for FinalizedDataType {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.generics == other.generics
    }
}

pub trait CallMatches {
    fn call_matches(&self, value: &RuntimeValue) -> bool;
}

impl CallMatches for Arc<DataTypeSignature> {
    fn call_matches(&self, value: &RuntimeValue) -> bool {
        (self.clone().matches)(self.clone(), value)
    }
}

impl CallMatches for FinalizedDataType {
    fn call_matches(&self, value: &RuntimeValue) -> bool {
        (self.matches)(self.clone(), value)
    }
}

impl Debug for DataTypeSignature {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("{ Data Type Signature }")
    }
}

impl Debug for FinalizedDataType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{{ Data Type Signature of {} }}", &self.visual_name))
    }
}

impl From<DataTypeSignature> for FinalizedDataType {
    fn from(value: DataTypeSignature) -> Self {
        Self {
            name: value.name,
            visual_name: value.visual_name,
            matches: value.matches_finalized,
            generics: Vec::new()
        }
    }
}


impl From<Arc<DataTypeSignature>> for FinalizedDataType {
    fn from(value: Arc<DataTypeSignature>) -> Self {
        Self {
            name: value.name.clone(),
            visual_name: value.visual_name.clone(),
            matches: value.matches_finalized.clone(),
            generics: Vec::new()
        }
    }
}

impl FinalizedDataType {
    pub(crate) fn vis(&self) -> String {
        format!(
            "{}{}",
            &self.visual_name,
            if !self.generics.is_empty() {
                format!("<{}>", self.generics.iter().map(|x| x.vis()).collect::<Vec<String>>().join(", "))
            } else { "".to_string() }
        )
    }

    pub fn apply(mut self, generics: Vec<FinalizedDataType>) -> Self {
        self.generics = generics;
        self
    }
}

#[derive(Clone, Debug)]
pub enum DataTypeKind {
    BuiltIn,
    Enum,
    Struct,
}

pub struct GlobalTypes;

impl GlobalTypes {
    pub fn add_type(types: (Atom, Arc<DataTypeSignature>)) {
        GLOBAL_TYPES.w().insert(types.0, types.1);
    }

    pub fn has_type(target_type: Atom) -> bool {
        GLOBAL_TYPES.r().contains_key(&target_type)
    }

    pub fn find_type(mut target_type: VecDeque<Atom>, generics: Vec<ASTNode>, trace: Option<Span>, scope: &RuntimeScope) -> FinalizedDataType {
        let name = Vec::from(target_type.clone()).iter().map(|x| AtomStorage::string(*x).unwrap().as_str()).collect::<Vec<&str>>().join(".");
        let initial = *target_type.front().unwrap();

        if GLOBAL_TYPES.r().contains_key(&initial) {
            target_type.pop_front();
            let tp = GLOBAL_TYPES.r();
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

            let t = kv.clone();

            FinalizedDataType::from(t).apply(
                generics.iter().map(
                    |x| {
                        let (tp, tg) = x.value.clone().into_type().unwrap();

                        scope.find_type(
                            tp.into(),
                            tg,
                            Some(x.span),
                            None
                        )
                    }
                ).collect()
            )
        } else {
            Log::err(format!("Type '{}' couldn't be found.", &name), LogOrigin::Interpret);
            if let Some(tr) = trace { Log::trace_span(tr); }
            Control::exit();
        }
    }
}

pub fn process_special_cases(ty: Arc<DataTypeSignature>, value: &RuntimeValue) -> FinalizedDataType {
    match value {
        RuntimeValue::Function(fd) => {
            let mut v: FinalizedDataType = ty.into();

            let mut generics = fd.arg_types.clone();
            generics.push(fd.ret_type.clone());

            v.generics = generics;

            v
        },
        _ => ty.into()
    }
}

pub fn try_match(set: &HashMap<Atom, Arc<DataTypeSignature>>, value: &RuntimeValue) -> Option<FinalizedDataType> {
    let mut matched: Option<FinalizedDataType> = None;

    for (k, t) in set {
        let matches = (t.matches)(t.clone(), value);

        if matches {
            if !t.children.is_empty() {
                matched = try_match(&t.children, value);
            } else {
                matched = Some(
                    process_special_cases(t.clone(), value)
                );
            }
            break;
        }
    }

    matched
}

lazy_static! {
    pub static ref GLOBAL_TYPES: Rw<HashMap<Atom, Arc<DataTypeSignature>>> = Rw::new(HashMap::new());
}
