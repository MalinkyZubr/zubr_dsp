//use std::ops::{Add, Div, Mul, Neg, Sub};

use num_traits::NumCast;
use num_traits::Num;


pub trait Sharable = Send + Sync + Clone + Copy + Default + 'static;
pub trait SharableNum = Sharable + Num<FromStrRadixErr=()> + NumCast;

pub trait Unit: Send + Clone {
    fn gen() -> Self;
}
impl Unit for () {
    fn gen() -> Self {
        ()
    }
}
