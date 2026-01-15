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
use crate::pipeline::orchestration_layer::pipeline_thread_orchestrator::PipelineThreadOrchestrator;
use crate::pipeline::orchestration_layer::all_buffer_ws::StaticThreadPoolTopographical;
//use crate::frontend::curses::app::{App, AppBuilder};


pub type ConstructionQueue = SegQueue<Box<dyn CollectibleThread>>;


#[derive(Clone)]
pub struct PipelineParameters {
    pub retries: usize,
    pub timeout: u64,
    pub max_infrastructure_errors: usize,
    pub max_compute_errors: usize,
    pub unchanged_state_time: u64,
    pub backpressure_val: usize,
}
impl PipelineParameters {
    pub fn new(
        retries: usize,
        timeout: u64,
        backpressure_val: usize,
        max_infrastructure_errors: usize,
        max_compute_errors: usize,
        unchanged_state_time: u64,
    ) -> PipelineParameters {
        Self {
            retries,
            timeout,
            backpressure_val,
            max_compute_errors,
            max_infrastructure_errors,
            unchanged_state_time,
        }
    }
}


pub struct ConstructingPipeline {
    nodes: ConstructionQueue,
    parameters: PipelineParameters,
}
impl ConstructingPipeline {
    pub fn new(
        retries: usize,
        timeout: u64,
        backpressure_val: usize,
        max_infrastructure_errors: usize,
        max_compute_errors: usize,
        unchanged_state_time: u64,
    ) -> Self {
        let parameters = PipelineParameters::new(
            retries,
            timeout,
            backpressure_val,
            max_infrastructure_errors,
            max_compute_errors,
            unchanged_state_time,
        );
        Self {
            nodes: SegQueue::new(),
            parameters,
        }
    }
    pub fn get_cloned_parameters(&self) -> PipelineParameters {
        self.parameters.clone()
    }
    pub fn get_nodes(&self) -> ConstructionQueue {
        self.nodes.clone()
    }
    pub fn finish_pipeline(mut self, num_threads: usize) -> ActivePipeline {
        let static_topological_threadpool = PipelineThreadOrchestrator::new(Vec::from(self.nodes), num_threads);

        ActivePipeline {
            thread_pool: static_topological_threadpool,
            parameters: self.parameters,
            start_time: Instant::now(),
        }
    }
}

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
