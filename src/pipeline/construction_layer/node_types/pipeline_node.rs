use crate::pipeline::communication_layer::comms_core::{iterative_send, WrappedReceiver, WrappedSender};
use crate::pipeline::construction_layer::pipeline_traits::Sharable;
use std::fmt::Debug;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Instant;
use async_trait::async_trait;
use crate::pipeline::construction_layer::node_types::node_traits::{CPUCollectibleNode, CollectibleNode, IOCollectibleNode};
use crate::pipeline::construction_layer::node_types::pipeline_step::PipelineStep;

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
impl<I: Sharable, O: Sharable, const NI: usize, const NO: usize> CollectibleNode for PipelineNode<I, O, NI, NO> {
    async fn run_senders(&mut self, id: usize, increment_size: &mut usize) -> Vec<usize> {
        match self.buffered_data.take() {
            Some(output_data) => {
                iterative_send(&mut self.output, output_data).await.unwrap() // error handling later
            },
            None => {
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

impl<I: Sharable, O: Sharable, const NI: usize, const NO: usize> CPUCollectibleNode for PipelineNode<I, O, NI, NO> {
    fn call_thread_cpu(&mut self, id: usize) {
        let start_time = Instant::now();

        let received_data: [I; NI] = self.receive_input(id);
        let compute_result = self.execute_pipeline_step(received_data);

        match compute_result {
            Ok(value) => self.buffered_data = Some(value),
            Err(_) => ()
        }
        self.node_status
            .update_analytics(start_time.elapsed().as_nanos() as u64);
    }
}

#[async_trait]
impl<I: Sharable, O: Sharable, const NI: usize, const NO: usize> IOCollectibleNode for PipelineNode<I, O, NI, NO> {
    async fn call_thread_io(&mut self, id: usize) {
        let start_time = Instant::now();

        let received_data: [I; NI] = self.receive_input(id);
        let compute_result = self.execute_pipeline_step_io(received_data).await;

        match compute_result {
            Ok(value) => self.buffered_data = Some(value),
            Err(_) => ()
        }

        self.node_status
            .update_analytics(start_time.elapsed().as_nanos() as u64);
    }
}