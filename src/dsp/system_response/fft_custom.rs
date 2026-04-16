use num::Complex;
use std::f32::{consts::{PI}};
use std::usize::MAX;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use crate::general::parallel_computation::{self, *};
use rayon::{ThreadPool, ThreadPoolBuilder};
use crate::pipeline::api::ReceiveType;
use crate::pipeline::api::*;


// static twiddle computation is less time efficient, apparently. Cache really makes a difference
pub struct FFTBitReversal { // log_2(n) levels to the n sized fft for radix 2, nlogn total space in the vector
    bit_reversal_mapping: Vec<usize>,
    fft_size: usize,
    twiddle_factors: Vec<Complex<f32>>,
    index_bits_needed: usize,
    strides: Vec<usize>,
    is_ifft: bool
}

impl FFTBitReversal {
    pub fn new(buffer_size: usize, is_ifft: bool) -> Self {
        let index_bits_needed = (buffer_size as f64).log2() as usize;
        
        FFTBitReversal { 
            bit_reversal_mapping: Self::generate_bit_reversal_mapping(buffer_size, index_bits_needed),
            fft_size: buffer_size,
            twiddle_factors: Self::compute_twiddle_factors(buffer_size),
            index_bits_needed,
            is_ifft,
            strides: Self::compute_strides(buffer_size)
        }
    }

    fn compute_strides(buffer_size: usize) -> Vec<usize> {
        let mut strides: Vec<usize> = vec![0; buffer_size + 1];

        let mut butterfly_size = 2;

        while butterfly_size <= buffer_size {
            strides[butterfly_size] = buffer_size / (butterfly_size / 2);
            butterfly_size *= 2;
        }

        return strides;
    }

    fn compute_twiddle_factors(buffer_size: usize) -> Vec<Complex<f32>> {
        let mut twiddles: Vec<Complex<f32>> = Vec::with_capacity(buffer_size / 2);

        for index in 0..(buffer_size / 2) {
            let real_comp = (-2.0 * PI * index as f32 / buffer_size as f32).cos();
            let imag_comp = (-2.0 * PI * index as f32 / buffer_size as f32).sin();

            twiddles.push(Complex::new(real_comp, imag_comp));
        };

        return twiddles;
    }

    pub fn get_bit_reversal(value: usize, index_bits_needed: usize) -> usize { // no need for max efficiency since this only happens at the very beginning
        let string_size: usize = index_bits_needed;
        let mut reversed: usize = 0;

        for right_side_index in 0..(string_size / 2) {
            let left_side_index = string_size - 1 - right_side_index;
            let left_side_value = ((value & (1 << left_side_index)) > 0) as usize;
            let right_side_value = ((value & (1 << right_side_index)) > 0) as usize;

            reversed |= left_side_value << right_side_index;
            reversed |= right_side_value << left_side_index;
        }

        if string_size % 2 == 1 {
            reversed |= value & (1 << (string_size / 2));
        }

        return reversed;
    }

    fn generate_bit_reversal_mapping(buffer_size: usize, index_bits_needed: usize) -> Vec<usize> {
        let mut reversal_map: Vec<usize> = vec![0; buffer_size];

        for start_index in 0..buffer_size {
            if reversal_map[start_index] != usize::MAX {
                let reversed_index = FFTBitReversal::get_bit_reversal(start_index, index_bits_needed);

                if reversed_index != start_index {
                    reversal_map[start_index] = reversed_index;
                    reversal_map[reversed_index] = usize::MAX;
                }
                else {
                    reversal_map[start_index] = usize::MAX;
                }
            }
        }

        return reversal_map;
    }

    fn bit_reversal_in_place(&self, buffer: &mut [Complex<f32>]) {
        for first_index in 0..buffer.len() {
            match self.bit_reversal_mapping.get(first_index) {
                None => {},
                Some(second_index) => {
                    if *second_index != usize::MAX {
                        let temp = buffer[first_index];
                        buffer[first_index] = buffer[*second_index];
                        buffer[*second_index] = temp;
                    }
                }
            }
        }
    }

    fn get_proper_twiddle_factor(&self, compute_index: usize, stride: usize) -> Complex<f32> {
        return self.twiddle_factors[compute_index * (stride / 2)];
    }

    fn compute_single_butterfly(&self, butterfly_size: usize, butterfly_index: usize, buffer: &mut[Complex<f32>], stride: usize) {
        //println!("TWIDDLES: {:?}, STRIDE: {}", &self.twiddle_factors, stride);
        let half_size = (butterfly_size / 2);
        for compute_index in 0..half_size {
            let true_index = butterfly_index + compute_index;
            let true_offset_index = true_index + half_size;

            let p = buffer[true_index];
            let q = buffer[true_offset_index] * 
                self.get_proper_twiddle_factor(compute_index, stride);
            
            buffer[true_index] = p + q;
            buffer[true_offset_index] = p - q;
        }
    }

    fn compute_fft(&self, buffer: &mut [Complex<f32>]) { // iterative version
        self.bit_reversal_in_place(buffer);

        for fft_stage in 1..self.index_bits_needed + 1 {
            let butterfly_size = 1 << fft_stage;
            let mut butterfly_index = 0;

            let stride = self.strides[butterfly_size];

            while butterfly_index < self.fft_size {
                self.compute_single_butterfly(butterfly_size, butterfly_index, buffer, stride);
                
                butterfly_index += butterfly_size;
            }
        }
    }

    pub fn fft(&mut self, mut buffer: Vec<Complex<f32>>) -> Vec<Complex<f32>> {
        self.compute_fft(&mut buffer);
        return buffer
    }

    pub fn ifft(&mut self, mut buffer: Vec<Complex<f32>>) -> Vec<Complex<f32>> {
        for value in buffer.iter_mut() {
            *value = value.conj();
        }

        buffer = self.fft(buffer);

        let length = buffer.len();
        for value in buffer.iter_mut() {
            *value = *value / length as f32;
        }

        return buffer;
    }
}


impl PipelineStep<Vec<Complex<f32>>, Vec<Complex<f32>>> for FFTBitReversal {
    fn run_SISO(&mut self, input: Vec<Complex<f32>>) -> Result<ODFormat<Vec<Complex<f32>>>, String> {
        if self.is_ifft {
            Ok(ODFormat::Standard(self.ifft(input)))
        }
        else {
            Ok(ODFormat::Standard(self.fft(input)))
        }
    }
}