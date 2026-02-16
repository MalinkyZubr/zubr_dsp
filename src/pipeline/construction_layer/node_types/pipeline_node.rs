use std::collections::HashMap;
use crate::pipeline::communication_layer::comms_core::{WrappedReceiver, WrappedSender};
use crate::pipeline::construction_layer::pipeline_traits::Sharable;
use std::fmt::Debug;
use std::future::Future;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Instant;
use async_trait::async_trait;
use log::Level;
use crate::pipeline::construction_layer::node_types::pipeline_step::{PipelineStep};

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
    step: Box<dyn PipelineStep<I, O, NI>>,
    node_status: NodeStatus,
    initial_state: Option<O>,
    buffered_data: Option<O>,
}
impl<I: Sharable, O: Sharable, const NI: usize, const NO: usize> PipelineNode<I, O, NI, NO> {
    pub fn new(step: Box<dyn PipelineStep<I, O, NI>>, input: Vec<WrappedReceiver<I>>, output: Vec<WrappedSender<O>>, initial_state: Option<O>) -> PipelineNode<I, O, NI, NO> {
        let input: [WrappedReceiver<I>; NI] = input.try_into().unwrap();
        let output: [WrappedSender<O>; NO] = output.try_into().unwrap();
        
        PipelineNode {
            input,
            output,
            node_status: NodeStatus::new(),
            step,
            buffered_data: initial_state.clone(),
            initial_state,
        }
    }

    fn receive_input(&mut self, id: usize) -> [I; NI] {
        let input: [I; NI] = std::array::from_fn(|i| {
            self.input[i].recv().unwrap() // error handling later when get work
        });
        input
    }

    fn execute_pipeline_step(&mut self, input_data: [I; NI]) -> Result<O, ()> {
        self.step.run_cpu(input_data)
    }
    
    async fn execute_pipeline_step_io(&mut self, input_data: [I; NI]) -> Result<O, ()> {
        self.step.run_io(input_data).await
    }
}


#[async_trait]
pub trait CollectibleThread {
    async fn run_senders(&mut self, id: usize, increment_size: &mut usize) -> Vec<usize>; // runs a join set and waits for all tasks in that set to finish
    fn clone_output_stop_flag(&self, id: usize) -> Option<Arc<AtomicBool>>;
    fn load_initial_state(&mut self);
    fn has_initial_state(&self) -> bool;
    fn get_id(&self) -> usize;
}


pub trait CPUCollectibleThread: Send + CollectibleThread {
    fn call_thread(&mut self, id: usize);
}


#[async_trait]
pub trait IOCollectibleThread: Send + CollectibleThread {
    async fn call_thread(&mut self, id: usize);
}


#[async_trait]
impl<I: Sharable, O: Sharable, const NI: usize, const NO: usize> CollectibleThread for PipelineNode<I, O, NI, NO> {
    async fn run_senders(&mut self, id: usize, increment_size: &mut usize) -> Vec<usize> {
        match self.buffered_data.take() {
            Some(output_data) => {
                for sender in 0..self.output.len() - 1 {
                    self.output[sender].send(output_data.clone()).await;
                }
                self.output[self.output.len() - 1].send(output_data).await; // add error handling later
                *increment_size += 1;
                vec![0] // I forgot what this does so this is just a placeholder
            },
            None => {
                //log_message(format!("No data to send on node {}", id), Level::Error);
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


impl<I: Sharable, O: Sharable, const NI: usize, const NO: usize> CPUCollectibleThread for PipelineNode<I, O, NI, NO> {
    fn call_thread(&mut self, id: usize) {
        //log_message(format!("ThreadID: {} is called", id, ), Level::Trace);
        let start_time = Instant::now();

        let received_data: [I; NI] = self.receive_input(id);
        //if received_data.is_some() {
        let compute_result = self.execute_pipeline_step(received_data);

        match compute_result {
            Ok(value) => self.buffered_data = Some(value),
            Err(_) => ()//log_message(format!("Error in computation on node {}", id), Level::Error),
        }

        self.node_status
            .update_analytics(start_time.elapsed().as_nanos() as u64);
        //} else {
            //self.buffered_data = None;
            //log_message(format!("No data received on node {}", id), Level::Error);
        //}
        // log_message(
        //     format!("ThreadID: {} state machine end of action loop", id), // find a way to propogate identifiers downwards for logs
        //     Level::Trace,
        // );
    }
}


#[async_trait]
impl<I: Sharable, O: Sharable, const NI: usize, const NO: usize> IOCollectibleThread for PipelineNode<I, O, NI, NO> {
    async fn call_thread(&mut self, id: usize) {
        //log_message(format!("ThreadID: {} is called", id, ), Level::Trace);
        let start_time = Instant::now();

        let received_data: [I; NI] = self.receive_input(id);
        //if received_data.is_some() {
        let compute_result = self.execute_pipeline_step_io(received_data).await;

        match compute_result {
            Ok(value) => self.buffered_data = Some(value),
            Err(_) => ()//log_message(format!("Error in computation on node {}", id), Level::Error),
        }

        self.node_status
            .update_analytics(start_time.elapsed().as_nanos() as u64);
        //} else {
        //self.buffered_data = None;
        //log_message(format!("No data received on node {}", id), Level::Error);
        //}
        // log_message(
        //     format!("ThreadID: {} state machine end of action loop", id), // find a way to propogate identifiers downwards for logs
        //     Level::Trace,
        // );
    }
}

