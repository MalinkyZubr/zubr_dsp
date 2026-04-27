use crate::engine::communication_layer::data_management::{BufferArray, DataWrapper};
use crate::engine::structural::generic_node_operation::PipelineNodeOp;
use crate::engine::structural::pipeline_type_traits::SharableNum;
use num_traits::{FromBytes, ToBytes};
use std::mem::size_of;

pub struct NumToBytes {}
impl<T: SharableNum + ToBytes<Bytes=[u8; size_of::<T>()]>, const BUFFER_SIZE: usize>
PipelineNodeOp<BufferArray<T, BUFFER_SIZE>, BufferArray<u8, { BUFFER_SIZE * size_of::<T>() }>, 1>
for NumToBytes
{
    fn run_cpu(&mut self, _input: &mut [DataWrapper<BufferArray<T, BUFFER_SIZE>>; 1], _output: &mut DataWrapper<BufferArray<u8, { BUFFER_SIZE * size_of::<T>() }>>) -> Result<(), ()> {
        for (idx, numerical) in _input[0].read().read().iter().enumerate() {
            let bytes: [u8; size_of::<T>()] = numerical.to_le_bytes();
            _output.read().read_mut()[idx * bytes.len()..(idx + 1) * bytes.len()].copy_from_slice(&bytes);
        }

        Ok(())
    }
}

pub struct BytesToNum {}
impl<T: SharableNum + FromBytes<Bytes=[u8; size_of::<T>()]>, const BUFFER_SIZE: usize>
PipelineNodeOp<BufferArray<u8, { BUFFER_SIZE * size_of::<T>() }>, BufferArray<T, BUFFER_SIZE>, 1>
for BytesToNum
{
    fn run_cpu(&mut self, _input: &mut [DataWrapper<BufferArray<u8, { BUFFER_SIZE * size_of::<T>() }>>; 1], _output: &mut DataWrapper<BufferArray<T, BUFFER_SIZE>>) -> Result<(), ()> {
        for (idx, bytes) in _input[0].read().read_mut().chunks_exact(size_of::<T>()).enumerate() {
            let numerical = T::from_le_bytes(bytes.try_into().unwrap());
            _output.read().set(idx, numerical);
        }

        Ok(())
    }
}
