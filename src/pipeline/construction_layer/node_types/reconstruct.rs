use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Instant;
use log::Level;
use crate::pipeline::construction_layer::pipeline_traits::Sharable;
use crate::pipeline::communication_layer::comms_core::{WrappedReceiver, WrappedSender};
use crate::pipeline::communication_layer::formats::ReceiveType;
use crate::pipeline::construction_layer::node_types::pipeline_node::{CPUCollectibleThread, CollectibleThread, NodeStatus};

pub struct PipelineSeriesReconstructor<I: Sharable, const NO: usize>
{
    // need to have a buuilder struct that wraps in identification info to make the graph after
    input: WrappedReceiver<I>,
    output: [WrappedSender<Vec<I>>; NO],
    node_status: NodeStatus,
    receive_demands: usize
}


impl<I: Sharable, const NO: usize> PipelineSeriesReconstructor<I, NO> {
    pub fn new(input: WrappedReceiver<I>, output: [WrappedSender<Vec<I>>; NO], receive_demands: usize) -> Self {
        Self { input, output, node_status: NodeStatus::new(), receive_demands }
    }
}


impl<I: Sharable> CollectibleThread for PipelineSeriesReconstructor<I> {
    async fn run_senders(&mut self, id: usize, increment_size: &mut usize) -> Vec<usize> {
        let input = (0..self.receive_demands)
            .map(|_| self.input.receive())
            .collect();
        self.output.send(input);
        *increment_size += 1;
    }
    fn clone_output_stop_flag(&self, id: usize) -> Option<Arc<AtomicBool>> {
        for sender in self.output.iter() {
            if *sender.get_dest_id() == id {
                return Some(sender.clone_stop_flag());
            }
        }
        None
    }
    fn load_initial_state(&mut self) {
        panic!("Series reconstructor does not support initial state")
    }
    fn has_initial_state(&self) -> bool {
        false
    }
}