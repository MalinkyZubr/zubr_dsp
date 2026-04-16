use std::time::Instant;
use async_trait::async_trait;
use crate::pipeline::construction_layer::node_types::pipeline_step;
use crate::pipeline::construction_layer::pipeline_traits::Sharable;
use crate::pipeline::communication_layer::data_management::*;
use crate::pipeline::construction_layer::node_types::pipeline_step::PipelineStep;
use tokio::time::{sleep, Duration};

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
impl <T: Sharable, const BUFFER_SIZE: usize> PipelineStep<BufferArray<T, BUFFER_SIZE>, BufferArray<T, BUFFER_SIZE>, 1> for Throttle<BUFFER_SIZE> {
    async fn run_io(&mut self, _input: &mut [DataWrapper<BufferArray<T, BUFFER_SIZE>>; 1], _output: &mut DataWrapper<BufferArray<T, BUFFER_SIZE>>) -> Result<(), ()> {
        _input[0].swap_st(_output);
        if Instant::now() < self.target {
            sleep(self.target - Instant::now()).await;
        }
        self.target += self.delay;
        
        Ok(())
    }
}