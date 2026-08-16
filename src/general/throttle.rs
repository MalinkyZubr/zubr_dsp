use std::mem;
use crate::engine::data_plane::structural::pipeline_type_traits::Sharable;
use std::time::Instant;
use async_trait::async_trait;
use tokio::time::{sleep, Duration};
use crate::engine::data_plane::communication_layer::data_management::BufferArray;
use crate::engine::data_plane::structural::generic_node_operation::PipelineNodeOp;

pub struct Throttle<const BUFFER_SIZE: usize> {
    delay: Duration,
    target: Instant,
}


impl<const BUFFER_SIZE: usize> Throttle<BUFFER_SIZE> {
    pub fn new(sample_rate: f32) -> Self {
        let delay = Duration::from_secs_f32((1f32 / sample_rate) * BUFFER_SIZE as f32);
        Self { 
            delay, 
            target: Instant::now() + delay
        }
    }
}


#[async_trait]
impl <T: Sharable, const BUFFER_SIZE: usize> PipelineNodeOp<BufferArray<T, BUFFER_SIZE>, BufferArray<T, BUFFER_SIZE>, 1> for Throttle<BUFFER_SIZE> {
    async fn run_io(&mut self, _input: &mut [BufferArray<T, BUFFER_SIZE>; 1], _output: &mut BufferArray<T, BUFFER_SIZE>) -> Result<(), ()> {
        mem::swap(&mut _input[0], _output);
        if Instant::now() < self.target {
            sleep(self.target - Instant::now()).await;
        }
        self.target += self.delay;
        
        Ok(())
    }
}