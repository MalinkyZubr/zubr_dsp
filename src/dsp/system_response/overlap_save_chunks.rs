use crate::pipeline::communication_layer::data_management::*;
use crate::pipeline::communication_layer::generic_constraints::*;
use crate::pipeline::construction_layer::node_types::pipeline_step::*;
use crate::pipeline::construction_layer::pipeline_traits::*;
use num::Num;

pub struct OverlapSaveBreaker<
    T: Num + Sharable,
    const INPUT_SIZE: usize,
    const OVERLAPPING_BLOCK_SIZE: usize,
    const NON_OVERLAPPING_BLOCK_SIZE: usize,
    const NUM_CHUNKS: usize,
    const UNPADDED_FILTER_SIZE: usize,
> where
    [(); { (INPUT_SIZE / NON_OVERLAPPING_BLOCK_SIZE == NUM_CHUNKS) as usize }]: True,
    [(); {
        (NON_OVERLAPPING_BLOCK_SIZE <= OVERLAPPING_BLOCK_SIZE - NON_OVERLAPPING_BLOCK_SIZE + 1)
            as usize
    }]: True,
{
    internal_buffer: BufferArray<BufferArray<T, OVERLAPPING_BLOCK_SIZE>, NUM_CHUNKS>,
    overlap_buffer: BufferArray<T, { OVERLAPPING_BLOCK_SIZE - NON_OVERLAPPING_BLOCK_SIZE }>,
}
impl<
        T: Num + Sharable,
        const INPUT_SIZE: usize, // the total size of the input block
        const OVERLAPPING_BLOCK_SIZE: usize, // equal to the FFT size, padded filter size
        const NON_OVERLAPPING_BLOCK_SIZE: usize, // the number of new samples per output block
        const NUM_CHUNKS: usize, // the number of chunks to break the input into, each of size OVERLAPPING_BLOCK_SIZE
        const UNPADDED_FILTER_SIZE: usize, // the size of the unpadded filter, the filter size minus however many 0 coefficients are added
    >
    OverlapSaveBreaker<
        T,
        INPUT_SIZE,
        OVERLAPPING_BLOCK_SIZE,
        NON_OVERLAPPING_BLOCK_SIZE,
        NUM_CHUNKS,
        UNPADDED_FILTER_SIZE,
    >
where
    [(); { (INPUT_SIZE / NON_OVERLAPPING_BLOCK_SIZE == NUM_CHUNKS) as usize }]: True,
    [(); {
        (NON_OVERLAPPING_BLOCK_SIZE <= OVERLAPPING_BLOCK_SIZE - NON_OVERLAPPING_BLOCK_SIZE + 1)
            as usize
    }]: True,
{
    pub fn new() -> Self {
        assert!(
            NON_OVERLAPPING_BLOCK_SIZE <= OVERLAPPING_BLOCK_SIZE - NON_OVERLAPPING_BLOCK_SIZE + 1
        );

        Self {
            internal_buffer: BufferArray::new(),
            overlap_buffer: BufferArray::new(),
        }
    }
}

impl<
        T: Num + Sharable,
        const INPUT_SIZE: usize,
        const OVERLAPPING_BLOCK_SIZE: usize,
        const NON_OVERLAPPING_BLOCK_SIZE: usize,
        const NUM_CHUNKS: usize,
        const UNPADDED_FILTER_SIZE: usize,
    >
    PipelineStep<
        BufferArray<T, INPUT_SIZE>,
        BufferArray<BufferArray<T, OVERLAPPING_BLOCK_SIZE>, NUM_CHUNKS>,
        1,
    >
    for OverlapSaveBreaker<
        T,
        INPUT_SIZE,
        OVERLAPPING_BLOCK_SIZE,
        NON_OVERLAPPING_BLOCK_SIZE,
        NUM_CHUNKS,
        UNPADDED_FILTER_SIZE,
    >
where
    [(); { (INPUT_SIZE / NON_OVERLAPPING_BLOCK_SIZE == NUM_CHUNKS) as usize }]: True,
    [(); {
        (NON_OVERLAPPING_BLOCK_SIZE <= OVERLAPPING_BLOCK_SIZE - NON_OVERLAPPING_BLOCK_SIZE + 1)
            as usize
    }]: True,
{
    fn run_cpu(
        &mut self,
        input: &mut [DataWrapper<BufferArray<T, INPUT_SIZE>>; 1],
        output: &mut DataWrapper<
            BufferArray<BufferArray<T, { OVERLAPPING_BLOCK_SIZE }>, { NUM_CHUNKS }>,
        >,
    ) -> Result<(), ()> {
        let size = input[0].read_immut().len();
        let chunks = input[0]
            .read()
            .read_mut()
            .chunks_mut(NON_OVERLAPPING_BLOCK_SIZE);
        for (idx, chunk) in chunks.enumerate() {
            self.internal_buffer.get_mut(idx).read_mut()
                [0..OVERLAPPING_BLOCK_SIZE - NON_OVERLAPPING_BLOCK_SIZE]
                .swap_with_slice(self.overlap_buffer.read_mut()); // put the overlap buffer inside
            self.internal_buffer.get_mut(idx).read_mut()
                [OVERLAPPING_BLOCK_SIZE - NON_OVERLAPPING_BLOCK_SIZE..]
                .copy_from_slice(chunk);

            self.overlap_buffer.read_mut().copy_from_slice(
                &chunk[chunk.len() - (OVERLAPPING_BLOCK_SIZE - NON_OVERLAPPING_BLOCK_SIZE)..],
            )
        }
        output.swap(&mut self.internal_buffer);
        Ok(())
    }
}

pub struct OverlapSaveCombiner<
    const INPUT_SIZE: usize,                 // the total size of the input block
    const OVERLAPPING_BLOCK_SIZE: usize,     // equal to the FFT size, padded filter size
    const NON_OVERLAPPING_BLOCK_SIZE: usize, // the number of new samples per output block
    const NUM_CHUNKS: usize, // the number of chunks to break the input into, each of size OVERLAPPING_BLOCK_SIZE
    const UNPADDED_FILTER_SIZE: usize, // the size of the unpadded filter, the filter size minus however many 0 coefficients are added
> where
    [(); { (INPUT_SIZE / NON_OVERLAPPING_BLOCK_SIZE == NUM_CHUNKS) as usize }]: True,
    [(); {
        (NON_OVERLAPPING_BLOCK_SIZE <= OVERLAPPING_BLOCK_SIZE - NON_OVERLAPPING_BLOCK_SIZE + 1)
            as usize
    }]: True, {}

impl<
        const INPUT_SIZE: usize,                 // the total size of the input block
        const OVERLAPPING_BLOCK_SIZE: usize,     // equal to the FFT size, padded filter size
        const NON_OVERLAPPING_BLOCK_SIZE: usize, // the number of new samples per output block
        const NUM_CHUNKS: usize, // the number of chunks to break the input into, each of size OVERLAPPING_BLOCK_SIZE
        const UNPADDED_FILTER_SIZE: usize, // the size of the unpadded filter, the filter size minus however many 0 coefficients are added
    >
    OverlapSaveCombiner<
        INPUT_SIZE,
        OVERLAPPING_BLOCK_SIZE,
        NON_OVERLAPPING_BLOCK_SIZE,
        NUM_CHUNKS,
        UNPADDED_FILTER_SIZE,
    >
where
    [(); { (INPUT_SIZE / NON_OVERLAPPING_BLOCK_SIZE == NUM_CHUNKS) as usize }]: True,
    [(); {
        (NON_OVERLAPPING_BLOCK_SIZE <= OVERLAPPING_BLOCK_SIZE - NON_OVERLAPPING_BLOCK_SIZE + 1)
            as usize
    }]: True,
{
    pub fn new() -> Self {
        Self {}
    }
}

impl<
        T: Num + Sharable,
        const INPUT_SIZE: usize, // the total size of the input block
        const OVERLAPPING_BLOCK_SIZE: usize, // equal to the FFT size, padded filter size
        const NON_OVERLAPPING_BLOCK_SIZE: usize, // the number of new samples per output block
        const NUM_CHUNKS: usize, // the number of chunks to break the input into, each of size OVERLAPPING_BLOCK_SIZE
        const UNPADDED_FILTER_SIZE: usize, // the size of the unpadded filter, the filter size minus however many 0 coe
    >
    PipelineStep<
        BufferArray<BufferArray<T, OVERLAPPING_BLOCK_SIZE>, NUM_CHUNKS>,
        BufferArray<T, INPUT_SIZE>,
        1,
    >
    for OverlapSaveCombiner<
        INPUT_SIZE,
        OVERLAPPING_BLOCK_SIZE,
        NON_OVERLAPPING_BLOCK_SIZE,
        NUM_CHUNKS,
        UNPADDED_FILTER_SIZE,
    >
where
    [(); { (INPUT_SIZE / NON_OVERLAPPING_BLOCK_SIZE == NUM_CHUNKS) as usize }]: True,
    [(); {
        (NON_OVERLAPPING_BLOCK_SIZE <= OVERLAPPING_BLOCK_SIZE - NON_OVERLAPPING_BLOCK_SIZE + 1)
            as usize
    }]: True,
{
    fn run_cpu(
        &mut self,
        input: &mut [DataWrapper<BufferArray<BufferArray<T, OVERLAPPING_BLOCK_SIZE>, NUM_CHUNKS>>;
                 1],
        output: &mut DataWrapper<BufferArray<T, INPUT_SIZE>>,
    ) -> Result<(), ()> {
        for (idx, output_chunk) in output
            .read()
            .read_mut()
            .chunks_mut(NON_OVERLAPPING_BLOCK_SIZE)
            .enumerate()
        {
            output_chunk.swap_with_slice(
                &mut input[0].read().get_mut(idx).read_mut()
                    [OVERLAPPING_BLOCK_SIZE - NON_OVERLAPPING_BLOCK_SIZE..],
            );
        }
        Ok(())
    }
}


/// This function generates the overlap save steps for given signal parameters.
/// Params:
/// - `T`: The type of the signal.
/// - `INPUT_SIZE`: The size of the input signal. Some large block of samples.
/// - `OVERLAPPING_BLOCK_SIZE`: The size of the output blocks. This must be equal to the FFT size, and the padded filter size.
/// - `NON_OVERLAPPING_BLOCK_SIZE`: This is how many unique samples are present in each block. IE, if the OVERLAPPING_BLOCK_SIZE is 1024, how many of those samples are not from overlap with the previous block?
/// - `NUM_CHUNKS`: The number of chunks to split the input signal into.
/// - `UNPADDED_FILTER_SIZE`: The size of the unpadded filter.
pub fn generate_overlap_save_steps<
    T: Num + Sharable,
    const INPUT_SIZE: usize,
    const OVERLAPPING_BLOCK_SIZE: usize,
    const NON_OVERLAPPING_BLOCK_SIZE: usize,
    const NUM_CHUNKS: usize,
    const UNPADDED_FILTER_SIZE: usize,
>() -> (
    OverlapSaveBreaker<
        T,
        INPUT_SIZE,
        OVERLAPPING_BLOCK_SIZE,
        NON_OVERLAPPING_BLOCK_SIZE,
        NUM_CHUNKS,
        UNPADDED_FILTER_SIZE,
    >,
    OverlapSaveCombiner<
        INPUT_SIZE,
        OVERLAPPING_BLOCK_SIZE,
        NON_OVERLAPPING_BLOCK_SIZE,
        NUM_CHUNKS,
        UNPADDED_FILTER_SIZE,
    >,
)
where
    [(); { (INPUT_SIZE / NON_OVERLAPPING_BLOCK_SIZE == NUM_CHUNKS) as usize }]: True,
    [(); {
        (NON_OVERLAPPING_BLOCK_SIZE <= OVERLAPPING_BLOCK_SIZE - NON_OVERLAPPING_BLOCK_SIZE + 1)
            as usize
    }]: True,
{
    (OverlapSaveBreaker::new(), OverlapSaveCombiner::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::communication_layer::data_management::*;

    #[test]
    fn test_overlap_save_breaker_new() {
        let breaker: OverlapSaveBreaker<f32, 1024, 32, 16, 64, 16> = OverlapSaveBreaker::new();
        let (mut breaker, mut combiner) =
            generate_overlap_save_steps::<f32, 1024, 512, 256, 4, 512>();
    }

    #[test]
    fn test_overlap_save_combiner_new() {
        let combiner: OverlapSaveCombiner<1024, 32, 16, 64, 16> = OverlapSaveCombiner::new();
        // Test that the struct is created successfully
    }

    #[test]
    fn test_overlap_save_breaker_run_cpu_basic() {
        let mut breaker: OverlapSaveBreaker<f32, 1024, 32, 16, 64, 16> = OverlapSaveBreaker::new();

        // Create input data: [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]
        let reference_buffer: [f32; 1024] = rand::random();
        let input_buffer = BufferArray::new_with_value(reference_buffer.clone());

        let mut input_wrapper = DataWrapper::new_with_value(input_buffer);
        let mut input = [input_wrapper];

        let mut output_wrapper = DataWrapper::new();

        let result = breaker.run_cpu(&mut input, &mut output_wrapper);

        assert!(result.is_ok());

        let first_output = output_wrapper.read().get(0).read();
        assert_eq!(first_output[0..16], [0.0; 16]);

        println!("THIRD CHUNK: {:?}", output_wrapper.read().get(2).read());

        for (idx, chunk) in output_wrapper
            .read_immut()
            .read()
            .iter()
            .skip(1)
            .enumerate()
        {
            let out_chunk_ref = chunk.read();
            let prev_chunk_ref = output_wrapper.read_immut().get(idx).read();
            println!("OUT CUnK {:?}", out_chunk_ref);
            println!("PREV CUNK {:?}", prev_chunk_ref);
            assert_eq!(out_chunk_ref[0..16], prev_chunk_ref[16..]);
            assert_eq!(
                out_chunk_ref[16..],
                reference_buffer[(idx + 1) * 16..(idx + 2) * 16]
            );
        }
    }

    #[test]
    fn test_roundtrip_overlap_save() {
        // Test that breaker followed by combiner preserves data (minus edge effects)
        let (mut breaker, mut combiner) =
            generate_overlap_save_steps::<f32, 1024, 32, 16, 64, 16>();

        let reference_buffer: [f32; 1024] = rand::random();
        let input_buffer = BufferArray::new_with_value(reference_buffer.clone());

        let mut input_wrapper = DataWrapper::new_with_value(input_buffer);
        let mut input = [input_wrapper];

        let mut output_wrapper_1 = DataWrapper::new();

        let result_1 = breaker.run_cpu(&mut input, &mut output_wrapper_1);
        assert!(result_1.is_ok());

        let mut output_wrapper_2 = DataWrapper::new();
        let result_2 = combiner.run_cpu(&mut [output_wrapper_1], &mut output_wrapper_2);
        assert!(result_2.is_ok());

        assert!(output_wrapper_2.read().read() == &reference_buffer);
    }
}
