use num::complex::Complex;
use num::Num;
use num_traits::cast::cast;
use num_traits::NumCast;
use crate::pipeline::communication_layer::data_management::*;
use crate::pipeline::construction_layer::pipeline_traits::*;

pub type TFGenerator<T: Num + Sharable + NumCast, const N: usize> = fn() -> BufferArray<Complex<T>, N>;


pub fn tf_hilbert<T: Num + Sharable + NumCast, const N: usize>() -> BufferArray<Complex<T>, N> {
    let mut tf_buffer = BufferArray::<Complex<T>, N>::new();
    
    let mut pos_upper_limit = (N - 1) / 2;
    if N % 2 == 0 {
        pos_upper_limit = (N / 2) - 1;
    }

    for i in 0..N {
        if i == 0 || (N % 2 == 0 && i == N / 2) { // set the DC and nyquist to 0 for hilbert
            tf_buffer.set(i, Complex::new(cast(0.0).unwrap(), cast(0.0).unwrap()));
        } 
        else if i >= 1 && i <= pos_upper_limit { // positive frequencies multiplies by -j
            tf_buffer.set(i, Complex::new(cast(0.0).unwrap(), cast(-1.0).unwrap()));
        }
        else { // negative frequencies multiplied by j
            tf_buffer.set(i, Complex::new(cast(0.0).unwrap(), cast(1.0).unwrap()));
        }
    }
    tf_buffer
}


pub fn tf_analytic<T: Num + Sharable + NumCast, const N: usize>() -> BufferArray<Complex<T>, N> {
    let mut tf_buffer = BufferArray::<Complex<T>, N>::new();
    for bin in 1..N / 2 { // all positive frequncies doubled
        tf_buffer.set(bin, Complex::new(cast(2.0).unwrap(), cast(0).unwrap()));
    }

    for bin in N / 2 + 1..N { // all negative frequencies are set to 0
        tf_buffer.set(bin, Complex::new(cast(0.0).unwrap(), cast(0.0).unwrap()));
    }
    
    tf_buffer
}