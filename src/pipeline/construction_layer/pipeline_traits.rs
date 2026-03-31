//use std::ops::{Add, Div, Mul, Neg, Sub};

pub trait Sharable = Send + Sync + Clone + Copy + Default + 'static;

pub trait Source {}
pub trait Sink {}

pub trait Unit: Send + Clone {
    fn gen() -> Self;
}
impl Unit for () {
    fn gen() -> Self {
        ()
    }
}
