use crate::pipeline::communication_layer::data_management::*;
use crate::pipeline::construction_layer::node_types::pipeline_step::*;
use crate::pipeline::construction_layer::pipeline_traits::*;
use num::Num;
use std::{mem, slice};

pub struct OverlapSaveBreaker<
    T: Num + Sharable,
    const InputSize: usize,
    const ChunkSize: usize,
    const FilterSize: usize,
> where
    [(); ((InputSize % ChunkSize) == 0) as usize - 1]:,
    [(); InputSize / ChunkSize]:,
    [(); ChunkSize + FilterSize - 1]:,
    [(); FilterSize - 1]:,
{
    internal_buffer:
        BufferArray<BufferArray<T, { ChunkSize + FilterSize - 1 }>, { InputSize / ChunkSize }>,
    overlap_buffer: BufferArray<T, { FilterSize - 1 }>,
}
impl<
        T: Num + Sharable,
        const InputSize: usize,
        const ChunkSize: usize,
        const FilterSize: usize,
    > OverlapSaveBreaker<T, InputSize, ChunkSize, FilterSize>
where
    [(); ((InputSize % ChunkSize) == 0) as usize - 1]:,
    [(); { InputSize / ChunkSize }]:,
    [(); { ChunkSize + FilterSize - 1 }]:,
    [(); { FilterSize - 1 }]:,
{
    pub fn new() -> Self {
        Self {
            internal_buffer: BufferArray::new(),
            overlap_buffer: BufferArray::new(),
        }
    }
}

impl<
        T: Num + Sharable,
        const InputSize: usize,
        const ChunkSize: usize,
        const FilterSize: usize,
    >
    PipelineStep<
        BufferArray<T, InputSize>,
        BufferArray<BufferArray<T, { ChunkSize + FilterSize - 1 }>, { InputSize / ChunkSize }>,
        1,
    > for OverlapSaveBreaker<T, InputSize, ChunkSize, FilterSize>
where
    [(); ((InputSize % ChunkSize) == 0) as usize - 1]:,
    [(); { ChunkSize + FilterSize - 1 }]:,
    [(); { FilterSize - 1 }]:,
{
    fn run_cpu(
        &mut self,
        input: &mut [DataWrapper<BufferArray<T, InputSize>>; 1],
        output: &mut DataWrapper<
            BufferArray<BufferArray<T, { ChunkSize + FilterSize - 1 }>, { InputSize / ChunkSize }>,
        >,
    ) -> Result<(), ()> {
        let num_chunks = InputSize / ChunkSize;
        for (idx, chunk) in input[0].read().read_mut().chunks_mut(ChunkSize).enumerate() {
            self.internal_buffer
                .get_mut(idx)
                .read_mut()
                .swap_with_slice(self.overlap_buffer.read_mut()); // put the overlap buffer inside
            self.internal_buffer.get_mut(idx).read_mut()[FilterSize - 1..].swap_with_slice(chunk);
            self.overlap_buffer
                .read_mut()
                .copy_from_slice(&chunk[chunk.len() - FilterSize + 1..])
        }
        output.swap(&mut self.internal_buffer);
        Ok(())
    }
}

pub struct OverlapSaveCombiner<
    T: Num + Sharable,
    const InputSize: usize,
    const ChunkSize: usize,
    const FilterSize: usize,
> {
    internal_buffer: BufferArray<T, { InputSize }>,
}

impl<
        T: Num + Sharable,
        const InputSize: usize,
        const ChunkSize: usize,
        const FilterSize: usize,
    > OverlapSaveCombiner<T, InputSize, ChunkSize, FilterSize>
{
    pub fn new() -> Self {
        Self {
            internal_buffer: BufferArray::new(),
        }
    }
}

impl<
        T: Num + Sharable,
        const InputSize: usize,
        const ChunkSize: usize,
        const FilterSize: usize,
    >
    PipelineStep<
        BufferArray<BufferArray<T, { ChunkSize + FilterSize - 1 }>, { InputSize / ChunkSize }>,
        BufferArray<T, InputSize>,
        1,
    > for OverlapSaveCombiner<T, InputSize, ChunkSize, FilterSize>
{
    fn run_cpu(
        &mut self,
        input: &mut [DataWrapper<
            BufferArray<BufferArray<T, { ChunkSize + FilterSize - 1 }>, { InputSize / ChunkSize }>,
        >; 1],
        output: &mut DataWrapper<BufferArray<T, InputSize>>,
    ) -> Result<(), ()> {
        for (idx, output_chunk) in self
            .internal_buffer
            .read_mut()
            .chunks_mut(ChunkSize)
            .enumerate()
        {
            output_chunk
                .swap_with_slice(&mut input[0].read().get_mut(idx).read_mut()[FilterSize - 1..]);
        }
        output.swap(&mut self.internal_buffer);
        Ok(())
    }
}
