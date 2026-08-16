use std::ops::{AddAssign, DivAssign, MulAssign, RemAssign, SubAssign};
use log::warn;
use num::Complex;
use rustfft::{Fft, FftNum, FftPlanner};
use std::sync::Arc;
use num_traits::{cast, NumCast};
use crate::engine::data_plane::communication_layer::data_management::BufferArray;
use crate::engine::data_plane::structural::generic_node_operation::PipelineNodeOp;

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
impl<T: FftNum + Default, const BUFFER_SIZE: usize>
    PipelineNodeOp<BufferArray<Complex<T>, BUFFER_SIZE>, BufferArray<Complex<T>, BUFFER_SIZE>, 1>
    for FFT<T, BUFFER_SIZE>
{
    fn run_cpu(
        &mut self,
        input: &mut [BufferArray<Complex<T>, BUFFER_SIZE>; 1],
        output: &mut BufferArray<Complex<T>, BUFFER_SIZE>,
    ) -> Result<(), ()> {
        self.fft.process(input[0].read_mut());
        output.swap_pointers(&mut input[0]);
        Ok(())
    }
}

pub struct IFFT<T: FftNum, const BUFFER_SIZE: usize> {
    fft: Arc<dyn Fft<T>>,
}
impl<T: FftNum, const BUFFER_SIZE: usize> IFFT<T, BUFFER_SIZE> {
    pub fn new() -> Self {
        if BUFFER_SIZE < 2 {
            panic!("BUFFER_SIZE < 2");
        }
        if BUFFER_SIZE % 2 != 0 {
            warn!("fft_size should be even for FFT block to maximize efficiency");
        }

        let mut planner = FftPlanner::new();
        IFFT {
            fft: planner.plan_fft_inverse(BUFFER_SIZE),
        }
    }
}


impl<T: FftNum + Default + DivAssign + AddAssign + MulAssign + RemAssign + SubAssign + NumCast, const BUFFER_SIZE: usize>
    PipelineNodeOp<BufferArray<Complex<T>, BUFFER_SIZE>, BufferArray<Complex<T>, BUFFER_SIZE>, 1>
for IFFT<T, BUFFER_SIZE>
{
    fn run_cpu(
        &mut self,
        input: &mut [BufferArray<Complex<T>, BUFFER_SIZE>; 1],
        output: &mut BufferArray<Complex<T>, BUFFER_SIZE>,
    ) -> Result<(), ()> {
        self.fft.process(input[0].read_mut());
        output.swap_pointers(&mut input[0]);
        
        for value in output.read_mut().iter_mut() {
            *value /= cast::<usize, T>(BUFFER_SIZE).unwrap();
        }
        
        Ok(())
    }
}