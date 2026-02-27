use futures::future;
use crate::pipeline::communication_layer::comms_core::{
    iterative_send, WrappedReceiver, WrappedSender,
};
use crate::pipeline::construction_layer::node_types::node_traits::{CollectibleNode, RunModel};
use crate::pipeline::construction_layer::pipeline_traits::Sharable;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

#[derive(Debug)]
pub struct PipelineSeriesReconstructor<I: Sharable, const NO: usize> {
    // need to have a buuilder struct that wraps in identification info to make the graph after
    input: WrappedReceiver<I>,
    output: [WrappedSender<Vec<I>>; NO],
    receive_demands: usize,
}

impl<I: Sharable, const NO: usize> PipelineSeriesReconstructor<I, NO> {
    pub fn new(
        input: WrappedReceiver<I>,
        output: [WrappedSender<Vec<I>>; NO],
        receive_demands: usize,
    ) -> Self {
        Self {
            input,
            output,
            receive_demands,
        }
    }
}

#[async_trait::async_trait]
impl<I: Sharable, const NO: usize> CollectibleNode for PipelineSeriesReconstructor<I, NO> {
    fn is_ready_exec(&self) -> bool {
        self.input.channel_satiated()
    }
    fn get_successors(&self) -> Vec<usize> {
        self.output.iter().map(|x| *x.get_dest_id()).collect()
    }
    fn get_run_model(&self) -> RunModel {
        RunModel::Communicator
    }
    fn get_num_inputs(&self) -> usize {
        1
    }
    fn get_num_outputs(&self) -> usize {
        NO
    }
    async fn run_senders(&mut self, id: usize) -> Option<Vec<usize>> {
        let mut results = vec![];
        for _ in 0..self.receive_demands {
            results.push(self.input.recv_async().await.unwrap()); // unwrap is okay because this assumes all predecessors are ready 
        }

        iterative_send(&mut self.output, results).await.ok()
    }
    fn load_initial_state(&mut self) {
        panic!("Series reconstructor does not support initial state")
    }
    fn has_initial_state(&self) -> bool {
        false
    }
}