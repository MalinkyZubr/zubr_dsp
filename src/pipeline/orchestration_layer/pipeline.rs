// deprecating the format adapters, switching to type adapters which is more essential
// format adapters were designed for a previous (invalid) understanding of how some DSP algorithms worked. They only create latency
// type adapters (primarily modulators) are much more important and baked directly into a proper DSP pipeline for radio transmission and reception

use std::time::Instant;
//use crate::frontend::curses::app::{App, AppBuilder};

pub struct ActivePipeline {
    start_time: Instant,
}
impl ActivePipeline {
    pub fn start(&mut self) {}
    pub fn pause(&mut self) {}
    pub fn kill(mut self) {}
    pub fn is_running(&self) -> bool {
        false
    }
}
