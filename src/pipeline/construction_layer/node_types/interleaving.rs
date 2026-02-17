use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Instant;
use futures::SinkExt;
use log::Level;
use crate::pipeline::construction_layer::pipeline_traits::Sharable;
use crate::pipeline::communication_layer::comms_core::{WrappedReceiver, WrappedSender};
use crate::pipeline::construction_layer::node_types::pipeline_node::{CPUCollectibleThread, CollectibleThread, NodeStatus, PipelineNode};

pub struct PipelineInterleavedSeparator<I: Sharable, const NUM_CHANNELS: usize>
{
    // need to have a buuilder struct that wraps in identification info to make the graph after
    input: WrappedReceiver<Vec<I>>,
    output: [WrappedSender<Vec<I>>; NUM_CHANNELS],
    node_status: NodeStatus,
    buffered_data: Option<[Vec<I>; NUM_CHANNELS]>,
}


impl<I: Sharable, const NUM_CHANNELS: usize> PipelineInterleavedSeparator<I, NUM_CHANNELS> {
    pub fn new(input: WrappedReceiver<Vec<I>>, output: [WrappedSender<Vec<I>>; NUM_CHANNELS]) -> PipelineInterleavedSeparator<I, NUM_CHANNELS> {
        PipelineInterleavedSeparator {
            input,
            output,
            node_status: NodeStatus::new(),
            buffered_data: None,
        }
    }
}


#[async_trait::async_trait]
impl<I: Sharable, const NUM_CHANNELS: usize> CollectibleThread for PipelineInterleavedSeparator<I, NUM_CHANNELS> {
    async fn run_senders(&mut self, id: usize, increment_size: &mut usize) -> Vec<usize> {
        match self.buffered_data.take() {
            Some(data) => {
                for (index, val) in data.into_iter().enumerate() {
                    self.output[index].send(val).await; // error handling later
                }
                vec![]
            }
            _ => vec![]
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
    fn load_initial_state(&mut self) {
        panic!("Initial state not supported for interleaved separator");
    }
    fn has_initial_state(&self) -> bool {
        false
    }
}

impl<I: Sharable, const NUM_CHANNELS: usize> CPUCollectibleThread for PipelineInterleavedSeparator<I, NUM_CHANNELS> {
    fn call_thread(&mut self, id: usize) {
        let mut input: Vec<I> = self.input.recv().unwrap();
        let mut output_values: [Vec<I>; NUM_CHANNELS] = vec![Vec::new(); NUM_CHANNELS].try_into().unwrap();
        
        let mut current_channel: usize = 0;
        for value in input {
            output_values[current_channel].push(value);
            current_channel = (current_channel + 1) % NUM_CHANNELS;
        }
    }
}