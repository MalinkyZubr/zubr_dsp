use crate::pipeline::communication_layer::comms_core::{WrappedReceiver, WrappedSender};
use crate::pipeline::construction_layer::node_types::node_traits::{CollectibleNode, RunModel};
use crate::pipeline::construction_layer::pipeline_traits::Sharable;
use futures::SinkExt;

#[derive(Debug)]
pub struct PipelineSeriesDeconstructor<I: Sharable, const NO: usize> {
    // need to have a buuilder struct that wraps in identification info to make the graph after
    input: WrappedReceiver<Vec<I>>,
    output: [WrappedSender<I>; NO],
}

impl<I: Sharable, const NO: usize> PipelineSeriesDeconstructor<I, NO> {
    pub fn new(input: WrappedReceiver<Vec<I>>, output: [WrappedSender<I>; NO]) -> Self {
        PipelineSeriesDeconstructor { input, output }
    }
}

#[async_trait::async_trait]
impl<I: Sharable, const NO: usize> CollectibleNode for PipelineSeriesDeconstructor<I, NO> {
    async fn run_senders(&mut self, id: usize) -> Option<Vec<usize>> {
        let received = self.input.recv_async().await.unwrap();
        for item in received {
            for sender in self.output.iter_mut() {
                match sender.send(item.clone()).await {
                    Ok(_) => (),
                    Err(_) => return None,
                }
            }
        }

        let mut satiated_edges: Vec<usize> = Vec::new();
        for sender in self.output.iter_mut() {
            if sender.channel_satiated() {
                satiated_edges.push(*sender.get_dest_id());
            }
        }

        Some(satiated_edges)
    }
    fn load_initial_state(&mut self) {
        panic!("Series deconstructor should not have initial state")
    }
    fn has_initial_state(&self) -> bool {
        false
    }
    fn get_num_inputs(&self) -> usize {
        1
    }
    fn get_num_outputs(&self) -> usize {
        NO
    }
    fn is_ready_exec(&self) -> bool {
        self.input.channel_satiated()
    }
    fn get_successors(&self) -> Vec<usize> {
        self.output.iter().map(|x| *x.get_dest_id()).collect()
    }
    fn get_run_model(&self) -> RunModel {
        RunModel::Communicator
    }
}