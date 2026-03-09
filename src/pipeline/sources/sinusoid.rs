use crate::pipeline::interfaces::ODFormat;
use crate::pipeline::interfaces::PipelineStep;
use std::f32::consts::PI;

pub struct SinusoidalSource<T> {
    frequency: f32,
    sampling_frequency: f32,
    buff_size: usize,
    phase: f32,

    previous_time: f32,
}
impl SinusoidalSource {
    pub fn new(frequency: f32, sampling_frequency: f32, phase: f32, buff_size: usize) -> Self {
        Self {
            frequency,
            sampling_frequency,
            buff_size,
            phase,
            previous_time: 0.0,
        }
    }
}

impl SinusoidalSource {
    fn increment_time(&mut self, time: f32) -> f32 {
        let time = ((time + 2.0 * PI * self.frequency / self.sampling_frequency) + self.phase)
            % (2.0 * PI);
        time
    }
}
impl PipelineStep<(), Vec<f32>> for SinusoidalSource {
    fn run_DISO(&mut self) -> Result<ODFormat<Vec<f32>>, String> {
        let mut buffer = Vec::with_capacity(self.buff_size);

        let mut time = self.increment_time(self.previous_time);

        for _ in 0..self.buff_size {
            buffer.push(time.cos());
            time = self.increment_time(time);
        }

        self.previous_time = time;

        Ok(ODFormat::Standard(buffer))
    }
}
