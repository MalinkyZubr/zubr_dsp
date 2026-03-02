use rodio::Source as RodioSource;
use rodio::{ChannelCount, Sample, SampleRate};
use std::time::Duration;

pub struct SourceObject {
    samples: Vec<Sample>,
    index: usize,
    num_channels: ChannelCount,
    sample_rate: SampleRate,
}
impl SourceObject {
    pub fn new(
        mut samples: Vec<Sample>,
        num_channels: ChannelCount,
        sample_rate: SampleRate,
    ) -> Self {
        Self {
            samples,
            num_channels,
            sample_rate,
            index: 0,
        }
    }
}

impl Iterator for SourceObject {
    type Item = Sample;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.samples.len() {
            let s = self.samples[self.index];
            self.index += 1;
            Some(s)
        } else {
            None
        }
    }
}

impl RodioSource for SourceObject {
    fn current_span_len(&self) -> Option<usize> {
        Some(self.samples.len().saturating_sub(self.index))
    }
    fn channels(&self) -> ChannelCount {
        self.num_channels
    }
    fn sample_rate(&self) -> SampleRate {
        self.sample_rate
    }
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}
