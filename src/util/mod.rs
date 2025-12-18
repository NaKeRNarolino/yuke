use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

pub struct Rw<T> {
    inner: RwLock<T>
}

pub type Arw<T> = Arc<Rw<T>>;

pub fn arw<T>(v: T) -> Arw<T> {
    Arc::new(Rw::new(v))
}

impl<T> Rw<T> {
    pub fn new(value: T) -> Self {
        Self {
            inner: RwLock::new(value)
        }
    }

    pub fn r(&'_ self) -> RwLockReadGuard<'_, T> {
        self.inner.read().unwrap()
    }

    pub fn w(&'_ self) -> RwLockWriteGuard<'_, T> {
        self.inner.write().unwrap()
    }
}

pub trait Unbox<T: Clone> {
    fn unbox(self) -> T;
}

impl<T: Clone> Unbox<T> for Box<T> {
    fn unbox(self) -> T {
        (*self).clone()
    }
}