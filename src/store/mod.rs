pub mod sourcemap;

use lazy_static::lazy_static;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::sync::RwLock;
use thiserror::Error;

pub struct AtomStorage {
    string_to_atom: HashMap<String, usize>,
    atom_to_string: Vec<String>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Atom(pub usize);

impl Display for Atom {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(AtomStorage::string(*self).unwrap())
    }
}

#[derive(Error, Debug)]
pub enum AtomStorageError {
    #[error("Atom {atom:?} has no string attached to it.")]
    AtomNotFound { atom: Atom },
}

impl AtomStorage {
    fn new() -> Self {
        AtomStorage {
            string_to_atom: HashMap::new(),
            atom_to_string: Vec::new(),
        }
    }

    pub fn get_atom(&mut self, string: String) -> Atom {
        if let Some(id) = self.string_to_atom.get(&string) {
            Atom(*id)
        } else {
            self.atom_to_string.push(string.clone());
            self.string_to_atom
                .insert(string, self.atom_to_string.len() - 1);

            Atom(self.atom_to_string.len() - 1)
        }
    }

    pub fn get_string(&self, atom: Atom) -> Result<&String, AtomStorageError> {
        match self.atom_to_string.get(atom.0) {
            None => Err(AtomStorageError::AtomNotFound { atom }),
            Some(v) => Ok(v),
        }
    }

    pub fn atom(string: String) -> Atom {
        ATOM_TABLE.write().unwrap().get_atom(string)
    }

    pub fn string(atom: Atom) -> Result<&'static String, AtomStorageError> {
        let t = ATOM_TABLE.read().unwrap();

        match t.get_string(atom) {
            Ok(v) => Ok(unsafe { &*(v as *const String) }),
            Err(e) => Err(e),
        }
    }
}

lazy_static! {
    pub static ref ATOM_TABLE: RwLock<AtomStorage> = RwLock::new(AtomStorage::new());
}
