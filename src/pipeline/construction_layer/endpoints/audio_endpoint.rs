use super::rodio_source::*;
use crate::pipeline::api::*;
use rodio::Sink as RodioSink;
use rodio::{
    ChannelCount, OutputStream, OutputStreamBuilder, Sample, SampleRate, Source as RodioSource,
};
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::errors::Error;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use thread_priority::{set_current_thread_priority, ThreadPriority};

pub struct AudioSink {
    sink: Option<RodioSink>,
    sample_rate: SampleRate,
    channels: ChannelCount,
    detach_on_kill: bool,
}
impl AudioSink {
    pub fn new(channels: u16, sample_rate: u32, sink: RodioSink, detach_on_kill: bool) -> Self {
        set_current_thread_priority(ThreadPriority::Max);
        // let stream_handle = OutputStreamBuilder::open_default_stream()
        //     .expect("open default audio stream");
        //
        // let sink = rodio::Sink::connect_new(&stream_handle.mixer());
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
impl PipelineStep<Vec<Sample>, ()> for AudioSink {
    fn run_SIDO(&mut self, input: Vec<Sample>) -> Result<ODFormat<()>, String> {
        let new_source = SourceObject::new(input, self.channels, self.sample_rate);

        match &mut self.sink {
            Some(sink) => {
                sink.append(new_source);
                Ok(ODFormat::Standard(()))
            }
            None => Err("Sink does not exist".to_string()),
        }
    }

    fn pause_behavior(&mut self) {
        match &mut self.sink {
            Some(sink) => sink.pause(),
            None => panic!("Cannot pause nonexistant"),
        }
    }
    fn start_behavior(&mut self) {
        match &mut self.sink {
            Some(sink) => sink.play(),
            None => panic!("Cannot start nonexistant"),
        }
    }
    fn kill_behavior(&mut self) {
        let flag = self.detach_on_kill.clone();
        let sink = self.sink.take();

        match sink {
            Some(sink) => {
                if flag {
                    sink.detach();
                } else {
                    sink.stop();
                    sink.empty();
                }
            }
            None => panic!("Cannot kill nonexistant"),
        }
    }
}
impl Sink for AudioSink {}

pub struct ToSamples {}
impl PipelineStep<Vec<f32>, Vec<Sample>> for ToSamples {
    fn run_SISO(&mut self, input: Vec<f32>) -> Result<ODFormat<Vec<Sample>>, String> {
        let output = input.iter().map(|sample| Sample::from(*sample)).collect();

        Ok(ODFormat::Standard(output))
    }
}
