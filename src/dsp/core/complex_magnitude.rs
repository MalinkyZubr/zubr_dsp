use crate::engine::data_plane::communication_layer::data_management::{BufferArray};
use crate::engine::data_plane::structural::generic_node_operation::PipelineNodeOp;
use crate::engine::data_plane::structural::pipeline_type_traits::Sharable;
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
        input: &mut [BufferArray<Complex<T>, BufferSize>; 1],
        output: &mut BufferArray<T, BufferSize>,
    ) -> Result<(), ()> {
        for index in 0..BufferSize {
            output.set(index, input[0].get(index).norm())
        }
        
        Ok(())
    }
}