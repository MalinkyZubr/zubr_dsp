use crate::pipeline::api::*;
use crate::pipeline::communication_layer::comms_core::{WrappedReceiver, WrappedSender};
use crate::pipeline::communication_layer::formats::{ODFormat, ReceiveType};
use crate::pipeline::construction_layer::pipeline_step::PipelineStep;
use crate::pipeline::construction_layer::pipeline_tap::PipelineTap;
use crate::pipeline::construction_layer::pipeline_traits::Sharable;
use std::fmt::Debug;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Instant;

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

pub struct PipelineNode<I: Sharable, O: Sharable> {
    // need to have a buuilder struct that wraps in identification info to make the graph after
    input: Vec<WrappedReceiver<I>>,
    output: Vec<WrappedSender<O>>,
    step: Box<dyn PipelineStep<I, O>>,
    tap: Option<PipelineTap<I>>,
    node_status: NodeStatus,
    buffered_data: Option<ODFormat<O>>,
}
impl<I: Sharable, O: Sharable> PipelineNode<I, O> {
    pub fn new(step: Box<dyn PipelineStep<I, O>>, input: Vec<WrappedReceiver<I>>, output: Vec<WrappedSender<O>>, tap: Option<PipelineTap<I>>) -> PipelineNode<I, O> {
        PipelineNode {
            input,
            output,
            tap,
            node_status: NodeStatus::new(),
            step,
            buffered_data: None,
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

pub trait CollectibleThreadPrecursor {
    fn add_input<I: Sharable>(&mut self, receiver: WrappedReceiver<I>);
    fn add_output<O: Sharable>(&mut self, sender: WrappedSender<O>);
    fn attach_step<I: Sharable, O: Sharable>(&mut self, step: Box<dyn PipelineStep<I, O>>);
    fn attach_tap<I: Sharable>(&mut self, tap: PipelineTap<I>);
    fn set_required_input_count(&mut self, count: usize);
    fn set_name(&mut self, name: String);
    fn get_id(&self) -> usize;
    fn clear_outputs(&mut self);
    fn into_collectible_thread<I: Sharable, O: Sharable>(self) -> Box<dyn CollectibleThread>;
}

pub trait CollectibleThread: Send {
    fn call_thread(&mut self, id: &usize);
    async fn run_senders(&mut self, id: &usize, increment_size: &mut usize); // runs a join set and waits for all tasks in that set to finish
    fn clone_output_stop_flag(&self, id: usize) -> Arc<AtomicBool>;
}

impl<I: Sharable, O: Sharable> CollectibleThread for PipelineNode<I, O> {
    fn call_thread(&mut self, id: &usize) {
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

    async fn run_senders(&mut self, id: &usize, increment_size: &mut usize) -> Vec<usize> {
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
}
