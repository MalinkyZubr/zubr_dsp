use super::rodio_source::*;
use crate::engine::communication_layer::data_management::*;
use crate::engine::structural::generic_node_operation::PipelineNodeOp;
use crate::engine::structural::pipeline_type_traits::*;
use rodio::Sink as RodioSink;
use rodio::{
    ChannelCount, OutputStream, OutputStreamBuilder, Sample, SampleRate, Source as RodioSource,
};
use thread_priority::{set_current_thread_priority, ThreadPriority};

pub struct AudioSink<const BUFFER_SIZE: usize> {
    sink: Option<RodioSink>,
    sample_rate: SampleRate,
    channels: ChannelCount,
}

impl<const BUFFER_SIZE: usize> AudioSink<BUFFER_SIZE> {
    pub fn new(channels: u16, sample_rate: u32, sink: RodioSink) -> Self {
        let _ = set_current_thread_priority(ThreadPriority::Max);

        Self {
            sink: Some(sink),
            sample_rate,
            channels,
        }
    }
}

impl<const BUFFER_SIZE: usize> PipelineNodeOp<BufferArray<Sample, BUFFER_SIZE>, (), 1> 
    for AudioSink<BUFFER_SIZE> 
{
    fn run_cpu(
        &mut self,
        input: &mut [DataWrapper<BufferArray<Sample, BUFFER_SIZE>>; 1],
        _output: &mut DataWrapper<()>,
    ) -> Result<(), ()> {
        let samples: Vec<Sample> = input[0].read().read().to_vec();
        let new_source = SourceObject::new(samples, self.channels, self.sample_rate);

        match &mut self.sink {
            Some(sink) => {
                sink.append(new_source);
                Ok(())
            }
            None => Err(()),
        }
    }
}