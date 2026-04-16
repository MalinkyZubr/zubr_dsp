use super::rodio_source::*;
use crate::pipeline::communication_layer::data_management::*;
use crate::pipeline::construction_layer::node_types::pipeline_step::PipelineStep;
use crate::pipeline::construction_layer::pipeline_traits::*;
use rodio::Sink as RodioSink;
use rodio::{
    ChannelCount, OutputStream, OutputStreamBuilder, Sample, SampleRate, Source as RodioSource,
};
use thread_priority::{set_current_thread_priority, ThreadPriority};

pub struct AudioSink<const BUFFER_SIZE: usize> {
    sink: Option<RodioSink>,
    sample_rate: SampleRate,
    channels: ChannelCount,
    detach_on_kill: bool,
}

impl<const BUFFER_SIZE: usize> AudioSink<BUFFER_SIZE> {
    pub fn new(channels: u16, sample_rate: u32, sink: RodioSink, detach_on_kill: bool) -> Self {
        let _ = set_current_thread_priority(ThreadPriority::Max);
        let sample_rate = SampleRate::from(sample_rate);
        let channels = ChannelCount::from(channels);

        Self {
            sink: Some(sink),
            sample_rate,
            channels,
            detach_on_kill,
        }
    }
}

impl<const BUFFER_SIZE: usize> PipelineStep<BufferArray<Sample, BUFFER_SIZE>, (), 1> 
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

impl<const BUFFER_SIZE: usize> Sink for AudioSink<BUFFER_SIZE> {}


// Alternative audio sink that works directly with f32 samples
pub struct DirectAudioSink<const BUFFER_SIZE: usize> {
    sink: Option<RodioSink>,
    sample_rate: SampleRate,
    channels: ChannelCount,
    detach_on_kill: bool,
}

impl<const BUFFER_SIZE: usize> DirectAudioSink<BUFFER_SIZE> {
    pub fn new(channels: u16, sample_rate: u32, sink: RodioSink, detach_on_kill: bool) -> Self {
        let _ = set_current_thread_priority(ThreadPriority::Max);
        let sample_rate = SampleRate::from(sample_rate);
        let channels = ChannelCount::from(channels);

        Self {
            sink: Some(sink),
            sample_rate,
            channels,
            detach_on_kill,
        }
    }
}

impl<const BUFFER_SIZE: usize> PipelineStep<BufferArray<f32, BUFFER_SIZE>, (), 1> 
    for DirectAudioSink<BUFFER_SIZE> 
{
    fn run_cpu(
        &mut self,
        input: &mut [DataWrapper<BufferArray<f32, BUFFER_SIZE>>; 1],
        _output: &mut DataWrapper<()>,
    ) -> Result<(), ()> {
        let input_data = input[0].read().read();
        let samples: Vec<Sample> = input_data.iter()
            .map(|&sample| Sample::from(sample))
            .collect();
        
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

impl<const BUFFER_SIZE: usize> Sink for DirectAudioSink<BUFFER_SIZE> {}