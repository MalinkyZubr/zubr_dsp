use core::slice::SlicePattern;
use std::{mem, slice};
use crate::pipeline::communication_layer::data_management::*;
use crate::pipeline::construction_layer::node_types::pipeline_step::*;
use crate::pipeline::construction_layer::pipeline_traits::*;
use num::Num;

pub struct OverlapSaveChunks<T: Num + Sharable, const IC: usize, const NC: usize>
where
    [(); ((IC % NC) == 0) as usize - 1]:, [(); {IC / NC}]:
{
    internal_buffer: [BufferArray<T, {IC / NC}>; NC],
}

impl<T: Num + Sharable, const IC: usize, const NC: usize>
    PipelineStep<BufferArray<T, IC>, BufferArray<BufferArray<T, { IC / NC }>, NC>, 1>
    for OverlapSaveChunks<T, IC, NC>
where
    [(); ((IC % NC) == 0) as usize - 1]:,
{
    fn run_cpu(
        &mut self,
        input: &mut [DataWrapper<BufferArray<T, IC>>; 1],
        output: &mut DataWrapper<BufferArray<BufferArray<T, { IC / NC }>, NC>>,
    ) -> Result<(), ()> {
        let slice_size = IC / NC;
        for slice_num in 0..NC {
            self.internal_buffer[slice_num].read_mut().copy_from_slice(
                &input[0].read().read()[slice_num * slice_size..(slice_num * slice_size + slice_size)]
            )
        }
        output.swap()
        Ok(())
    }
}
