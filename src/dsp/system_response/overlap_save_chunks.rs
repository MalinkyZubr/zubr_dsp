use crate::pipeline::communication_layer::data_management::*;
use crate::pipeline::construction_layer::node_types::pipeline_step::*;
use crate::pipeline::construction_layer::pipeline_traits::*;
use num::Num;

pub struct OverlapSaveBreaker<
    T: Num + Sharable,
    const INPUT_SIZE: usize,
    const CHUNK_SIZE: usize,
    const FILTER_SIZE: usize,
> where
    [(); ((INPUT_SIZE % CHUNK_SIZE) == 0) as usize - 1]:,
    [(); INPUT_SIZE / CHUNK_SIZE]:,
    [(); CHUNK_SIZE + FILTER_SIZE - 1]:,
    [(); FILTER_SIZE - 1]:,
{
    internal_buffer:
        BufferArray<BufferArray<T, { CHUNK_SIZE + FILTER_SIZE - 1 }>, { INPUT_SIZE / CHUNK_SIZE }>,
    overlap_buffer: BufferArray<T, { FILTER_SIZE - 1 }>,
}
impl<
        T: Num + Sharable,
        const INPUT_SIZE: usize,
        const CHUNK_SIZE: usize,
        const FILTER_SIZE: usize,
    > OverlapSaveBreaker<T, INPUT_SIZE, CHUNK_SIZE, FILTER_SIZE>
where
    [(); ((INPUT_SIZE % CHUNK_SIZE) == 0) as usize - 1]:,
    [(); INPUT_SIZE / CHUNK_SIZE ]:,
    [(); CHUNK_SIZE + FILTER_SIZE - 1]:,
    [(); FILTER_SIZE - 1]:,
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
        const INPUT_SIZE: usize,
        const CHUNK_SIZE: usize,
        const FILTER_SIZE: usize,
    >
    PipelineStep<
        BufferArray<T, INPUT_SIZE>,
        BufferArray<BufferArray<T, { CHUNK_SIZE + FILTER_SIZE - 1 }>, { INPUT_SIZE / CHUNK_SIZE }>,
        1,
    > for OverlapSaveBreaker<T, INPUT_SIZE, CHUNK_SIZE, FILTER_SIZE>
where
    [(); ((INPUT_SIZE % CHUNK_SIZE) == 0) as usize - 1]:,
    [(); CHUNK_SIZE + FILTER_SIZE - 1]:,
    [(); FILTER_SIZE - 1]:,
{
    fn run_cpu(
        &mut self,
        input: &mut [DataWrapper<BufferArray<T, INPUT_SIZE>>; 1],
        output: &mut DataWrapper<
            BufferArray<BufferArray<T, { CHUNK_SIZE + FILTER_SIZE - 1 }>, { INPUT_SIZE / CHUNK_SIZE }>,
        >,
    ) -> Result<(), ()> {
        for (idx, chunk) in input[0].read().read_mut().chunks_mut(CHUNK_SIZE).enumerate() {
            self.internal_buffer
                .get_mut(idx)
                .read_mut()[0..FILTER_SIZE - 1]
                .swap_with_slice(self.overlap_buffer.read_mut()); // put the overlap buffer inside
            self.internal_buffer.get_mut(idx).read_mut()[FILTER_SIZE - 1..].swap_with_slice(chunk);
            self.overlap_buffer
                .read_mut()
                .copy_from_slice(&chunk[chunk.len() - FILTER_SIZE + 1..])
        }
        output.swap(&mut self.internal_buffer);
        Ok(())
    }
}

pub struct OverlapSaveCombiner<
    T: Num + Sharable,
    const INPUT_SIZE: usize,
    const CHUNK_SIZE: usize,
    const FILTER_SIZE: usize,
> {
    internal_buffer: BufferArray<T, { INPUT_SIZE }>,
}

impl<
        T: Num + Sharable,
        const INPUT_SIZE: usize,
        const CHUNK_SIZE: usize,
        const FILTER_SIZE: usize,
    > OverlapSaveCombiner<T, INPUT_SIZE, CHUNK_SIZE, FILTER_SIZE>
{
    pub fn new() -> Self {
        Self {
            internal_buffer: BufferArray::new(),
        }
    }
}

impl<
        T: Num + Sharable,
        const INPUT_SIZE: usize,
        const CHUNK_SIZE: usize,
        const FILTER_SIZE: usize,
    >
    PipelineStep<
        BufferArray<BufferArray<T, { CHUNK_SIZE + FILTER_SIZE - 1 }>, { INPUT_SIZE / CHUNK_SIZE }>,
        BufferArray<T, INPUT_SIZE>,
        1,
    > for OverlapSaveCombiner<T, INPUT_SIZE, CHUNK_SIZE, FILTER_SIZE>
{
    fn run_cpu(
        &mut self,
        input: &mut [DataWrapper<
            BufferArray<BufferArray<T, { CHUNK_SIZE + FILTER_SIZE - 1 }>, { INPUT_SIZE / CHUNK_SIZE }>,
        >; 1],
        output: &mut DataWrapper<BufferArray<T, INPUT_SIZE>>,
    ) -> Result<(), ()> {
        for (idx, output_chunk) in self
            .internal_buffer
            .read_mut()
            .chunks_mut(CHUNK_SIZE)
            .enumerate()
        {
            output_chunk
                .swap_with_slice(&mut input[0].read().get_mut(idx).read_mut()[FILTER_SIZE - 1..]);
        }
        output.swap(&mut self.internal_buffer);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::communication_layer::data_management::*;

    #[test]
    fn test_overlap_save_breaker_new() {
        let breaker: OverlapSaveBreaker<f32, 8, 4, 3> = OverlapSaveBreaker::new();
        // Test that the struct is created successfully
        // Internal state verification would require access to private fields
    }

    #[test]
    fn test_overlap_save_combiner_new() {
        let combiner: OverlapSaveCombiner<f32, 8, 4, 3> = OverlapSaveCombiner::new();
        // Test that the struct is created successfully
    }

    #[test]
    fn test_overlap_save_breaker_run_cpu_basic() {
        let mut breaker: OverlapSaveBreaker<f32, 8, 4, 3> = OverlapSaveBreaker::new();
        
        // Create input data: [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]
        let mut input_buffer = BufferArray::new_with_value(
            [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0],
        );

        let mut input_wrapper = DataWrapper::new_with_value(input_buffer);
        let mut input = [input_wrapper];
        
        let mut output_wrapper = DataWrapper::new();
        
        let result = breaker.run_cpu(&mut input, &mut output_wrapper);
        
        assert!(result.is_ok());
        
        // Verify the output has the correct structure
        // First chunk should have overlap buffer (initially zeros) + first 4 elements
        let output_data = output_wrapper.read();
        let first_chunk = output_data.get(0).read();
        
        // First chunk: [0, 0, 1, 2, 3, 4] (2 overlap + 4 data)
        assert_eq!(first_chunk[0], 0.0); // overlap buffer initially zero
        assert_eq!(first_chunk[1], 0.0); // overlap buffer initially zero
        assert_eq!(first_chunk[2], 1.0); // first data element
        assert_eq!(first_chunk[3], 2.0);
        assert_eq!(first_chunk[4], 3.0);
        assert_eq!(first_chunk[5], 4.0); // fourth data element
    }

    #[test]
    fn test_overlap_save_breaker_multiple_chunks() {
        let mut breaker: OverlapSaveBreaker<f32, 8, 4, 3> = OverlapSaveBreaker::new();

        // Create input data: [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]
        let mut input_buffer = BufferArray::new_with_value(
            [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0],
        );

        let mut input_wrapper = DataWrapper::new_with_value(input_buffer);
        let mut input = [input_wrapper];

        let mut output_wrapper = DataWrapper::new();

        let result = breaker.run_cpu(&mut input, &mut output_wrapper);
        
        assert!(result.is_ok());
        
        let output_data = output_wrapper.read();
        
        // First chunk: [0, 0, 1, 2, 3, 4]
        let first_chunk = output_data.get(0).read();
        assert_eq!(first_chunk[2], 1.0);
        assert_eq!(first_chunk[3], 2.0);
        assert_eq!(first_chunk[4], 3.0);
        assert_eq!(first_chunk[5], 4.0);
        
        // Second chunk should have overlap from first chunk + second 4 elements
        let second_chunk = output_data.get(1).read();
        // The overlap should be the last FILTER_SIZE-1 elements from previous chunk
        assert_eq!(second_chunk[2], 5.0);
        assert_eq!(second_chunk[3], 6.0);
        assert_eq!(second_chunk[4], 7.0);
        assert_eq!(second_chunk[5], 8.0);
    }

    #[test]
    fn test_roundtrip_overlap_save() {
        // Test that breaker followed by combiner preserves data (minus edge effects)
        let mut breaker: OverlapSaveBreaker<f32, 8, 4, 3> = OverlapSaveBreaker::new();
        let mut combiner: OverlapSaveCombiner<f32, 8, 4, 3> = OverlapSaveCombiner::new();
        
        // Original input
        let mut original_buffer = BufferArray::new();
        for i in 0..8 {
            original_buffer.read_mut()[i] = (i + 1) as f32;
        }
        let mut input_wrapper = DataWrapper::new_with_value(original_buffer);
        let mut breaker_input = [input_wrapper];
        
        // Break into chunks
        let mut chunked_buffer = BufferArray::new();
        let mut chunked_wrapper = DataWrapper::new_with_value(chunked_buffer);
        
        breaker.run_cpu(&mut breaker_input, &mut chunked_wrapper).unwrap();
        
        // Combine chunks back
        let mut combiner_input = [chunked_wrapper];
        let mut reconstructed_buffer = BufferArray::new();
        let mut reconstructed_wrapper = DataWrapper::new_with_value(reconstructed_buffer);
        
        combiner.run_cpu(&mut combiner_input, &mut reconstructed_wrapper).unwrap();
        
        let reconstructed_data = reconstructed_wrapper.read();
        
        // The reconstructed data should match the original
        // (in a real overlap-save implementation, we'd account for filter edge effects)
        for i in 0..8 {
            assert_eq!(reconstructed_data.read()[i], (i + 1) as f32);
        }
    }
}