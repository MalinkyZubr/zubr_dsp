use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use futures::SinkExt;
use crate::pipeline::construction_layer::pipeline_traits::Sharable;
use crate::pipeline::communication_layer::comms_core::{WrappedReceiver, WrappedSender};
use crate::pipeline::construction_layer::node_types::node_traits::{CPUCollectibleNode, CollectibleNode};
use crate::pipeline::construction_layer::node_types::pipeline_node::NodeStatus;

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
impl<I: Sharable, const NO: usize> CollectibleNode for PipelineSeriesDeconstructor<I, NO> {
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


#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::communication_layer::comms_core::channel_wrapped;

    #[test]
    fn series_deconstructor_sends_each_item_to_each_output_and_increments() {
        let (mut in_tx, in_rx) = channel_wrapped::<Vec<i32>>(8);

        let (out_tx0, mut out_rx0) = channel_wrapped::<i32>(8);
        let (out_tx1, mut out_rx1) = channel_wrapped::<i32>(8);

        let mut node: PipelineSeriesDeconstructor<i32, 2> =
            PipelineSeriesDeconstructor::new(in_rx, [out_tx0, out_tx1]);

        futures::executor::block_on(async {
            in_tx.send(vec![10, 20, 30]).await.unwrap();

            let mut inc = 0usize;
            node.run_senders(0, &mut inc).await;

            // Current behavior: increments once per (item, channel) send.
            assert_eq!(inc, 3 * 2);
        });

        assert_eq!(out_rx0.recv().unwrap(), 10);
        assert_eq!(out_rx0.recv().unwrap(), 20);
        assert_eq!(out_rx0.recv().unwrap(), 30);

        assert_eq!(out_rx1.recv().unwrap(), 10);
        assert_eq!(out_rx1.recv().unwrap(), 20);
        assert_eq!(out_rx1.recv().unwrap(), 30);
    }
}