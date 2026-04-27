use std::mem;
use crate::engine::communication_layer::data_management::{BufferArray, DataWrapper};
use crate::engine::structural::generic_node_operation::PipelineNodeOp;
use crate::engine::structural::pipeline_type_traits::Sharable;
use num::{Num, NumCast};

pub struct SimpleDownsampler<const INPUT_BUFFER_SIZE: usize, const OUTPUT_BUFFER_SIZE: usize> {
    stride: usize,
}
impl<const INPUT_BUFFER_SIZE: usize, const OUTPUT_BUFFER_SIZE: usize>
    SimpleDownsampler<INPUT_BUFFER_SIZE, OUTPUT_BUFFER_SIZE>
{
    pub fn new() -> Self {
        assert!(INPUT_BUFFER_SIZE > OUTPUT_BUFFER_SIZE);
        Self {
            stride: INPUT_BUFFER_SIZE / OUTPUT_BUFFER_SIZE,
        }
    }
}
impl<
        T: Sharable + Num + NumCast,
        const INPUT_BUFFER_SIZE: usize,
        const OUTPUT_BUFFER_SIZE: usize,
    > PipelineNodeOp<BufferArray<T, INPUT_BUFFER_SIZE>, BufferArray<T, OUTPUT_BUFFER_SIZE>, 1>
    for SimpleDownsampler<INPUT_BUFFER_SIZE, OUTPUT_BUFFER_SIZE>
{
    fn run_cpu(&mut self, _input: &mut [DataWrapper<BufferArray<T, INPUT_BUFFER_SIZE>>; 1], _output: &mut DataWrapper<BufferArray<T, OUTPUT_BUFFER_SIZE>>) -> Result<(), ()> {
        let mut index = 0;
        
        while index < OUTPUT_BUFFER_SIZE {
            mem::swap(_input[0].read().get_mut(index * self.stride), _output.read().get_mut(index));
        }
        Ok(())
    }
}
