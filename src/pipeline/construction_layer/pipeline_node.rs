use std::collections::HashMap;
use crate::pipeline::api::*;
use crate::pipeline::communication_layer::comms_core::{WrappedReceiver, WrappedSender};
use crate::pipeline::communication_layer::formats::{ODFormat, ReceiveType};
use crate::pipeline::construction_layer::pipeline_step::PipelineStep;
use crate::pipeline::construction_layer::pipeline_traits::Sharable;
use std::fmt::Debug;
use std::future::Future;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Instant;
use async_trait::async_trait;

#[derive(Debug, Copy, Clone)]
struct DummyStep {}
impl<T: Sharable> PipelineStep<T, T> for DummyStep {
    fn run_SISO(&mut self, input: T) -> Result<ODFormat<T>, ()> {
        Ok(ODFormat::Standard(input))
    }
}

#[derive(Debug, Clone)]
pub struct NodeStatus {
    num_executions: usize,
    last_execution_time_ns: u64,
}
impl NodeStatus {
    pub fn new() -> NodeStatus {
        NodeStatus {
            num_executions: 0,
            last_execution_time_ns: 0,
        }
    }

    pub fn update_analytics(&mut self, execution_time_ns: u64) {
        self.num_executions += 1;
        self.last_execution_time_ns = execution_time_ns;
    }
}

pub struct PipelineNode<I: Sharable, O: Sharable, const NI: usize, const NO: usize> {
    // need to have a buuilder struct that wraps in identification info to make the graph after
    input: [WrappedReceiver<I>; NI],
    output: [WrappedSender<O>; NO],
    step: Box<dyn PipelineStep<I, O>>,
    node_status: NodeStatus,
    initial_state: Option<ODFormat<O>>,
    buffered_data: Option<ODFormat<O>>,
}
impl<I: Sharable, O: Sharable, const NI: usize, const NO: usize> PipelineNode<I, O, NI, NO> {
    pub fn new(step: Box<dyn PipelineStep<I, O>>, input: Vec<WrappedReceiver<I>>, output: Vec<WrappedSender<O>>, initial_state: Option<ODFormat<O>>) -> PipelineNode<I, O> {
        PipelineNode {
            input,
            output,
            node_status: NodeStatus::new(),
            step,
            buffered_data: initial_state.clone(),
            initial_state: initial_state,
        }
    }

    fn receive_input(&mut self, id: usize) -> Result<ReceiveType<I>, ()> {
        if self.input.len() == 1 {
            self.input[0].recv()
        } else {
            let mut receive_vector: Vec<I> = Vec::with_capacity(self.input.len());
            for receiver in self.input.iter_mut() {
                match receiver.recv() {
                    Ok(value) => match value {
                        ReceiveType::Single(x) => receive_vector.push(x),
                        _ => {
                            log_message(
                                format!(
                                    "Cannot reassemble on a joint id {} from {}",
                                    id,
                                    receiver.get_source_id()
                                ),
                                Level::Error,
                            );
                            return Err(());
                        }
                    },
                    Err(_) => {
                        log_message(
                            format!(
                                "Communication Failure receiving on node {} from {}",
                                id,
                                receiver.get_source_id()
                            ),
                            Level::Error,
                        );
                        return Err(());
                    }
                }
            }
            Ok(ReceiveType::Multichannel(receive_vector))
        }
    }

    fn execute_pipeline_step(&mut self, input_data: ReceiveType<I>) -> Result<ODFormat<O>, ()> {
        match input_data {
            ReceiveType::Single(value) => {
                if self.output.len() == 1 {
                    self.step.run_SISO(value)
                } else {
                    self.step.run_SIMO(value)
                }
            }
            ReceiveType::Multichannel(value) => {
                if self.output.len() == 1 {
                    self.step.run_MISO(value)
                } else {
                    self.step.run_MIMO(value)
                }
            }
            ReceiveType::Reassembled(value) => {
                if self.output.len() == 1 {
                    self.step.run_REASO(value)
                } else {
                    self.step.run_REAMO(value)
                }
            }
            ReceiveType::Dummy => {
                if self.output.len() == 1 {
                    self.step.run_DISO()
                } else {
                    Err(())
                }
            }
        }
    }
}


pub struct BuildingNode<I: Sharable, O: Sharable, const NI: usize, const NO: usize> {
    id: usize,
    name: String,
    step: Option<Box<dyn PipelineStep<I, O>>>,
    inputs: Vec<WrappedReceiver<I>>,
    outputs: Vec<WrappedSender<O>>,
    successors: HashMap<usize, usize>, // (id, num executions)
    input_count: usize,
    initial_state: Option<ODFormat<O>>,
}
impl<I: Sharable, O: Sharable, const NI: usize, const NO: usize> BuildingNode<I, O, NI, NO> {
    fn add_input(&mut self, receiver: WrappedReceiver<I>) {
        self.inputs.push(receiver)
    }

    fn add_output(&mut self, sender: WrappedSender<O>) {
        self.outputs.push(sender)
    }

    fn clear_outputs(&mut self) {
        self.outputs.clear();
    }

    fn attach_step(&mut self, step: impl PipelineStep<I, O>) {
        self.step = Some(Box::new(step));
    }

    fn set_required_input_count(&mut self, count: usize) {
        self.input_count = count;
    }

    fn set_name(&mut self, name: String) {
        self.name = name;
    }

    fn get_id(&self) -> usize {
        self.id
    }
    fn get_name(&self) -> String {
        self.name.clone()
    }
    fn into_collectible_thread(mut self) -> (Box<dyn CollectibleThread>, HashMap<usize, usize>) {
        if self.step.is_none() {
            panic!("Cannot convert BuildingNode into CollectibleThread without a step attached")
        }
        let step = self.step.take().unwrap();
        let new_node: PipelineNode<I, O, NI, NO> = PipelineNode::new(
            step,
            self.inputs.take(),
            self.outputs.take(),
            self.initial_state.take(),
        );
        (Box::new(new_node), self.successors)
    }
    fn add_initial_state(&mut self, initial_state: O) {
        self.initial_state = Some(initial_state);
    }
}


#[async_trait]
pub trait CollectibleThread: Send {
    fn call_thread(&mut self, id: usize);
    async fn run_senders(&mut self, id: usize, increment_size: &mut usize) -> Vec<usize>; // runs a join set and waits for all tasks in that set to finish
    fn clone_output_stop_flag(&self, id: usize) -> Option<Arc<AtomicBool>>;
    fn load_initial_state(&mut self);
    fn has_initial_state(&self) -> bool;
}


#[async_trait]
impl<I: Sharable, O: Sharable, const NI: usize, const NO: usize> CollectibleThread for PipelineNode<I, O, NI, NO> {
    fn call_thread(&mut self, id: usize) {
        log_message(format!("ThreadID: {} is called", id, ), Level::Trace);
        let start_time = Instant::now();

        let received_data: Option<ReceiveType<I>> = self.input.receive();
        if received_data.is_some() {
            let received_data = received_data.unwrap();
            let compute_result = self.execute_pipeline_step(received_data);

            match compute_result {
                Ok(value) => self.buffered_data = value,
                Err(_) => log_message(format!("Error in computation on node {}", id), Level::Error),
            }

            self.node_status
                .update_analytics(start_time.elapsed().as_nanos() as u64);
        } else {
            self.buffered_data = None;
            log_message(format!("No data received on node {}", id), Level::Error);
        }
        log_message(
            format!("ThreadID: {} state machine end of action loop", id), // find a way to propogate identifiers downwards for logs
            Level::Trace,
        );
    }

    async fn run_senders(&mut self, id: usize, increment_size: &mut usize) -> Vec<usize> {
        match self.buffered_data.take() {
            Some(output_data) => output_data.send(&mut self.output, increment_size).await,
            None => {
                log_message(format!("No data to send on node {}", id), Level::Error);
                Vec::new()
            }
        }
    }

    fn clone_output_stop_flag(&self, id: usize) -> Option<Arc<AtomicBool>> {
        for sender in self.output.iter() {
            if *sender.get_dest_id() == id {
                return Some(sender.clone_stop_flag());
            }
        }
        None
    }

    fn has_initial_state(&self) -> bool {
        self.initial_state.is_some()
    }

    fn load_initial_state(&mut self) {
        self.buffered_data = self.initial_state.clone()
    }
}
