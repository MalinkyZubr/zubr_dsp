use std::fs::File;
use std::io::{Read, Seek};
use crate::engine::communication_layer::data_management::{BufferArray, DataWrapper};
use crate::engine::structural::generic_node_operation::PipelineNodeOp;
pub struct RawFileSource<const BUFFER_SIZE: usize> {
    file_path: String,
    file: File,
}
impl<const BUFFER_SIZE: usize> RawFileSource<BUFFER_SIZE> {
    pub fn new(file_path: String) -> Self {
        Self {
            file_path: file_path.clone(),
            file: File::create(file_path).unwrap(),
        }
    }
}


impl<const BUFFER_SIZE: usize> PipelineNodeOp<(), BufferArray<u8, BUFFER_SIZE>, 0> for RawFileSource<BUFFER_SIZE>{
    fn run_cpu(&mut self, _input: &mut [DataWrapper<()>; 0], _output: &mut DataWrapper<BufferArray<u8, BUFFER_SIZE>>) -> Result<(), ()> {
        match self.file.read(_output.read().read_mut()) {
            Ok(num_read) => {
                if num_read < BUFFER_SIZE {
                    self.file.seek(std::io::SeekFrom::Start(0)).unwrap();
                }
            }
            Err(_) => {
                return Err(());
            }
        }

        Ok(())
    }
}
