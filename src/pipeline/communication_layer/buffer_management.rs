use std::mem;
use crate::pipeline::construction_layer::pipeline_traits::Sharable;
use bounded_spsc_queue::{Buffer, Consumer, Producer, make as spsc_make};
use std::mem::swap;


pub struct DataBuffer<T: Sharable, const BuffSize: usize> {
    buffer: [T; BuffSize]
}

impl<T: Sharable, const BSize: usize> DataBuffer<T, BSize> {
    pub fn new() -> Self {
        DataBuffer {
            buffer: [Default::default(); BSize]
        }
    }
    
    pub fn read(&mut self) -> &mut [T; BSize] {
        &mut self.buffer
    }
    
    pub fn move_to_array(&mut self, output_buffer: &mut [T; BSize]) {
        swap(&mut self.buffer, output_buffer);
    }
    
    pub fn move_from_array(&mut self, input_buffer: &mut [T; BSize]) {
        swap(&mut self.buffer, input_buffer);
    }
}
