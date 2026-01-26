use crate::pipeline::api::*;
use crate::pipeline::communication_layer::comms_core::ChannelState::{ERROR, OKAY};
use crate::pipeline::communication_layer::comms_core::{
    ChannelMetadata, ChannelState, WrappedSender,
};
use crate::pipeline::communication_layer::comms_top_level::{NodeReceiver, NodeSender};
use crate::pipeline::construction_layer::pipeline_step::PipelineStep;
use crate::pipeline::interfaces::{ODFormat, PipelineStep, ReceiveType};
use crate::pipeline::pipeline_traits::{HasID, Sharable};
use crossbeam_queue::ArrayQueue;
use std::cmp::PartialEq;
use std::fmt::Debug;
use std::sync::mpsc::RecvTimeoutError;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Instant;
use crate::pipeline::communication_layer::formats::{ODFormat, ReceiveType};

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
    node_state: ChannelState,
}
impl NodeStatus {
    pub fn new() -> NodeStatus {
        NodeStatus {
            num_executions: 0,
            last_execution_time_ns: 0,
            node_return_code: PipelineStepResult::Success,
            node_state: OKAY,
        }
    }

    pub fn update_analytics(&mut self, execution_time_ns: u64) {
        self.num_executions += 1;
        self.last_execution_time_ns = execution_time_ns;
    }
}

pub struct PipelineNode<I: Sharable, O: Sharable> {
    // need to have a buuilder struct that wraps in identification info to make the graph after
    input: NodeReceiver<I>,
    output: Vec<WrappedSender<O>>,
    step: Box<dyn PipelineStep<I, O>>,
    tap: Option<Arc<ArrayQueue<ODFormat<O>>>>,
    node_status: NodeStatus,
    buffered_data: ODFormat<O>,
}
impl<I: Sharable, O: Sharable> PipelineNode<I, O> {
    pub fn new(step: Box<dyn PipelineStep<I, O>>) -> PipelineNode<I, O> {
        PipelineNode {
            input: NodeReceiver::Dummy,
            output: NodeSender::Dummy,
            id: "".to_string(),
            tap: None,
            node_status: NodeStatus::new(),
            step,
        }
    }

    fn formatted_computation(&mut self, input_data: ReceiveType<I>) -> Result<ODFormat<O>, ()> {
        // This method decides based on the connections to and from this node which computation method should be run
        self.compute_handler(match (input_data, &self.output) {
            (ReceiveType::Single(t), NodeSender::SO(_) | NodeSender::MUO(_)) => {
                self.step.run_SISO(t)
            }
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
}

pub trait CollectibleThread: Send {
    fn call_thread(&mut self, id: &usize);
    async fn run_senders(&mut self); // runs a join set and waits for all tasks in that set to finish
    fn clone_stop_flag(&self, id: usize) -> Arc<AtomicBool>;
}

impl<I: Sharable, O: Sharable> CollectibleThread for PipelineNode<I, O> {
    fn call_thread(&mut self, id: &usize) {
        if self.node_status.node_state == OKAY {
            log_message(format!("ThreadID: {} is called", id,), Level::Trace);
            let start_time = Instant::now();

            let received_data: Option<ReceiveType<I>>  = self.input.receive();
            if received_data.is_none() {
                self.node_status.node_state = ERROR;
            }
            let received_data = received_data.unwrap();
            let compute_result = self.formatted_computation(received_data);

            match compute_result {
                Ok(value) => self.buffered_data = value,
                Err(_) => self.node_status.node_return_code = PipelineStepResult::ComputeError("Compute Error".to_string()),
            }

            self.node_status
                .update_analytics(start_time.elapsed().as_nanos() as u64)
        }
        log_message(
            format!("ThreadID: {} state machine end of action loop", id), // find a way to propogate identifiers downwards for logs
            Level::Trace,
        );
    }

    async fn run_senders(&mut self) {
        self.buffered_data.send(&mut self.output).await
    }

    fn clone_stop_flag(&self, id: usize) -> Option<Arc<AtomicBool>> {
        for sender in self.output.iter() {
            if *sender.get_dest_id() == id {
                return Some(sender.clone_stop_flag())
            }
        }
        None
    }
}
