use num::Complex;
use crate::engine::data_plane::communication_layer::data_management::BufferArray;
use crate::engine::data_plane::structural::generic_node_operation::PipelineNodeOp;
// xc (t)= Ac [1 + amn(t)] cos(2πfc t)


pub struct AMDemodulator<const BUFFER_SIZE: usize> {
    carrier_amplitude: f32,
    modulation_index: f32,
}
impl<const BUFFER_SIZE: usize> AMDemodulator<BUFFER_SIZE> {
    pub fn new(carrier_amplitude: f32, modulation_index: f32) -> Self {
        Self {
            carrier_amplitude,
            modulation_index,
        }
    }
}
impl<const BUFFER_SIZE: usize> PipelineNodeOp<BufferArray<Complex<f32>, BUFFER_SIZE>, BufferArray<Complex<f32>, BUFFER_SIZE>, 1> for AMDemodulator<BUFFER_SIZE> {
    fn run_cpu(&mut self, input: &mut [BufferArray<Complex<f32>, BUFFER_SIZE>; 1], output: &mut BufferArray<Complex<f32>, BUFFER_SIZE>) -> Result<(), ()> {
        input[0].swap_pointers(output);

        for output_sample in output.read_mut().iter_mut() {
            *output_sample = ((*output_sample / self.carrier_amplitude) - Complex::new(1.0, 0.0)) / self.modulation_index;
        }

        Ok(())
    }
}