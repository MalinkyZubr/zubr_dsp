use rustfft::FftPlanner;
use crate::dsp::fft::fftshift::fft_shift;
use num::{Complex, Num};

pub struct ImpulseResponse<T: Num + Sharable, const NumSamples: usize> {
    pub reversed_impulse_response: [],
}
impl ImpulseResponse {
    pub fn new_configured(mut impulse_response: Vec<f32>) -> Self {
        impulse_response.reverse();
        let ir_len = impulse_response.len();

        ImpulseResponse {
            reversed_impulse_response: impulse_response,
        }
    }
    pub fn new_from_function<F>(func: F, sample_start_n: i64, window_size: usize) -> Self
    where F: Fn(i64) -> f32 {
        let mut impulse_response = Vec::with_capacity(window_size);
        for index in sample_start_n..sample_start_n + window_size as i64 {
            impulse_response.push(func(index));
        }

        ImpulseResponse {
            reversed_impulse_response: impulse_response,
        }
    }

    pub fn transfer_function(mut self, padding: usize) -> TransferFunction {
        self.reversed_impulse_response.reverse();
        let mut impulse_response = self.reversed_impulse_response;

        if padding > 0 {
            impulse_response.extend(vec![0.0; padding]);
        }

        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(impulse_response.len());
        let mut complex_impulse_response: Vec<Complex<f32>> = impulse_response
            .iter_mut()
            .map(|x| Complex::new(*x, 0.0))
            .collect();

        fft.process(&mut complex_impulse_response);
            
        TransferFunction::new_configured(
            complex_impulse_response,
        )
    }

    pub fn normalize_at_frequency(ir: Self, frequency: f32, sample_rate: f32) -> Self {
        let tf = ir.transfer_function(0);
        let index = (frequency * tf.len() as f32 / sample_rate) as usize;
        let select_frequency_magnitude = tf.transfer_function[index].norm();
        
        let mut ir = tf.impulse_response(0);

        ir.reversed_impulse_response = ir.reversed_impulse_response.iter()
            .map(|x| x / select_frequency_magnitude)
            .collect();

        ir
    }
    pub fn normalize_to_max(ir: Self) -> Self {
        Self::normalize_at_frequency(ir, 0.0, 1.0)
    }
    pub fn normalize_to_sum(mut ir: Self) -> Self {
        let sum: f32 = ir.reversed_impulse_response.iter().sum();
        ir.reversed_impulse_response = ir.reversed_impulse_response.iter()
            .map(|x| x / sum)
            .collect();
        ir
    }
    
    pub fn len(&self) -> usize {
        return self.reversed_impulse_response.len();
    }
}

pub struct TransferFunction {
    pub transfer_function: Vec<Complex<f32>>
}
impl TransferFunction {
    pub fn new_configured(transfer_function: Vec<Complex<f32>>) -> Self {
        TransferFunction {
            transfer_function,
        }
    }
    pub fn new_from_function<F>(func: F, sample_start_n: i64, window_size: usize) -> Self
    where F: Fn(i64) -> Complex<f32> {
        let mut transfer_function = Vec::with_capacity(window_size);
        for index in sample_start_n..sample_start_n + window_size as i64 {
            transfer_function.push(func(index));
        }
        
        TransferFunction {
            transfer_function
        }
    }

    pub fn impulse_response(mut self, dropped: usize) -> ImpulseResponse {
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_inverse(self.transfer_function.len());
        
        fft.process(&mut self.transfer_function);
        
        self.transfer_function.truncate(self.transfer_function.len() - dropped);

        let impulse_response: Vec<f32> = self.transfer_function
            .iter_mut()
            .map(|x| x.re)
            .collect();
        
        ImpulseResponse::new_configured(impulse_response)
    }

    pub fn len(&self) -> usize {
        self.transfer_function.len()
    }
}