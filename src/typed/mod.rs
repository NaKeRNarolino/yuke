use crate::interpret::RuntimeScope;
use crate::interpret::structs::RuntimeValueType;
use crate::lexer::structs::Span;
use crate::log::{Control, Log, LogOrigin};
use crate::parser::structs::ASTNode;
use crate::store::{Atom, AtomStorage};
use crate::util::{Rw, Unbox};
use lazy_static::lazy_static;
use std::collections::{HashMap, VecDeque};
use std::fmt::{Debug, Display, Formatter, Write};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::marker::PhantomData;
use std::sync::Arc;
use colored::Colorize;
use uuid::Uuid;
use walrus::Data;
use crate::static_analysis::{StaticAnalysis, UserType};

#[derive(Clone)]
pub struct TypeSignature {
    pub name: Atom,
    pub kind: DataTypeKind,
    pub underlying: Arc<Rw<UnderlyingType>>,
    pub matches: Arc<fn(Arc<TypeSignature>, &RuntimeValueType) -> bool>,
    pub matches_built: Arc<fn(BuiltType, &RuntimeValueType) -> bool>
}

#[derive(Clone, Hash)]
pub struct UnderlyingType {
    pub methods: Vec<i32>, // todo!
    pub structure: TypeStructure
}

impl UnderlyingType {
    pub fn new(st: TypeStructure) -> Self {
        Self {
            methods: Vec::new(),
            structure: st,
        }
    }
}

#[derive(Clone)]
pub enum TypeStructure {
    Struct {
        keys: HashMap<Atom, BuiltType>
    },
    Iota {
        symbols: Vec<Atom>
    },
    Primitive(String)
}

impl Hash for TypeStructure {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            TypeStructure::Struct { keys } => {
                keys.keys().collect::<Vec<&Atom>>().hash(state);
                keys.values().collect::<Vec<&BuiltType>>().hash(state);
            }
            TypeStructure::Iota {
                symbols,
            } => symbols.hash(state),
            TypeStructure::Primitive(name) => {
                name.hash(state)
            }
        }
    }
}

impl Hash for TypeSignature {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.kind.hash(state);
        self.underlying.r().hash(state);
    }
}

#[derive(PartialEq, Clone, Debug)]
pub enum DataType {
    Int,
    Flt,
    Bln,
    Str,
    Uni,
    Null,
    Typ,
    Any,
    Fnc(Vec<DataType>),
    Array(Box<DataType>),
    UserType(Atom, UserType)
}

#[derive(PartialEq, Clone, Debug)]
pub enum DynamicType {
    Struct(HashMap<Atom, DataType>)
}

impl Display for DataType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", &match self {
            DataType::Int => "Int".to_string(),
            DataType::Flt => "Flt".to_string(),
            DataType::Bln => "Bln".to_string(),
            DataType::Str => "Str".to_string(),
            DataType::Uni => "Uni".to_string(),
            DataType::Null => "<Null>".to_string(),
            DataType::Typ { .. } => "Typ".to_string(),
            DataType::Fnc(g) => format!("Fnc<{}>",
                                        g.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(",")),
            DataType::Array(t) => format!("Arr<{}>", t),
            DataType::Any => "Any".to_string(),
            _ => "<*>".to_string()
        }.yellow()))
    }
}

impl DataType {
    pub fn from_atoms(atom: Atom, generics: Vec<ASTNode>, sa: &mut StaticAnalysis) -> Option<DataType> {
        let int_atom = AtomStorage::atom("Int".to_string());
        let flt_atom = AtomStorage::atom("Flt".to_string());
        let str_atom = AtomStorage::atom("Str".to_string());
        let bln_atom = AtomStorage::atom("Bln".to_string());
        let uni_atom = AtomStorage::atom("Uni".to_string());
        let any_atom = AtomStorage::atom("Any".to_string());
        let arr_atom = AtomStorage::atom("Arr".to_string());
        let fnc_atom = AtomStorage::atom("Fnc".to_string());

        if atom == int_atom {
            Some(DataType::Int)
        } else if atom == flt_atom {
            Some(DataType::Flt)
        } else if atom == str_atom {
            Some(DataType::Str)
        } else if atom == bln_atom {
            Some(DataType::Bln)
        } else if atom == uni_atom {
            Some(DataType::Uni)
        } else if atom == any_atom {
            Some(DataType::Any)
        } else if atom == arr_atom {
            assert_eq!(generics.len(), 1);
            let g = sa.type_of(generics[0].clone());

            Some(DataType::Array(Box::new(g)))
        } else {
            None
        }
    }

    pub fn matches(&self, other: &Self) -> bool {
        (self == other) || self.can_be_cast_into(other)
    }

    pub fn can_be_cast_into(&self, other: &Self) -> bool {
        match (self, other) {
            (DataType::Flt, DataType::Int) => true,
            (DataType::Array(t), DataType::Array(tt)) => t.matches(&tt),
            (_, DataType::Any) => true,
            (_, _) => false,
        }
    }

    pub fn is_num(&self) -> bool {
        matches!(self, DataType::Int) && matches!(self, DataType::Flt)
    }

    pub fn is_str(&self) -> bool {
        matches!(self, DataType::Str)
    }

    pub fn is_bln(&self) -> bool {
        matches!(self, DataType::Bln)
    }

    pub fn is_unit(&self) -> bool {
        matches!(self, DataType::Uni)
    }

    pub fn is_fnc(&self) -> bool {
        matches!(self, DataType::Fnc(_))
    }

    pub fn is_struct(&self) -> bool {
        matches!(self, DataType::UserType(_, UserType::Struct(_)))
    }

    pub fn is_arr(&self) -> bool { matches!(self, DataType::Array(_)) }
}


#[derive(Clone)]
pub struct BuiltType {
    pub type_ref: Arc<TypeSignature>,
    pub matches: Arc<dyn Fn(BuiltType, &RuntimeValueType) -> bool>,
    pub generics: Vec<BuiltType>,
}

impl PartialEq for BuiltType {
    fn eq(&self, other: &Self) -> bool {
        self.hashed() == other.hashed()
    }
}

pub trait CallMatches {
    fn call_matches(&self, value: &RuntimeValueType) -> bool;
}

impl CallMatches for Arc<TypeSignature> {
    fn call_matches(&self, value: &RuntimeValueType) -> bool {
        (self.clone().matches)(self.clone(), value)
    }
}

impl CallMatches for BuiltType {
    fn call_matches(&self, value: &RuntimeValueType) -> bool {
        (self.matches)(self.clone(), value)
    }
}

impl Debug for TypeSignature {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("{ Data Type Signature }")
    }
}

impl Debug for BuiltType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!(
            "{{ Data Type Signature of {} }}",
            &self.type_ref.name
        ))
    }
}


impl From<Arc<TypeSignature>> for BuiltType {
    fn from(value: Arc<TypeSignature>) -> Self {
        Self {
            type_ref: value.clone(),
            matches: value.matches_built.clone(),
            generics: Vec::new(),
        }
    }
}

impl Hash for BuiltType {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.type_ref.hash(state);
        self.generics.hash(state);
    }
}

impl BuiltType {
    pub fn hashed(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }
}

impl BuiltType {
    pub(crate) fn vis(&self) -> String {
        format!(
            "{}{}",
            &self.type_ref.name,
            if !self.generics.is_empty() {
                format!(
                    "<{}>",
                    self.generics
                        .iter()
                        .map(|x| x.vis())
                        .collect::<Vec<String>>()
                        .join(", ")
                )
            } else {
                "".to_string()
            }
        )
    }

    pub fn apply(mut self, generics: Vec<BuiltType>) -> Self {
        self.generics = generics;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Hash)]
pub enum DataTypeKind {
    BuiltIn,
    User
}

#[derive(Debug)]
pub struct Types {
    pub types: Rw<HashMap<Atom, Arc<TypeSignature>>>,
}

// pub struct GlobalYukeTypes;

impl Types {
    pub fn new() -> Self {
        Self {
            types: Rw::new(HashMap::new()),
        }
    }

    pub fn add_type(&self, types: (Atom, Arc<TypeSignature>)) {
        self.types.w().insert(types.0, types.1);
    }

    pub fn has_type(&self, target_type: Atom) -> bool {
        self.types.r().contains_key(&target_type)
    }

    pub fn find_type(
        &self,
        mut target_type: Atom,
        generics: Vec<ASTNode>,
        trace: Option<Span>,
        scope: &RuntimeScope,
    ) -> BuiltType {
        if self.types.r().contains_key(&target_type) {
            let tp = self.types.r();
            let mut kv = tp.get(&target_type).unwrap();

            let t = kv.clone();

            BuiltType::from(t).apply(
                generics
                    .iter()
                    .map(|x| {
                        let ( tp, tg) = x.value.clone().into_type().unwrap();

                        scope.find_type(tp.into(), tg, Some(x.span), None)
                    })
                    .collect(),
            )
        } else {
            Log::err(
                format!("Type '{}' couldn't be found.", &target_type),
                LogOrigin::Interpret,
            );
            if let Some(tr) = trace {
                Log::trace_span(tr);
            }
            Control::exit();
        }
    }
}

pub fn process_special_cases(
    ty: Arc<TypeSignature>,
    value: &RuntimeValueType,
) -> BuiltType {
    match value {
        RuntimeValueType::Function(fd) => {
            let mut v: BuiltType = ty.into();

            let mut generics = fd.arg_types.clone();
            generics.push(fd.ret_type.clone());

            v.generics = generics;

            v
        }
        RuntimeValueType::Array(ad) => {
            let mut v: BuiltType = ty.into();

            let generics = vec![ad.ty.clone()];

            v.generics = generics;

            v
        }
        _ => ty.into(),
    }
}

pub fn try_match(
    set: &HashMap<Atom, Arc<TypeSignature>>,
    value: &RuntimeValueType,
) -> Option<BuiltType> {
    let mut matched: Option<BuiltType> = None;

    for (k, t) in set {
        let matches = (t.matches)(t.clone(), value);

        if matches {
            matched = Some(process_special_cases(t.clone(), value));
            break;
        }
    }

    matched
}