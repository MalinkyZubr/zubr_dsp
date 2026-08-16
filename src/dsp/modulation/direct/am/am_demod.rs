use log::error;
use crate::dsp::modulation::direct::am::am_mod::AMModulator;
use crate::engine::data_plane::communication_layer::data_management::BufferArray;
use crate::engine::data_plane::structural::generic_node_operation::PipelineNodeOp;

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
    fn run_cpu(&mut self, _input: &mut [BufferArray<f32, BUFFER_SIZE>; 1], _output: &mut BufferArray<f32, BUFFER_SIZE>) -> Result<(), ()> {
        _input[0].swap_pointers(_output);
        
        for sample in _output.read_mut().iter_mut() {
            //error!("{:?}", sample);
            *sample /= self.carrier_amplitude;
            *sample -= 1.0;
            *sample /= self.modulation_index;
        }

        Ok(())
    }
}