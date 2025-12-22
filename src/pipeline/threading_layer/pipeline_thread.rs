use crate::pipeline::api::*;
use crate::pipeline::construction_layer::pipeline_node::{PipelineNode, PipelineStepResult};
use crate::pipeline::interfaces::PipelineStep;
use crate::pipeline::orchestration_layer::logging::log_message;
use crate::pipeline::orchestration_layer::logging::Level;
use crate::pipeline::orchestration_layer::pipeline::PipelineParameters;
use crate::pipeline::pipeline_traits::{HasID, Sharable};
use crate::pipeline::threading_layer::thread_state_space::ThreadStateSpace;
use num_enum::TryFromPrimitive;
use std::convert::TryFrom;
use std::fmt::Debug;
use std::sync::mpsc::RecvTimeoutError;
use std::thread::sleep;
use std::time::{Duration, Instant};

struct ThreadErrorCounter {
    max_infrastructure_errors: usize,
    infrastructure_errors_received: usize, // kill when reach limit
    max_compute_errors: usize,
    compute_errors_received: usize, // pause when reach limit
}
impl ThreadErrorCounter {
    pub fn new(max_infrastructure_errors: usize, max_compute_errors: usize) -> Self {
        Self {
            max_compute_errors,
            max_infrastructure_errors,
            infrastructure_errors_received: 0,
            compute_errors_received: 0,
        }
    }
    pub fn infrastructure_error(&mut self) {
        self.infrastructure_errors_received += 1;
    }
    pub fn compute_error(&mut self) {
        self.compute_errors_received += 1;
    }
    pub fn success(&mut self) {
        self.infrastructure_errors_received = 0;
        self.compute_errors_received = 0;
    }
    pub fn compute_error_lim_check(&mut self, id: &String) -> bool {
        if self.max_compute_errors == 0 {
            false
        } else if self.compute_errors_received > self.max_compute_errors {
            log_message(
                format!("ThreadID: {} max allowed compute errors received", id),
                Level::Warn,
            );
            true
        } else {
            false
        }
    }
    pub fn infrastructure_error_lim_check(&mut self, id: &String) -> bool {
        if self.max_infrastructure_errors == 0 {
            false
        } else if self.infrastructure_errors_received > self.max_infrastructure_errors {
            log_message(
                format!(
                    "ThreadID: {} max allowed infrastructure errors received",
                    id
                ),
                Level::Error,
            );
            true
        } else {
            false
        }
    }
}

struct ThreadStateMachine {
    state: ThreadStateSpace,
    //previous_result: PipelineStepResult,
    error_counter: ThreadErrorCounter,
    no_change_timer: u64,
    id: String,
}
impl ThreadStateMachine {
    pub fn new(parameters: &PipelineParameters, id: String) -> Self {
        let error_counter = ThreadErrorCounter::new(
            parameters.max_infrastructure_errors,
            parameters.max_compute_errors,
        );
        Self {
            error_counter,
            state: ThreadStateSpace::PAUSED,
            id,
            no_change_timer: parameters.unchanged_state_time,
        }
    }
    fn evaluate_step_result<I: Sharable, O: Sharable>(
        &mut self,
        output: &PipelineStepResult,
        step: &mut Box<dyn PipelineStep<I, O>>,
    ) {
        match output {
            PipelineStepResult::SendError => {
                self.error_counter.infrastructure_error();
                log_message(
                    format!("ThreadID: {} send error received", &self.id),
                    Level::Warn,
                );
                if self.error_counter.infrastructure_error_lim_check(&self.id) {
                    self.set_kill_state(step)
                }
            }
            PipelineStepResult::RecvTimeoutError(err) => {
                self.error_counter.infrastructure_error();
                match err {
                    RecvTimeoutError::Timeout => log_message(
                        format!("ThreadID: {} receive timeout received", &self.id),
                        Level::Warn,
                    ),
                    RecvTimeoutError::Disconnected => log_message(
                        format!("ThreadID: {} receiver disconnected received", &self.id),
                        Level::Warn,
                    ),
                }
                if self.error_counter.infrastructure_error_lim_check(&self.id) {
                    self.set_kill_state(step)
                }
            }
            PipelineStepResult::ComputeError(message) => {
                self.error_counter.compute_error();
                log_message(
                    format!("ThreadID: {} compute error {}, pausing", &self.id, message),
                    Level::Warn,
                );
                if self.error_counter.compute_error_lim_check(&self.id) {
                    self.set_pause_state(step)
                };
            }
            PipelineStepResult::Success => self.error_counter.success(),
        }
    }
    fn set_kill_state<I: Sharable, O: Sharable>(&mut self, step: &mut Box<dyn PipelineStep<I, O>>) {
        self.state = ThreadStateSpace::KILLED;
        step.kill_behavior();
        log_message(
            format!("ThreadID: {} state set killed", &self.id),
            Level::Info,
        );
    }
    fn set_pause_state<I: Sharable, O: Sharable>(
        &mut self,
        step: &mut Box<dyn PipelineStep<I, O>>,
    ) {
        self.state = ThreadStateSpace::PAUSED;
        step.pause_behavior();
        log_message(
            format!("ThreadID: {} state set paused", &self.id),
            Level::Info,
        );
    }
    fn set_running_state<I: Sharable, O: Sharable>(
        &mut self,
        step: &mut Box<dyn PipelineStep<I, O>>,
    ) {
        self.state = ThreadStateSpace::RUNNING;
        step.start_behavior();
        log_message(
            format!("ThreadID: {} state set running", &self.id),
            Level::Info,
        );
    }
    fn kill_request_handler<I: Sharable, O: Sharable>(
        &mut self,
        step: &mut Box<dyn PipelineStep<I, O>>,
    ) {
        if self.state != ThreadStateSpace::KILLED {
            self.set_kill_state(step);
            log_message(
                format!("ThreadID: {} set kill state, exiting", &self.id),
                Level::Info,
            );
        } else {
            sleep(Duration::from_millis(self.no_change_timer))
        }
    }
    fn pause_request_handler<I: Sharable, O: Sharable>(
        &mut self,
        step: &mut Box<dyn PipelineStep<I, O>>,
    ) {
        match self.state {
            ThreadStateSpace::PAUSED => sleep(Duration::from_millis(self.no_change_timer)),
            ThreadStateSpace::RUNNING => self.set_pause_state(step),
            ThreadStateSpace::KILLED => {
                log_message(
                    format!(
                        "ThreadID: {} is killed, cannot set to paused state",
                        &self.id
                    ),
                    Level::Warn,
                );
                sleep(Duration::from_millis(self.no_change_timer))
            }
        }
    }
    fn running_request_handler<I: Sharable, O: Sharable>(
        &mut self,
        step: &mut Box<dyn PipelineStep<I, O>>,
    ) {
        match self.state {
            ThreadStateSpace::RUNNING => (),
            ThreadStateSpace::PAUSED => self.set_running_state(step),
            ThreadStateSpace::KILLED => {
                log_message(
                    format!(
                        "ThreadID: {} is killed, cannot set to running state",
                        &self.id
                    ),
                    Level::Warn,
                );
                sleep(Duration::from_millis(self.no_change_timer))
            }
        }
    }
    fn call<I: Sharable, O: Sharable>(
        &mut self,
        node: &mut PipelineNode<I, O>,
        step: &mut Box<dyn PipelineStep<I, O>>,
    ) -> Result<PipelineStepResult, ()> {
        let result = match self.state {
            ThreadStateSpace::RUNNING => {
                log_message(format!("ThreadID: {} call start", &self.id), Level::Debug);
                let res = node.call(step);
                self.evaluate_step_result(&res, step);
                log_message(format!("ThreadID: {} call done", &self.id), Level::Debug);
                Ok(res)
            }
            _ => Err(()),
        };
        result
    }
    fn state_transition<I: Sharable, O: Sharable>(
        &mut self,
        requested_state: ThreadStateSpace,
        step: &mut Box<dyn PipelineStep<I, O>>,
    ) {
        if requested_state == ThreadStateSpace::KILLED {
            self.kill_request_handler(step);
        } else {
            match requested_state {
                ThreadStateSpace::PAUSED => self.pause_request_handler(step),
                ThreadStateSpace::RUNNING => self.running_request_handler(step),
                _ => (),
            }
        }
    }

    fn get_current_state(&self) -> ThreadStateSpace {
        self.state.clone()
    }
}

pub struct PipelineThread<I: Sharable, O: Sharable> {
    pub execution_time: u64,
    pub return_code: PipelineStepResult,
    pub id: String,

    thread_state_machine: ThreadStateMachine,

    step: Box<dyn PipelineStep<I, O>>,
    node: PipelineNode<I, O>,
}

impl<I: Sharable, O: Sharable> PipelineThread<I, O> {
    pub fn new(
        step: impl PipelineStep<I, O> + 'static,
        node: PipelineNode<I, O>,
        parameters: PipelineParameters,
    ) -> PipelineThread<I, O> {
        // requires node to be borrowed as static?
        let id = node.get_id();
        let mut thread = PipelineThread {
            execution_time: 0,
            return_code: PipelineStepResult::Success,
            id: node.get_id(),
            node,
            step: Box::new(step),
            thread_state_machine: ThreadStateMachine::new(&parameters, id),
        };

        thread
    }

    pub fn request_state_change(&mut self, requested_state: ThreadStateSpace) {
        self.thread_state_machine
            .state_transition(requested_state, &mut self.step);
    }
}

pub trait CollectibleThread: Send {
    fn call_thread(&mut self);
}

impl<I: Sharable, O: Sharable> CollectibleThread for PipelineThread<I, O> {
    fn call_thread(&mut self) {
        if self.thread_state_machine.state != ThreadStateSpace::KILLED {
            log_message(
                format!(
                    "ThreadID: {} requested state {}, current state {}",
                    self.id,
                    self.thread_state_machine.get_current_state(),
                    self.thread_state_machine.state
                ),
                Level::Trace,
            );
            let start_time = Instant::now();

            let last_return_code = self.return_code.clone();
            self.return_code = self
                .thread_state_machine
                .call(&mut self.node, &mut self.step)
                .unwrap_or(last_return_code);

            self.execution_time = start_time.elapsed().as_micros() as u64;
        }
        log_message(
            format!("ThreadID: {} state machine end of action loop", self.id),
            Level::Trace,
        );
    }
}
