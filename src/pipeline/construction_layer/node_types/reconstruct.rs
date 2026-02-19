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
    async fn run_senders(&mut self, id: usize) -> Vec<usize> {
        let mut results = vec![];
        for _ in 0..self.receive_demands {
            results.push(self.input.recv_async().await.unwrap());
        }

        iterative_send(&mut self.output, results).await.unwrap()
    }
    fn load_initial_state(&mut self) {
        panic!("Series reconstructor does not support initial state")
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
    fn series_reconstructor_collects_receive_demands_items_and_sends_vec() {
        let (mut in_tx, in_rx) = channel_wrapped::<i32>(8);

        let (out_tx0, mut out_rx0) = channel_wrapped::<Vec<i32>>(8);
        let (out_tx1, mut out_rx1) = channel_wrapped::<Vec<i32>>(8);

        let mut node: PipelineSeriesReconstructor<i32, 2> =
            PipelineSeriesReconstructor::new(in_rx, [out_tx0, out_tx1], 3);

        futures::executor::block_on(async {
            in_tx.send(1).await.unwrap();
            in_tx.send(2).await.unwrap();
            in_tx.send(3).await.unwrap();

            let mut inc = 0usize;
            node.run_senders(0, &mut inc).await;

            assert_eq!(inc, 1);
        });

        assert_eq!(out_rx0.recv().unwrap(), vec![1, 2, 3]);
        assert_eq!(out_rx1.recv().unwrap(), vec![1, 2, 3]);
    }
}
