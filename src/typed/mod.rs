use std::collections::{HashMap, VecDeque};
use std::fmt::{Debug, Formatter};
use std::sync::Arc;
use lazy_static::lazy_static;
use crate::interpret::structs::RuntimeValue;
use crate::lexer::structs::Span;
use crate::log::{Control, Log, LogOrigin};
use crate::store::{Atom, AtomStorage};
use crate::util::Rw;

#[derive(Clone)]
pub struct DataTypeSignature {
    pub name: String,
    pub visual_name: String,
    pub kind: DataTypeKind,
    pub matches: Arc<fn(Arc<DataTypeSignature>, &RuntimeValue) -> bool>,
    pub children: HashMap<Atom, Arc<DataTypeSignature>>
}

pub trait CallMatches {
    fn call_matches(&self, value: &RuntimeValue) -> bool;
}

impl CallMatches for Arc<DataTypeSignature> {
    fn call_matches(&self, value: &RuntimeValue) -> bool {
        (self.clone().matches)(self.clone(), value)
    }
}

impl Debug for DataTypeSignature {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("{ Data Type Signature }")
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

    pub fn find_type(mut target_type: VecDeque<Atom>, trace: Option<Span>) -> Arc<DataTypeSignature> {
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

            kv.clone()
        } else {
            Log::err(format!("Type '{}' couldn't be found.", &name), LogOrigin::Interpret);
            if let Some(tr) = trace { Log::trace_span(tr); }
            Control::exit();
        }
    }
}

pub fn try_match(set: &HashMap<Atom, Arc<DataTypeSignature>>, value: &RuntimeValue) -> Option<Arc<DataTypeSignature>> {
    let mut matched: Option<Arc<DataTypeSignature>> = None;

    for (k, t) in set {
        let matches = (t.matches)(t.clone(), value);

        if matches {
            if !t.children.is_empty() {
                matched = try_match(&t.children, value);
            } else {
                matched = Some(t.clone());
            }
            break;
        }
    }

    matched
}

lazy_static! {
    pub static ref GLOBAL_TYPES: Rw<HashMap<Atom, Arc<DataTypeSignature>>> = Rw::new(HashMap::new());
}
