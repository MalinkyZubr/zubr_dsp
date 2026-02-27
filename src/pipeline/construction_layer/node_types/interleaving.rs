use std::collections::HashSet;
use crate::pipeline::communication_layer::comms_core::{WrappedReceiver, WrappedSender};
use crate::pipeline::construction_layer::node_types::node_traits::{CollectibleNode, RunModel};
use crate::pipeline::construction_layer::pipeline_traits::Sharable;
use futures::SinkExt;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

#[derive(Debug)]
pub struct PipelineInterleavedSeparator<I: Sharable, const NUM_CHANNELS: usize> {
    // need to have a builder struct that wraps in identification info to make the graph after
    input: WrappedReceiver<Vec<I>>,
    output: [WrappedSender<Vec<I>>; NUM_CHANNELS],
    buffered_data: Option<[Vec<I>; NUM_CHANNELS]>,
}

impl<I: Sharable, const NUM_CHANNELS: usize> PipelineInterleavedSeparator<I, NUM_CHANNELS> {
    pub fn new(
        input: WrappedReceiver<Vec<I>>,
        output: [WrappedSender<Vec<I>>; NUM_CHANNELS],
    ) -> PipelineInterleavedSeparator<I, NUM_CHANNELS> {
        PipelineInterleavedSeparator {
            input,
            output,
            buffered_data: None,
        }
    }
}

#[async_trait::async_trait]
impl<I: Sharable, const NUM_CHANNELS: usize> CollectibleNode
    for PipelineInterleavedSeparator<I, NUM_CHANNELS>
{
    fn is_ready_exec(&self) -> bool {
        self.input.channel_satiated()
    }
    fn get_successors(&self) -> Vec<usize> {
        self.output.iter().map(|x| *x.get_dest_id()).collect()
    }
    fn get_run_model(&self) -> RunModel {
        match &self.buffered_data {
            Some(_) => RunModel::Communicator,
            None => RunModel::CPU,
        }
    }
    fn get_num_inputs(&self) -> usize {
        1
    }
    fn get_num_outputs(&self) -> usize {
        NUM_CHANNELS
    }
    async fn run_senders(&mut self, id: usize) -> Option<Vec<usize>> { // very inefficient function. Optimize later
        match self.buffered_data.take() {
            Some(data) => {
                let mut satiated_edges: HashSet<usize> = HashSet::new();
                for (index, val) in data.into_iter().enumerate() {
                    let sender = &mut self.output[index];
                    match sender.send(val).await {
                        Ok(_) => (),
                        Err(_) => return None
                    }
                    if sender.channel_satiated() {
                        satiated_edges.insert(*sender.get_dest_id());
                    }
                }
                Some(satiated_edges.into_iter().collect())
            }
            _ => None,
        }
    }
    fn load_initial_state(&mut self) {
        panic!("Initial state not supported for interleaved separator");
    }
    fn has_initial_state(&self) -> bool {
        false
    }

    fn call_thread_cpu(&mut self, id: usize) {
        let input: Vec<I> = self.input.recv().unwrap();
        let mut output_values: [Vec<I>; NUM_CHANNELS] =
            vec![Vec::new(); NUM_CHANNELS].try_into().unwrap();

        let mut current_channel: usize = 0;
        for value in input {
            output_values[current_channel].push(value);
            current_channel = (current_channel + 1) % NUM_CHANNELS;
        }
    }
}