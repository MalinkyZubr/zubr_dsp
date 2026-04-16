use crate::pipeline::communication_layer::data_management::{BufferArray, DataWrapper};
use crate::pipeline::construction_layer::pipeline_traits::*;
use crate::pipeline::construction_layer::node_types::pipeline_step::*;

// xc (t)= Ac [1 + amn(t)] cos(2πfc t)


pub struct AMModulator<const BUFFER_SIZE: usize> {
    carrier_amplitude: f32,
    carrier_frequency: f32,
    modulation_index: f32,
    sample_period: f32,
    phase_accumulator: f32,
    phase_increment: f32,
}
impl<const BUFFER_SIZE: usize> AMModulator<BUFFER_SIZE> {
    pub fn new(carrier_amplitude: f32, carrier_frequency: f32, modulation_index: f32, sample_frequency: f32) -> Self {
        let sample_period = 1f32 / sample_frequency;
        Self {
            carrier_amplitude,
            carrier_frequency,
            modulation_index,
            sample_period,
            phase_accumulator: 0f32,
            phase_increment: 2f32 * std::f32::consts::PI * carrier_frequency * sample_period,
        }
    }
}
impl<const BUFFER_SIZE: usize> PipelineStep<BufferArray<f32, BUFFER_SIZE>, BufferArray<f32, BUFFER_SIZE>, 1> for AMModulator<BUFFER_SIZE> {
    fn run_cpu(&mut self, _input: &mut [DataWrapper<BufferArray<f32, BUFFER_SIZE>>; 1], _output: &mut DataWrapper<BufferArray<f32, BUFFER_SIZE>>) -> Result<(), ()> {
        for (input_sample, output_sample) in _input[0].read().read_mut().iter().zip(_output.read().read_mut()) {
            *output_sample = self.carrier_amplitude * 
                (1f32 + self.modulation_index * input_sample)
                * f32::cos(self.phase_accumulator);
            self.phase_accumulator = (self.phase_accumulator + self.phase_increment).rem_euclid(2f32 * std::f32::consts::PI);
        }
        
        Ok(())
    }
}