use std::fmt::Debug;
use std::ops::{Add, Div, Mul, Neg, Sub};

pub trait HasDefault {
    fn default() -> Self;
}

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

pub trait HasID {
    fn get_id(&self) -> String;
    fn set_id(&mut self, id: &str);
}