use std::mem;
use crate::pipeline::communication_layer::data_management::{BufferArray, DataWrapper};
use crate::pipeline::construction_layer::node_types::pipeline_step::PipelineStep;
use crate::pipeline::construction_layer::pipeline_traits::Sharable;
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
    > PipelineStep<BufferArray<T, INPUT_BUFFER_SIZE>, BufferArray<T, OUTPUT_BUFFER_SIZE>, 1>
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
