use std::ops::{AddAssign, DivAssign, MulAssign, RemAssign, SubAssign};
use crate::engine::data_plane::::data_management::{BufferArray, DataWrapper};
use crate::engine::data_plane::::generic_node_operation::PipelineNodeOp;
use log::warn;
use num::Complex;
use rustfft::{Fft, FftNum, FftPlanner};
use std::sync::Arc;
use num_traits::{cast, NumCast};

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
        input: &mut [DataWrapper<BufferArray<Complex<T>, BUFFER_SIZE>>; 1],
        output: &mut DataWrapper<BufferArray<Complex<T>, BUFFER_SIZE>>,
    ) -> Result<(), ()> {
        self.fft.process(input[0].read().read_mut());
        output.swap_st(&mut input[0]);
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
        input: &mut [DataWrapper<BufferArray<Complex<T>, BUFFER_SIZE>>; 1],
        output: &mut DataWrapper<BufferArray<Complex<T>, BUFFER_SIZE>>,
    ) -> Result<(), ()> {
        self.fft.process(input[0].read().read_mut());
        output.swap_st(&mut input[0]);
        
        for value in output.read().read_mut().iter_mut() {
            *value /= cast::<usize, T>(BUFFER_SIZE).unwrap();
        }
        
        Ok(())
    }
}