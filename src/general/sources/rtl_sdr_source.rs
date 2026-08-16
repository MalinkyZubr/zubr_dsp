use std::mem;
use async_std::stream::StreamExt;
use async_trait::async_trait;
use crate::engine::data_plane::communication_layer::data_management::BufferArray;
use crate::engine::data_plane::structural::generic_node_operation::{PipelineNodeOp, PipelineSource};
use num::Complex;
use desperado::{IqAsyncSource};


pub struct RtlSdrSource<const BUFFER_SIZE: usize> {
    sdr_source: IqAsyncSource,
}
impl<const BUFFER_SIZE: usize> RtlSdrSource<BUFFER_SIZE> {
    pub async fn new(device_index: usize, sample_rate: u32, center_frequency: u32, gain: Option<i32>) -> Self {
        let source = IqAsyncSource::from_rtlsdr(device_index, center_frequency, sample_rate, gain).await.unwrap();
        Self {
            sdr_source: source
        }
    }
}


#[async_trait]
impl<const BUFFER_SIZE: usize> PipelineNodeOp<(), BufferArray<Complex<f32>, BUFFER_SIZE>, 0> for RtlSdrSource<BUFFER_SIZE> {
    async fn run_io(&mut self, _input: &mut [(); 0], _output: &mut BufferArray<Complex<f32>, BUFFER_SIZE>) -> Result<(), ()> {
        let samples: desperado::error::Result<Vec<Complex<f32>>> = self.sdr_source.next().await.unwrap();

        match samples {
            Ok(mut samples) => {
                for (input_sample, output_sample) in samples.iter_mut().zip(_output.read_mut().iter_mut()) {
                    mem::swap(input_sample, output_sample);
                }
                Ok(())
            }
            Err(e) => Err(())
        }
    }
}
