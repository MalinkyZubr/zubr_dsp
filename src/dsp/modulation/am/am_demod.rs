use log::error;
use crate::dsp::modulation::am::am_mod::AMModulator;
use crate::engine::communication_layer::data_management::{BufferArray, DataWrapper};
use crate::engine::structural::pipeline_type_traits::*;
use crate::engine::structural::generic_node_operation::*;


pub struct AMDemodulator<const BUFFER_SIZE: usize> {
    carrier_amplitude: f32,
    modulation_index: f32,
}


impl<const BUFFER_SIZE: usize> AMDemodulator<BUFFER_SIZE> {
    pub fn new(carrier_amplitude: f32, modulation_index: f32) -> Self {
        AMDemodulator { carrier_amplitude, modulation_index }
    }
}


impl<const BUFFER_SIZE: usize> PipelineNodeOp<BufferArray<f32, BUFFER_SIZE>, BufferArray<f32, BUFFER_SIZE>, 1> for AMDemodulator<BUFFER_SIZE> {
    // EXPECTS THE ENVELOPE OF THE MODULATED SIGNAL! THIS ONLY UNDOES OFFSETS
    fn run_cpu(&mut self, _input: &mut [DataWrapper<BufferArray<f32, BUFFER_SIZE>>; 1], _output: &mut DataWrapper<BufferArray<f32, BUFFER_SIZE>>) -> Result<(), ()> {
        _input[0].swap_st(_output);
        
        for sample in _output.read().read_mut().iter_mut() {
            //error!("{:?}", sample);
            *sample /= self.carrier_amplitude;
            *sample -= 1.0;
            *sample /= self.modulation_index;
        }

        Ok(())
    }
}