// deprecating the format adapters, switching to type adapters which is more essential
// format adapters were designed for a previous (invalid) understanding of how some DSP algorithms worked. They only create latency
// type adapters (primarily modulators) are much more important and baked directly into a proper DSP pipeline for radio transmission and reception

use crate::pipeline::orchestration_layer::logging::log_message;
use crate::pipeline::orchestration_layer::logging::Level;
use crossbeam_queue::SegQueue;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::mpmc::RecvTimeoutError;
use std::sync::mpsc;
use std::sync::Arc;
//use super::dummy::{dummy_thread_function, DummyManager, DummyRunner};
use std::thread::{self, JoinHandle};
use std::time::Instant;
use crate::pipeline::construction_layer::pipeline_node::CollectibleThread;
//use crate::frontend::curses::app::{App, AppBuilder};


pub struct ActivePipeline {
    thread_pool: PipelineThreadOrchestrator,
    parameters: PipelineParameters,
    start_time: Instant,
}
impl ActivePipeline {
    pub fn start(&mut self) {
        log_message(
            format!("Starting active pipeline net size: {} nodes", self.thread_pool.get_num_nodes()),
            Level::Debug,
        );
        self.thread_pool.start_pipeline()
    }
    pub fn pause(&mut self) {
        log_message(
            format!("Stopping active pipeline net size: {} nodes", self.thread_pool.get_num_nodes()),
            Level::Debug,
        );
    }
    pub fn kill(mut self) {
        log_message(
            format!("Killing active pipeline net size: {} nodes", self.thread_pool.get_num_nodes()),
            Level::Debug,
        );
        self.thread_pool.stop_pipeline();
    }
    pub fn get_thread_diagnostics(&self) -> Vec<ThreadDiagnostic> {
        let mut diagnostics = Vec::with_capacity(self.nodes.len());

        for thread in self.nodes.iter() {
            diagnostics.push(ThreadDiagnostic::new(thread));
        }

        diagnostics
    }
    pub fn is_running(&self) -> bool {
        self.state_passer.state.load(Ordering::Acquire) == ThreadStateSpace::RUNNING as u8
    }
}
