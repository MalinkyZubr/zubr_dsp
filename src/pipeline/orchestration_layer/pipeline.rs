// deprecating the format adapters, switching to type adapters which is more essential
// format adapters were designed for a previous (invalid) understanding of how some DSP algorithms worked. They only create latency
// type adapters (primarily modulators) are much more important and baked directly into a proper DSP pipeline for radio transmission and reception

use crate::pipeline::orchestration_layer::logging::log_message;
use crate::pipeline::orchestration_layer::logging::Level;
use crate::pipeline::threading_layer::pipeline_thread::PipelineThread;
use crate::pipeline::threading_layer::thread_state_space::ThreadStateSpace;
use crossbeam_queue::SegQueue;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::mpmc::RecvTimeoutError;
use std::sync::mpsc;
use std::sync::Arc;
//use super::dummy::{dummy_thread_function, DummyManager, DummyRunner};
use std::thread::{self, JoinHandle};
use std::time::Instant;
//use crate::frontend::curses::app::{App, AppBuilder};

pub type ConstructionQueue = Arc<SegQueue<PipelineThread>>;

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

pub struct CommandStatePasser {
    state: Arc<AtomicU8>,
    aggregate_receiver: Option<mpsc::Receiver<ThreadStateSpace>>,
    copyable_sender: mpsc::Sender<ThreadStateSpace>,
    manager_thread_handle: Option<JoinHandle<()>>,
    timeout: u64,
}
impl CommandStatePasser {
    pub fn new(timeout: u64) -> Self {
        let (copyable_sender, aggregate_receiver) = mpsc::channel();
        Self {
            state: Arc::new(AtomicU8::new(1)),
            copyable_sender,
            aggregate_receiver: Some(aggregate_receiver),
            manager_thread_handle: None,
            timeout,
        }
    }
    pub fn extract_for_node(&self) -> (Arc<AtomicU8>, mpsc::Sender<ThreadStateSpace>) {
        (self.state.clone(), self.copyable_sender.clone())
    }
    pub fn broadcast_requested_state(&mut self, requested_state: ThreadStateSpace) {
        self.state.store(requested_state as u8, Ordering::Release);
    }
    fn broadcast_requested_state_stat(
        requested_state: ThreadStateSpace,
        state_broadcaster: &mut Arc<AtomicU8>,
    ) {
        state_broadcaster.store(requested_state as u8, Ordering::Release);
    }
    pub fn spawn_manager_thread(&mut self) {
        let receiver = self.aggregate_receiver.take();
        let state = self.state.clone();
        let timeout = self.timeout.clone();

        match receiver {
            Some(receiver) => {
                self.manager_thread_handle = Some(thread::spawn(move || {
                    Self::internal_state_management_thread(receiver, state, timeout)
                }))
            }
            None => panic!(
                "Cannot start pipeline manager thread without aggregate receiver instantiated"
            ),
        }
    }
    pub fn join(mut self) {
        log_message(format!("Joining pipeline management thread"), Level::Info);

        match self.manager_thread_handle {
            None => panic!("Cannot join the manager thread handle before it is spawned"),
            Some(handle) => handle.join().unwrap(),
        }

        log_message(
            format!("Pipeline management thread successfully joined"),
            Level::Info,
        );
    }
    fn internal_state_management_thread(
        receiver: mpsc::Receiver<ThreadStateSpace>,
        mut state_broadcaster: Arc<AtomicU8>,
        timeout: u64,
    ) {
        log_message(format!("Pipeline management thread starting"), Level::Info);
        while state_broadcaster.load(Ordering::Acquire) != ThreadStateSpace::KILLED as u8 {
            let received_state = receiver.recv_timeout(std::time::Duration::from_millis(timeout));
            match received_state {
                Err(err) => Self::internal_receive_error_handler(err),
                Ok(state) => Self::state_supercession(&mut state_broadcaster, state),
            }
        }
        log_message(format!("Pipeline management thread exiting"), Level::Info);
    }
    fn internal_receive_error_handler(err: RecvTimeoutError) {
        match err {
            RecvTimeoutError::Disconnected => log_message(
                format!("Pipeline management thread lost connection to senders"),
                Level::Error,
            ),
            RecvTimeoutError::Timeout => (),
        }
    }
    fn state_supercession(state_broadcaster: &mut Arc<AtomicU8>, received_state: ThreadStateSpace) {
        let current_state = state_broadcaster.load(Ordering::Acquire);

        if current_state > received_state.clone() as u8 {
            log_message(
                format!(
                    "Pipeline management thread cannot switch to {} after receiving {}",
                    received_state, current_state
                ),
                Level::Warn,
            );
        } else {
            Self::broadcast_requested_state_stat(received_state.clone(), state_broadcaster);
            log_message(
                format!(
                    "Pipeline management thread rebroadcasted state: {}",
                    received_state
                ),
                Level::Debug,
            );
        }
    }
    pub fn is_started(&self) -> bool {
        self.manager_thread_handle.is_some()
    }
}

pub struct ConstructingPipeline {
    nodes: ConstructionQueue,
    parameters: PipelineParameters,
    state_passer: CommandStatePasser,
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
            nodes: Arc::new(SegQueue::new()),
            parameters,
            state_passer: CommandStatePasser::new(timeout),
        }
    }
    pub fn get_cloned_parameters(&self) -> PipelineParameters {
        self.parameters.clone()
    }
    pub fn get_state_communicators(&self) -> (Arc<AtomicU8>, mpsc::Sender<ThreadStateSpace>) {
        self.state_passer.extract_for_node()
    }
    pub fn get_nodes(&self) -> ConstructionQueue {
        self.nodes.clone()
    }
    pub fn finish_pipeline(mut self) -> ActivePipeline {
        let mut static_nodes = Vec::with_capacity(self.nodes.len());

        while self.nodes.len() > 0 {
            static_nodes.push(self.nodes.pop().unwrap());
        }

        ActivePipeline {
            nodes: static_nodes,
            parameters: self.parameters,
            state_passer: self.state_passer,
            start_time: Instant::now(),
        }
    }
    // pub fn finish_with_tui(mut self) -> App {
    //
    // }
}

pub struct ActivePipeline {
    nodes: Vec<PipelineThread>,
    parameters: PipelineParameters,
    state_passer: CommandStatePasser,
    start_time: Instant,
}
impl ActivePipeline {
    pub fn start(&mut self) {
        log_message(
            format!("Starting active pipeline length: {}", self.nodes.len()),
            Level::Debug,
        );
        if !self.state_passer.is_started() {
            self.state_passer.spawn_manager_thread();
        }
        self.state_passer
            .broadcast_requested_state(ThreadStateSpace::RUNNING)
    }

    pub fn stop(&mut self) {
        log_message(
            format!("Stopping active pipeline length: {}", self.nodes.len()),
            Level::Debug,
        );
        self.state_passer
            .broadcast_requested_state(ThreadStateSpace::PAUSED)
    }

    pub fn kill(mut self) {
        log_message(
            format!("Killing active pipeline length: {}", self.nodes.len()),
            Level::Debug,
        );
        self.state_passer
            .broadcast_requested_state(ThreadStateSpace::KILLED);

        for thread in self.nodes {
            thread.join()
        }

        self.state_passer.join();
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
