use crate::interpret::RuntimeScope;
use crate::interpret::structs::RuntimeValue;
use crate::lexer::structs::Span;
use crate::log::{Control, Log, LogOrigin};
use crate::parser::structs::ASTNode;
use crate::store::{Atom, AtomStorage};
use crate::util::{Rw, Unbox};
use lazy_static::lazy_static;
use std::collections::{HashMap, VecDeque};
use std::fmt::{Debug, Display, Formatter, Write};
use std::sync::Arc;
use colored::Colorize;
use uuid::Uuid;
use walrus::Data;

#[derive(Clone)]
pub struct DataTypeSignature {
    pub name: String,
    pub visual_name: String,
    pub kind: DataTypeKind,
    pub matches: Arc<fn(Arc<DataTypeSignature>, &RuntimeValue) -> bool>,
    pub matches_finalized: Arc<fn(FinalizedDataType, &RuntimeValue) -> bool>,
    pub children: HashMap<Atom, Arc<DataTypeSignature>>,
}

#[derive(Clone)]
pub struct TypeSig {
    pub name: String,
    pub visual_name: String,
    pub kind: DataTypeKind,
    pub children: HashMap<Atom, Arc<TypeSig>>,
    pub generics: Vec<TypeSig>,
}

#[derive(PartialEq, Clone)]
pub enum DataType {
    Num(NumTypes),
    Bln,
    Str,
    Uni,
    Null,
    Typ,
    Fnc(Vec<DataType>),
    Dynamic { name: String, value: DynamicType },
    Array(Box<DataType>)
}

#[derive(PartialEq, Clone)]
pub enum DynamicType {
    Struct(HashMap<Atom, DataType>)
}

#[derive(PartialEq, Clone)]
pub enum NumTypes {
    Int,
    Flt,
    Gen,
}

impl Display for DataType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", &match self {
            DataType::Num(nt) => format!(
                "Num{}",
                if nt.to_string() != "*" {
                    format!("{}{}", ".", nt.to_string())
                } else {
                    "".to_string()
                }
            ),
            DataType::Bln => "Bln".to_string(),
            DataType::Str => "Str".to_string(),
            DataType::Uni => "Uni".to_string(),
            DataType::Null => "<Null>".to_string(),
            DataType::Typ { .. } => "Typ".to_string(),
            DataType::Fnc(g) => format!("Fnc<{}>",
                                        g.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(",")),
            DataType::Dynamic { name, .. } => format!("?{}", name),
            DataType::Array(t) => format!("Arr<{}>", t)
        }.yellow()))
    }
}

impl Display for NumTypes {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            NumTypes::Int => "Int",
            NumTypes::Flt => "Flt",
            NumTypes::Gen => "*",
        })
    }
}

impl DataType {
    pub fn from_atoms(atoms: Vec<Atom>) -> DataType {
        let num_atom = AtomStorage::atom("Num".to_string());
        let int_atom = AtomStorage::atom("Int".to_string());
        let flt_atom = AtomStorage::atom("Flt".to_string());
        let str_atom = AtomStorage::atom("Str".to_string());
        let bln_atom = AtomStorage::atom("Bln".to_string());
        let uni_atom = AtomStorage::atom("Uni".to_string());

        if atoms[0] == num_atom {
            DataType::Num(if let Some(v) = atoms.get(1) {
                if v == &int_atom {
                    NumTypes::Int
                } else {
                    NumTypes::Flt
                }
            } else {
                NumTypes::Gen
            })
        } else if atoms[0] == str_atom {
            DataType::Str
        } else if atoms[0] == bln_atom {
            DataType::Bln
        } else if atoms[0] == uni_atom {
            DataType::Uni
        } else {
            DataType::Null
        }
    }

    pub fn matches(&self, other: &Self) -> bool {
        (self == other) || self.can_be_cast_into(other)
    }

    pub fn can_be_cast_into(&self, other: &Self) -> bool {
        match (self, other) {
            (DataType::Num(NumTypes::Int), DataType::Num(NumTypes::Gen)) => true,
            (DataType::Num(NumTypes::Flt), DataType::Num(NumTypes::Gen)) => true,
            (DataType::Array(t), DataType::Array(tt)) => t.matches(&tt),
            (_, _) => false,
        }
    }

    pub fn num() -> DataType {
        DataType::Num(NumTypes::Gen)
    }

    pub fn is_num(&self) -> bool {
        matches!(self, DataType::Num(_))
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

    pub fn is_struct(&self) -> bool {
        matches!(self, DataType::Dynamic { value: DynamicType::Struct(_), .. })
    }

    pub fn is_fnc(&self) -> bool {
        matches!(self, DataType::Fnc(_))
    }

    pub fn is_arr(&self) -> bool { matches!(self, DataType::Array(_)) }
}

impl TypeSig {
    pub fn generics(&self, generics: Vec<TypeSig>) -> Self {
        let mut s_c = self.clone();
        s_c.generics = generics;
        s_c
    }
}

impl PartialEq for TypeSig {
    fn eq(&self, other: &Self) -> bool {
        (self.name == other.name
            && self.kind == other.kind
            && self.visual_name == other.visual_name)
    }
}

#[derive(Clone)]
pub struct FinalizedDataType {
    pub name: String,
    pub visual_name: String,
    pub matches: Arc<dyn Fn(FinalizedDataType, &RuntimeValue) -> bool>,
    pub generics: Vec<FinalizedDataType>,
    pub uuid: Uuid,
}

impl PartialEq for FinalizedDataType {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.generics == other.generics && self.uuid == other.uuid
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
        f.write_str(&format!(
            "{{ Data Type Signature of {} }}",
            &self.visual_name
        ))
    }
}

impl From<DataTypeSignature> for FinalizedDataType {
    fn from(value: DataTypeSignature) -> Self {
        Self {
            uuid: Uuid::new_v5(&Uuid::NAMESPACE_OID, &value.name.as_ref()),
            name: value.name,
            visual_name: value.visual_name,
            matches: value.matches_finalized,
            generics: Vec::new(),
        }
    }
}

impl From<Arc<DataTypeSignature>> for FinalizedDataType {
    fn from(value: Arc<DataTypeSignature>) -> Self {
        Self {
            uuid: Uuid::new_v5(&Uuid::NAMESPACE_OID, &value.name.as_ref()),
            name: value.name.clone(),
            visual_name: value.visual_name.clone(),
            matches: value.matches_finalized.clone(),
            generics: Vec::new(),
        }
    }
}

impl TypeSig {
    pub fn vis(&self) -> String {
        format!(
            "{}{}",
            &self.visual_name,
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

    pub fn null() -> TypeSig {
        TypeSig {
            name: "Null".to_string(),
            visual_name: "Null".to_string(),
            kind: DataTypeKind::BuiltIn,
            children: HashMap::new(),
            generics: Vec::new(),
        }
    }

    pub fn uni() -> TypeSig {
        TypeSig {
            name: "Uni".to_string(),
            visual_name: "Uni".to_string(),
            kind: DataTypeKind::BuiltIn,
            children: HashMap::new(),
            generics: Vec::new(),
        }
    }
}

impl Display for TypeSig {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.vis())
    }
}

impl FinalizedDataType {
    pub(crate) fn vis(&self) -> String {
        format!(
            "{}{}",
            &self.visual_name,
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

    pub fn apply(mut self, generics: Vec<FinalizedDataType>) -> Self {
        self.generics = generics;
        self
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum DataTypeKind {
    BuiltIn,
    Mod,
}

#[derive(Debug)]
pub struct Types {
    pub types: Rw<HashMap<Atom, Arc<DataTypeSignature>>>,
}

// pub struct GlobalYukeTypes;

impl Types {
    pub fn new() -> Self {
        Self {
            types: Rw::new(HashMap::new()),
        }
    }

    pub fn add_type(&self, types: (Atom, Arc<DataTypeSignature>)) {
        self.types.w().insert(types.0, types.1);
    }

    pub fn has_type(&self, target_type: Atom) -> bool {
        self.types.r().contains_key(&target_type)
    }

    pub fn find_type(
        &self,
        mut target_type: VecDeque<Atom>,
        generics: Vec<ASTNode>,
        trace: Option<Span>,
        scope: &RuntimeScope,
    ) -> FinalizedDataType {
        let name = Vec::from(target_type.clone())
            .iter()
            .map(|x| AtomStorage::string(*x).unwrap().as_str())
            .collect::<Vec<&str>>()
            .join(".");
        let initial = *target_type.front().unwrap();

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

                        scope.find_type(tp.into(), tg, dy, Some(x.span), None)
                    })
                    .collect(),
            )
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
//
// impl GlobalYukeTypes {
//     pub fn add_type(types: (Atom, Arc<TypeSig>)) {
//         GLOBAL_Y_TYPES.w().insert(types.0, types.1);
//     }
//
//     pub fn has_type(target_type: Atom) -> bool {
//         GLOBAL_Y_TYPES.r().contains_key(&target_type)
//     }
//
//     pub fn find_type_s(
//         target_type: Vec<impl Into<String>>,
//         generics: Vec<ASTNode>,
//         trace: Option<Span>,
//     ) -> TypeSig {
//         Self::find_type(
//             target_type
//                 .into_iter()
//                 .map(|x| AtomStorage::atom(x.into()))
//                 .collect(),
//             generics,
//             trace,
//         )
//     }
//
//     pub fn find_type(
//         mut target_type: VecDeque<Atom>,
//         generics: Vec<ASTNode>,
//         trace: Option<Span>,
//     ) -> TypeSig {
//         let name = Vec::from(target_type.clone())
//             .iter()
//             .map(|x| AtomStorage::string(*x).unwrap().as_str())
//             .collect::<Vec<&str>>()
//             .join(".");
//         let initial = *target_type.front().unwrap();
//
//         if GLOBAL_Y_TYPES.r().contains_key(&initial) {
//             target_type.pop_front();
//             let tp = GLOBAL_Y_TYPES.r();
//             let mut kv = tp.get(&initial).unwrap();
//
//             while !target_type.is_empty() {
//                 let next = target_type.pop_front().unwrap();
//                 kv = match kv.children.get(&next) {
//                     None => {
//                         Log::err(
//                             format!("Type '{}' couldn't be found.", &name),
//                             LogOrigin::Interpret,
//                         );
//                         if let Some(tr) = trace {
//                             Log::trace_span(tr);
//                         }
//                         Control::exit();
//                     }
//                     Some(v) => v,
//                 }
//             }
//
//             let t = kv.clone();
//
//             (t).generics(
//                 generics
//                     .iter()
//                     .map(|x| {
//                         let (dy, tp, tg) = x.value.clone().into_type().unwrap();
//
//                         GlobalYukeTypes::find_type(tp.into(), tg, Some(x.span))
//                     })
//                     .collect(),
//             )
//         } else {
//             Log::err(
//                 format!("Type '{}' couldn't be found.", &name),
//                 LogOrigin::Interpret,
//             );
//             if let Some(tr) = trace {
//                 Log::trace_span(tr);
//             }
//             Control::exit();
//         }
//     }
// }

pub fn process_special_cases(
    ty: Arc<DataTypeSignature>,
    value: &RuntimeValue,
) -> FinalizedDataType {
    match value {
        RuntimeValue::Function(fd) => {
            let mut v: FinalizedDataType = ty.into();

            let mut generics = fd.arg_types.clone();
            generics.push(fd.ret_type.clone());

            v.generics = generics;

            v
        }
        RuntimeValue::Array(ad) => {
            let mut v: FinalizedDataType = ty.into();

            let generics = vec![ad.ty.clone()];

            v.generics = generics;

            v
        }
        _ => ty.into(),
    }
}

pub fn try_match(
    set: &HashMap<Atom, Arc<DataTypeSignature>>,
    value: &RuntimeValue,
) -> Option<FinalizedDataType> {
    let mut matched: Option<FinalizedDataType> = None;

    for (k, t) in set {
        let matches = (t.matches)(t.clone(), value);

        if matches {
            if !t.children.is_empty() {
                matched = try_match(&t.children, value);
            } else {
                matched = Some(process_special_cases(t.clone(), value));
            }
            break;
        }
    }

    matched
}

lazy_static! {
    // pub static ref GLOBAL_Y_TYPES: Rw<HashMap<Atom, Arc<TypeSig>>> = Rw::new(HashMap::new());
}
