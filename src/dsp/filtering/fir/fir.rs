use crate::pipeline::communication_layer::data_management::{BufferArray, DataWrapper};
use crate::pipeline::construction_layer::node_types::pipeline_step::PipelineStep;
use crate::pipeline::construction_layer::pipeline_traits::Sharable;
use num::Complex;
use num_traits::{cast, Num, NumCast};
use rustfft::{Fft, FftNum, FftPlanner};
use std::sync::Arc;

pub struct FIRFilter<T: Sharable + Num + Copy + NumCast + FftNum, const TOTAL_FILTER_SIZE: usize> {
    coefficients: BufferArray<Complex<T>, TOTAL_FILTER_SIZE>,
    unpadded_filter_size: usize,
}

impl<T: Sharable + Num + Copy + NumCast + FftNum, const TOTAL_FILTER_SIZE: usize>
    FIRFilter<T, TOTAL_FILTER_SIZE>
{
    pub fn new<const UNPADDED_FILTER_SIZE: usize>(
        coefficients: BufferArray<Complex<T>, UNPADDED_FILTER_SIZE>,
    ) -> Self {
        assert!(TOTAL_FILTER_SIZE.is_power_of_two(), "FIR filter size must be a power of two for SIMD acceleration.\nFeeling freaky?\n(simd doesnt exist yet. Whoopsie poopsie)");
        assert!(UNPADDED_FILTER_SIZE <= TOTAL_FILTER_SIZE, "You cant supply a filter that is larger than memory allocation of the node. Wheres your FIR filter license!?\nDont have one?\nYou're going to FIR jail.\nFor FIR life.");
        let mut padded_coefficients: BufferArray<Complex<T>, TOTAL_FILTER_SIZE> =
            BufferArray::new();
        padded_coefficients
            .read_mut()[0..UNPADDED_FILTER_SIZE]
            .copy_from_slice(coefficients.read());

        let fft: Arc<dyn Fft<T>> = FftPlanner::new().plan_fft_forward(TOTAL_FILTER_SIZE);
        fft.process(padded_coefficients.read_mut());

        let mut reverse_index = TOTAL_FILTER_SIZE - UNPADDED_FILTER_SIZE;
        while reverse_index > 0 {
            padded_coefficients.set(
                TOTAL_FILTER_SIZE - reverse_index,
                Complex::new(cast(0).unwrap(), cast(0).unwrap()),
            );
            reverse_index -= 1;
        }

        let i_fft: Arc<dyn Fft<T>> = FftPlanner::new().plan_fft_inverse(TOTAL_FILTER_SIZE);
        i_fft.process(padded_coefficients.read_mut());

        Self {
            coefficients: padded_coefficients,
            unpadded_filter_size: UNPADDED_FILTER_SIZE,
        }
    }
    
    pub fn get_unpadded_filter_size(&self) -> usize {
        self.unpadded_filter_size
    }
}

impl<T: Sharable + Num + Copy + NumCast + FftNum, const TOTAL_FILTER_SIZE: usize>
    PipelineStep<
        BufferArray<Complex<T>, TOTAL_FILTER_SIZE>,
        BufferArray<Complex<T>, TOTAL_FILTER_SIZE>,
        1,
    > for FIRFilter<T, TOTAL_FILTER_SIZE>
{
    fn run_cpu(&mut self, _input: &mut [DataWrapper<BufferArray<Complex<T>, TOTAL_FILTER_SIZE>>; 1], _output: &mut DataWrapper<BufferArray<Complex<T>, TOTAL_FILTER_SIZE>>) -> Result<(), ()> {
        for (coefficient, input_bin) in self.coefficients.read().iter().zip(_input[0].read().read_mut()) {
            *input_bin = *coefficient * *input_bin;
        }
        _output.swap_st(&mut _input[0]);
        Ok( ())
    }
}
