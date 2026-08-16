use std::mem;
use crate::engine::data_plane::communication_layer::comms_core::{
    iterative_send, WrappedReceiver, WrappedSender,
};
use crate::engine::data_plane::communication_layer::data_management::{BufferArray};
use crate::engine::data_plane::structural::generic_pipeline_node::{GenericNode, NodeState, RunModel};
use crate::engine::data_plane::structural::generic_pipeline_node::NodeState::ExecCommunicate;
use crate::engine::data_plane::structural::pipeline_type_traits::Sharable;


pub struct PipelineReconstructorNode<I: Sharable, const NO: usize, const NR: usize> {
    // need to have a buuilder struct that wraps in identification info to make the graph after
    input: WrappedReceiver<I>,
    output: [WrappedSender<BufferArray<I, NR>>; NO],
    satiated_edges: [usize; NO],
    buffered_input: BufferArray<I, NR>,
}

impl<I: Sharable, const NO: usize, const NR: usize> PipelineReconstructorNode<I, NO, NR> {
    pub fn new(
        mut input: WrappedReceiver<I>,
        output: [WrappedSender<BufferArray<I, NR>>; NO],
    ) -> Self {
        input.set_satiation_capacity(NR);
        Self {
            input,
            output,
            satiated_edges: [0; NO],
            buffered_input: BufferArray::new(),
        }
    }
}

#[async_trait::async_trait]
impl<I: Sharable, const NO: usize, const NR: usize> GenericNode
    for PipelineReconstructorNode<I, NO, NR>
{
    async fn run_senders(&mut self) -> Option<usize> {
        for idx in 0..NR {
            match self.input.recv_async().await {
                Some(mut data) => {
                    mem::swap(&mut data, &mut self.buffered_input.read_mut()[idx]);
                    self.input.refill_buffer(data);
                }
                None => return None,
            }; // unwrap is okay because this assumes all predecessors are ready
        }
        
        iterative_send(
            &mut self.output,
            &mut self.satiated_edges,
            &mut self.buffered_input,
        )
        .await
        .ok()
    }
    fn get_satiated_edges(&self, num_satiated: usize) -> &[usize] {
        if num_satiated > self.satiated_edges.len() {
            panic!("Number of satiated edges is greater than the number of outputs")
        }
        &self.satiated_edges[..num_satiated]
    }
    fn load_initial_value(&mut self) {
        panic!("Series reconstructor does not support initial state")
    }
    fn has_initial_value(&self) -> bool {
        false
    }
    fn get_num_inputs(&self) -> usize {
        1
    }
    fn get_num_outputs(&self) -> usize {
        NO
    }
    fn is_ready_exec(&self, _node_state: NodeState) -> bool {
        self.input.channel_satiated()
    }
    fn get_successors(&self) -> Vec<usize> {
        self.output.iter().map(|x| *x.get_dest_id()).collect()
    }
    fn get_predecessors(&self) -> Vec<usize> {
        vec![*self.input.get_source_id()]
    }
    fn get_run_model(&self) -> RunModel {
        RunModel::Communicator
    }
    fn initial_state(&self) -> NodeState {
        ExecCommunicate
    }
    fn next_state(&self, _current_state: NodeState) -> NodeState {
        ExecCommunicate
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;
    use std::sync::Arc;
    use tokio::sync::mpsc;
    use tokio::sync::Notify;

    fn create_test_channels<T: Sharable>(
        buffer_size: usize,
    ) -> (WrappedSender<T>, WrappedReceiver<T>) {
        let (tx, rx) = mpsc::channel(buffer_size);
        let notify = Arc::new(Notify::new());
        let capacity = Arc::new(AtomicUsize::new(1));

        let (channel_wrapped_producer, channel_wrapped_consumer) =
            crate::engine::data_plane::communication_layer::comms_core::make_crossbeam_queue_handles(12);

        (
            WrappedSender::new(
                tx,
                1,
                capacity.clone(),
                channel_wrapped_consumer,
            ),
            WrappedReceiver::new(rx, 0, capacity, channel_wrapped_producer),
        )
    }

    #[test]
    fn test_pipeline_series_reconstructor_new() {
        let (_, input) = create_test_channels::<i32>(10);
        let (output1, _) = create_test_channels::<BufferArray<i32, 3>>(10);
        let (output2, _) = create_test_channels::<BufferArray<i32, 3>>(10);

        let reconstructor = PipelineReconstructorNode::new(input, [output1, output2]);
        
        assert_eq!(reconstructor.get_num_inputs(), 1);
        assert_eq!(reconstructor.get_num_outputs(), 2);
        assert!(!reconstructor.has_initial_value());
        assert_eq!(reconstructor.get_run_model(), RunModel::Communicator);
    }

    #[test]
    fn test_get_successors() {
        let (_, input) = create_test_channels::<i32>(10);
        let (output1, _) = create_test_channels::<BufferArray<i32, 3>>(10);
        let (output2, _) = create_test_channels::<BufferArray<i32, 3>>(10);

        let reconstructor = PipelineReconstructorNode::new(input, [output1, output2]);

        let successors = reconstructor.get_successors();
        assert_eq!(successors.len(), 2);
        assert!(successors.contains(&1));
    }

    #[test]
    #[should_panic(expected = "Series reconstructor does not support initial state")]
    fn test_load_initial_state_panics() {
        let (_, input) = create_test_channels::<i32>(10);
        let (output1, _) = create_test_channels::<BufferArray<i32, 3>>(10);
        let (output2, _) = create_test_channels::<BufferArray<i32, 3>>(10);

        let mut reconstructor = PipelineReconstructorNode::new(input, [output1, output2]);

        reconstructor.load_initial_value();
    }

    // #[tokio::test]
    // async fn test_run_senders() {
    //     let (mut tx, input) = create_test_channels(10);
    //     let (output1, mut rx1) = create_test_channels(10);
    //     let (output2, mut rx2) = create_test_channels(10);
    // 
    //     let mut reconstructor = PipelineReconstructorNode::new(input, [output1, output2]);
    // 
    //     // Send test data
    //     tx.send_swap(&mut DataWrapper::new_with_value(1))
    //         .await
    //         .unwrap();
    //     tx.send_swap(&mut DataWrapper::new_with_value(2))
    //         .await
    //         .unwrap();
    // 
    //     // Verify both outputs received the data
    //     let mut received1 = rx1.recv_async().await.unwrap();
    //     let mut received2 = rx2.recv_async().await.unwrap();
    // 
    //     assert_eq!(received1.read(), [1, 2]);
    //     assert_eq!(received2.read(), [1, 2]);
    // }
}
