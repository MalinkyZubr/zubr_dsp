use crate::engine::communication_layer::data_management::*;
use crate::engine::structural::generic_node_operation::*;
use crate::engine::structural::pipeline_type_traits::*;
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

impl <T: Sharable + Num + NumCast + Float, const BufferSize: usize> PipelineNodeOp<BufferArray<Complex<T>, BufferSize>, BufferArray<T, BufferSize>, 1> for ComplexMagnitude {
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