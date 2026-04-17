use crate::pipeline::communication_layer::data_management::*;
use crate::pipeline::construction_layer::node_types::pipeline_step::*;
use crate::pipeline::construction_layer::pipeline_traits::*;
use num::complex::Complex;
use num::Num;
use num_traits::{cast, Float, NumCast};
use std::ops::MulAssign;


pub struct ComplexMagnitude {}
impl ComplexMagnitude {
    pub fn new() -> Self {
        Self {}
    }
}

impl <T: Sharable + Num + NumCast + Float, const BufferSize: usize> PipelineStep<BufferArray<Complex<T>, BufferSize>, BufferArray<T, BufferSize>, 1> for ComplexMagnitude {
    fn run_cpu(
        &mut self,
        input: &mut [DataWrapper<BufferArray<Complex<T>, BufferSize>>; 1],
        output: &mut DataWrapper<BufferArray<T, BufferSize>>,
    ) -> Result<(), ()> {
        for index in 0..BufferSize {
            output.read().set(index, input[0].read().get(index).norm())
        }
        
        Ok(())
    }
}