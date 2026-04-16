use crate::pipeline::communication_layer::data_management::{BufferArray, DataWrapper};
use crate::pipeline::construction_layer::node_types::pipeline_step::PipelineStep;
use log::warn;
use num::Complex;
use rustfft::{Fft, FftNum, FftPlanner};
use std::sync::Arc;

pub struct FFT<T: FftNum, const BUFFER_SIZE: usize> {
    fft: Arc<dyn Fft<T>>,
}
impl<T: FftNum, const BUFFER_SIZE: usize> FFT<T, BUFFER_SIZE> {
    pub fn new() -> Self {
        if BUFFER_SIZE < 2 {
            panic!("BUFFER_SIZE < 2");
        }
        if BUFFER_SIZE % 2 != 0 {
            warn!("fft_size should be even for FFT block to maximize efficiency");
        }

        let mut planner = FftPlanner::new();
        FFT {
            fft: planner.plan_fft_forward(BUFFER_SIZE),
        }
    }
}
impl<T: FftNum + Default, const BUFFERSIZE: usize>
    PipelineStep<BufferArray<Complex<T>, BUFFERSIZE>, BufferArray<Complex<T>, BUFFERSIZE>, 1>
    for FFT<T, BUFFERSIZE>
{
    fn run_cpu(
        &mut self,
        input: &mut [DataWrapper<BufferArray<Complex<T>, BUFFERSIZE>>; 1],
        output: &mut DataWrapper<BufferArray<Complex<T>, BUFFERSIZE>>,
    ) -> Result<(), ()> {
        self.fft.process(input[0].read().read_mut());
        output.swap_st(&mut input[0]);
        Ok(())
    }
}

pub struct IFFT<T: FftNum> {
    fft: Arc<dyn Fft<T>>,
}
impl<T: FftNum> IFFT<T> {
    pub fn new(fft_size: usize) -> Self {
        if fft_size < 2 {
            panic!("fft_size must be greater than 2");
        }
        if fft_size % 2 != 0 {
            warn!("fft_size should be even for FFT block to maximize efficiency");
        }

        let mut planner = FftPlanner::new();
        IFFT {
            fft: planner.plan_fft_inverse(fft_size),
        }
    }
}
