use crate::pipeline::communication_layer::data_management::*;
use crate::pipeline::construction_layer::node_types::pipeline_step::*;
use crate::pipeline::construction_layer::pipeline_traits::*;
use num::complex::Complex;
use num::Num;
use num_traits::{cast, NumCast};
use std::ops::MulAssign;

pub struct IntoAnalytic<const BufferSize: usize> {}


impl<const BufferSize: usize> IntoAnalytic<BufferSize> {
    pub fn new() -> Self {
        Self {}
    }
}
impl<T: Num + NumCast + Sharable, const BufferSize: usize>
    PipelineStep<BufferArray<Complex<T>, BufferSize>, BufferArray<Complex<T>, BufferSize>, 1>
    for IntoAnalytic<BufferSize>
where
    Complex<T>: MulAssign<Complex<T>>,
{
    fn run_cpu(
        &mut self,
        input: &mut [DataWrapper<BufferArray<Complex<T>, BufferSize>>; 1],
        output: &mut DataWrapper<BufferArray<Complex<T>, BufferSize>>,
    ) -> Result<(), ()> {
        input[0].swap_st(output);
        
        // zero and nyquist frequencies remain unchanged

        for bin in 1..BufferSize / 2 { // all positive frequncies doubled
            output.read().mutate(bin, |x: &mut Complex<T>| {
                *x *= Complex::new(cast(2.0).unwrap(), cast(0).unwrap())
            });
        }

        for bin in BufferSize / 2 + 1..BufferSize { // all negative frequencies are set to 0
            output.read().mutate(bin, |x: &mut Complex<T>| {
                *x *= Complex::new(cast(0).unwrap(), cast(0).unwrap())
            });
        }

        Ok(())
    }
}
