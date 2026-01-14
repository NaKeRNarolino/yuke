use crate::store::{Atom, AtomStorage};
use crate::util::Rw;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::Arc;

pub struct SourceMaps {
    atom_to_source: HashMap<Atom, Arc<Vec<String>>>,
}

impl SourceMaps {
    fn new() -> Self {
        Self {
            atom_to_source: HashMap::new(),
        }
    }

    fn push_source(&mut self, atom: Atom, source: Vec<String>) {
        self.atom_to_source.insert(atom, Arc::new(source));
    }

    fn get_source(&self, atom: &Atom) -> Arc<Vec<String>> {
        self.atom_to_source.get(atom).unwrap().clone()
    }

    pub fn push(atom: Atom, source: Vec<String>) {
        SOURCE_MAPS.w().push_source(atom, source)
    }

    pub fn get(atom: &Atom) -> Arc<Vec<String>> {
        SOURCE_MAPS.r().get_source(atom)
    }
}

lazy_static! {
    pub static ref SOURCE_MAPS: Rw<SourceMaps> = Rw::new(SourceMaps::new());
}
