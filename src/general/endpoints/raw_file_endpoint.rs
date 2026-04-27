use crate::engine::structural::pipeline_type_traits::SharableNum;
use std::fs::File;
use std::io::Write;
use crate::engine::communication_layer::data_management::{BufferArray, DataWrapper};
use crate::engine::structural::generic_node_operation::{PipelineSink, PipelineNodeOp};
use std::mem::size_of;
use log::{error, warn};
use num_traits::ToBytes;

pub struct RawFileSink<const BUFFER_SIZE: usize> {
    file_path: String,
    file: File,
    max_file_size: usize,
    current_file_size: usize,
}
impl<const BUFFER_SIZE: usize> RawFileSink<BUFFER_SIZE> {
    pub fn new(file_path: String, max_file_size_mb: usize) -> Self {
        if max_file_size_mb == 0 {
            warn!("Maximum file size is specified to 0 for file sink. This is dangerous! File size will grow infinitely as long as program runs!")
        }
        Self {
            file_path: file_path.clone(),
            file: File::create(file_path).unwrap(),
            max_file_size: max_file_size_mb * 1e6 as usize,
            current_file_size: 0,
        }
    }
}

impl<const BUFFER_SIZE: usize> PipelineNodeOp<BufferArray<u8, BUFFER_SIZE>, (), 1> for RawFileSink<BUFFER_SIZE> {
    fn run_cpu(&mut self, _input: &mut [DataWrapper<BufferArray<u8, BUFFER_SIZE>>; 1], _output: &mut DataWrapper<()>) -> Result<(), ()> {
        self.file.write_all(_input[0].read().read()).unwrap();
        self.current_file_size += _input[0].read().read().len();

        if self.current_file_size >= self.max_file_size && self.max_file_size != 0{
            error!("Reached maximum file size {}", self.max_file_size);
            Err(())
        }
        else {
            Ok(())
        }
    }
}