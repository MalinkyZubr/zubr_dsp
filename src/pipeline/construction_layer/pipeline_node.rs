use std::cmp::PartialEq;
use crate::pipeline::api::*;
use crate::pipeline::communication_layer::comms_top_level::{NodeReceiver, NodeSender};
use crate::pipeline::interfaces::{ODFormat, PipelineStep, ReceiveType};
use crate::pipeline::pipeline_traits::{HasID, Sharable};
use crossbeam_queue::ArrayQueue;
use std::fmt::Debug;
use std::sync::mpsc::RecvTimeoutError;
use std::sync::Arc;
use std::collections::HashMap;
use std::time::Instant;
use strum::Display;
use crate::pipeline::communication_layer::comms_core::{ChannelMetadata, ChannelState};
use crate::pipeline::communication_layer::comms_core::ChannelState::OKAY;

#[derive(Debug, PartialEq, Clone)]
pub enum PipelineStepResult {
    Success,
    SendError,
    RecvTimeoutError(RecvTimeoutError),
    ComputeError(String),
}

#[derive(Debug, Copy, Clone)]
struct DummyStep {}
impl<T: Sharable> PipelineStep<T, T> for DummyStep {
    fn run_SISO(&mut self, input: T) -> Result<ODFormat<T>, String> {
        Ok(ODFormat::Standard(input))
    }
}


#[derive(Debug, Clone)]
pub struct NodeStatus {
    num_executions: usize,
    last_execution_time_ns: u64,
    node_return_code: PipelineStepResult,
    node_state: ChannelState
}
impl NodeStatus {
    pub fn new() -> NodeStatus {
        NodeStatus {
            num_executions: 0,
            last_execution_time_ns: 0,
            node_return_code: PipelineStepResult::Success,
            node_state: OKAY
        }
    }
    
    pub fn update_analytics(&mut self, execution_time_ns: u64) {
        self.num_executions += 1;
        self.last_execution_time_ns = execution_time_ns;
    }
}


pub struct PipelineNode<I: Sharable, O: Sharable> {
    id: String,
    input: NodeReceiver<I>,
    output: NodeSender<O>,
    step: Box<dyn PipelineStep<I, O>>,
    tap: Option<Arc<ArrayQueue<ODFormat<O>>>>,
    node_status: NodeStatus
}

impl<I: Sharable, O: Sharable> HasID for PipelineNode<I, O> {
    fn get_id(&self) -> String {
        self.id.clone()
    }

    fn set_id(&mut self, id: &str) {
        self.id = id.to_string();
    }
}

impl<I: Sharable, O: Sharable> PipelineNode<I, O> {
    pub fn new(step: Box<dyn PipelineStep<I, O>>) -> PipelineNode<I, O> {
        PipelineNode {
            input: NodeReceiver::Dummy,
            output: NodeSender::Dummy,
            id: "".to_string(),
            tap: None,
            node_status: NodeStatus::new(),
            step
        }
    }

    pub fn call(&mut self) -> PipelineStepResult {
        let received_result = self.input.receive();
        match received_result {
            Err(err) => {
                let result = PipelineStepResult::RecvTimeoutError(err);
                result
            } // must have a way to handle if it is a dummy
            Ok(val) => {
                let result = self.route_computation(val);
                result
            }
        }
    }

    fn compute_handler(&mut self, output_data: Result<ODFormat<O>, String>) -> PipelineStepResult {
        match output_data {
            Err(err) => PipelineStepResult::ComputeError(err),
            Ok(extracted_data) => {
                let extracted_data = self.push_to_tap(extracted_data);
                match self.output.send(extracted_data) {
                    Err(_) => PipelineStepResult::SendError,
                    Ok(_) => PipelineStepResult::Success,
                }
            }
        }
    }
    fn push_to_tap(&mut self, output_data: ODFormat<O>) -> ODFormat<O> {
        match &mut self.tap {
            None => output_data,
            Some(tap) => {
                let cloned_output = output_data.clone();
                tap.push(output_data);
                cloned_output
            }
        }
    }

    fn route_computation(
        &mut self,
        input_data: ReceiveType<I>,
    ) -> PipelineStepResult {
        // This method decides based on the connections to and from this node which computation method should be run
        self.compute_handler(match (input_data, &self.output) {
            (ReceiveType::Single(t), NodeSender::SO(_) | NodeSender::MUO(_)) => self.step.run_SISO(t),
            (ReceiveType::Single(t), NodeSender::MO(_)) => self.step.run_SIMO(t),
            (ReceiveType::Multichannel(t), NodeSender::SO(_) | NodeSender::MUO(_)) => {
                self.step.run_MISO(t)
            }
            (ReceiveType::Multichannel(t), NodeSender::MO(_)) => self.step.run_MIMO(t),
            (ReceiveType::Reassembled(t), NodeSender::SO(_) | NodeSender::MUO(_)) => {
                self.step.run_REASO(t)
            }
            (ReceiveType::Reassembled(t), NodeSender::MO(_)) => self.step.run_REAMO(t),
            (ReceiveType::Dummy, NodeSender::SO(_)) => self.step.run_DISO(),
            (ReceiveType::Single(t), NodeSender::Dummy) => self.step.run_SIDO(t),
            (_, _) => Err(String::from("Received bad message from pipeline step")),
        })
    }
    
    pub fn get_metadata(&self) -> Vec<ChannelMetadata> {
        self.input.get_metadata()
    }
}


pub trait CollectibleThread: Send {
    fn call_thread(&mut self);
    fn get_id(&self) -> String;
    fn get_channel_metadata(&self) -> Vec<ChannelMetadata>;
}


impl<I: Sharable, O: Sharable> CollectibleThread for PipelineNode<I, O> {
    fn call_thread(&mut self) {
        if self.node_status.node_state == OKAY {
            log_message(
                format!(
                    "ThreadID: {} is called",
                    self.id,
                ),
                Level::Trace,
            );
            let start_time = Instant::now();
            
            let return_code = self.call();
            let last_return_code = self.node_status.node_return_code.clone();
            self.node_status.update_analytics(start_time.elapsed().as_nanos() as u64)
        }
        log_message(
            format!("ThreadID: {} state machine end of action loop", self.id),
            Level::Trace,
        );
    }

    fn get_id(&self) -> String {
        self.id.clone()
    }

    fn get_channel_metadata(&self) -> Vec<ChannelMetadata> {
        self.get_metadata()
    }
}
