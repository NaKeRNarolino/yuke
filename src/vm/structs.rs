use std::ops::{Add, Div, Mul, Sub};

#[derive(Debug)]
pub enum RuntimeValue {
    Num(f64),
    Str(String),
    Bln(bool),
}

impl From<f64> for RuntimeValue {
    fn from(value: f64) -> Self {
        RuntimeValue::Num(value)
    }
}

impl From<String> for RuntimeValue {
    fn from(value: String) -> Self {
        RuntimeValue::Str(value)
    }
}

impl From<bool> for RuntimeValue {
    fn from(value: bool) -> Self {
        RuntimeValue::Bln(value)
    }
}

impl Add for RuntimeValue {
    type Output = RuntimeValue;

    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (RuntimeValue::Num(x), RuntimeValue::Num(y)) => RuntimeValue::Num(x + y),
            (RuntimeValue::Str(x), RuntimeValue::Str(y)) => {
                RuntimeValue::Str(format!("{}{}", x, y))
            }
            (RuntimeValue::Bln(x), RuntimeValue::Bln(y)) => RuntimeValue::Bln(x || y),
            (f, s) => {
                panic!("Operation '+' is not implemented for {:?} and {:?}.", f, s);
            }
        }
    }
}

impl Mul for RuntimeValue {
    type Output = RuntimeValue;

    fn mul(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (RuntimeValue::Num(x), RuntimeValue::Num(y)) => RuntimeValue::Num(x * y),
            (RuntimeValue::Str(x), RuntimeValue::Num(y)) => {
                RuntimeValue::Str(x.repeat(y.floor() as usize))
            }
            (RuntimeValue::Bln(x), RuntimeValue::Bln(y)) => RuntimeValue::Bln(x && y),
            (f, s) => {
                panic!("Operation '*' is not implemented for {:?} and {:?}.", f, s);
            }
        }
    }
}

impl Div for RuntimeValue {
    type Output = RuntimeValue;

    fn div(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (RuntimeValue::Num(x), RuntimeValue::Num(y)) => RuntimeValue::Num(x / y),
            (f, s) => {
                panic!("Operation '/' is not implemented for {:?} and {:?}.", f, s);
            }
        }
    }
}

impl Sub for RuntimeValue {
    type Output = RuntimeValue;

    fn sub(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (RuntimeValue::Num(x), RuntimeValue::Num(y)) => RuntimeValue::Num(x - y),
            (f, s) => {
                panic!("Operation '-' is not implemented for {:?} and {:?}.", f, s);
            }
        }
    }
}
