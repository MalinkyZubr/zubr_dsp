use num::Complex;
use crate::dsp::sampling::sampling_formulas::index_from_frequeny;
use crate::pipeline::interfaces::PipelineStep;
use crate::pipeline::interfaces::ODFormat;

pub struct PowerCalculatorTD {}
impl PipelineStep<Vec<f32>, f32> for PowerCalculatorTD {
    fn run_SISO(&mut self, input: Vec<f32>) -> Result<ODFormat<f32>, String> {
        let input_size = input.len();
        let sum: f32 = input.iter()
            .map(|x| x.abs().powf(2.0))
            .sum();
        let power = sum / ((2.0 * input_size as f32) + 1.0);
        
        Ok(ODFormat::Standard(power))
    }
}

pub struct PowerCalculatorFD {} // parseval's relation is a wonderful thing
impl PipelineStep<Vec<Complex<f32>>, f32> for PowerCalculatorFD {
    fn run_SISO(&mut self, input: Vec<Complex<f32>>) -> Result<ODFormat<f32>, String> {
        let input_size = input.len();
        let sum: f32 = input.iter()
            .map(|x| x.norm().powf(2.0))
            .sum();
        let power = sum / ((2.0 * input_size as f32) + 1.0);

        Ok(ODFormat::Standard(power))
    }
}

pub struct PowerAtFrequency {
    frequency_domain_index: usize
}
impl PowerAtFrequency {
    pub fn new(frequency: f32, sample_rate: f32, buffer_size: usize) -> Self {
        Self {
            frequency_domain_index: index_from_frequeny(frequency, sample_rate, buffer_size)
        }
    }
}
impl PipelineStep<Vec<Complex<f32>>, f32> for PowerAtFrequency {
    fn run_SISO(&mut self, input: Vec<Complex<f32>>) -> Result<ODFormat<f32>, String> {
        Ok(ODFormat::Standard(input[self.frequency_domain_index].norm().powf(2.0)))
    }
}

pub fn calculate_gain_watt(input_power: f32, output_power: f32) -> f32 {
    output_power / input_power
}

pub fn calculate_gain_db(input_power: f32, output_power: f32) -> f32 {
    10.0 * (output_power / input_power).log10()
}