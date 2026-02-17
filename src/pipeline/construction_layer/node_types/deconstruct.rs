use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Instant;
use futures::SinkExt;
use log::Level;
use crate::pipeline::construction_layer::pipeline_traits::Sharable;
use crate::pipeline::communication_layer::comms_core::{WrappedReceiver, WrappedSender};
use crate::pipeline::construction_layer::node_types::pipeline_node::{CPUCollectibleThread, CollectibleThread, NodeStatus};

pub struct PipelineSeriesDeconstructor<I: Sharable, const NO: usize>
{
    // need to have a buuilder struct that wraps in identification info to make the graph after
    input: WrappedReceiver<Vec<I>>,
    output: [WrappedSender<I>; NO],
    node_status: NodeStatus,
}


impl<I: Sharable, const NO: usize> PipelineSeriesDeconstructor<I, NO> {
    pub fn new(input: WrappedReceiver<Vec<I>>, output: [WrappedSender<I>; NO]) -> Self {
        PipelineSeriesDeconstructor {
            input,
            output,
            node_status: NodeStatus::new(),
        }
    }
}


#[async_trait::async_trait]
impl<I: Sharable, const NO: usize> CollectibleThread for PipelineSeriesDeconstructor<I, NO> {
    async fn run_senders(&mut self, id: usize, increment_size: &mut usize) -> Vec<usize> {
        let received = self.input.recv().unwrap();
        for item in received {
            for sender in self.output.iter_mut() {
                sender.send(item.clone()).await;
                *increment_size += 1; // this is wrong. Must increment on each channel separately. Fix later
            }
        }
        
        Vec::new()
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
        panic!("Series deconstructor should not have initial state")
    }
    fn has_initial_state(&self) -> bool {
        false
    }
}