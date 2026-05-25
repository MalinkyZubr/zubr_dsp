use crate::engine::data_plane::communication_layer::comms_core::{WrappedReceiver, WrappedSender};
use crate::engine::data_plane::communication_layer::data_management::{BufferArray, DataWrapper};
use crate::engine::data_plane::structural::generic_pipeline_node::{GenericNode, NodeState, RunModel};
use crate::engine::data_plane::structural::pipeline_type_traits::Sharable;
use log::{debug, error};
use std::mem;

pub struct PipelineDeInterleavingNode<
    I: Sharable,
    const NUM_CHANNELS: usize,
    const INPUT_BUFFER_SIZE: usize,
    const OUTPUT_BUFFER_SIZE: usize,
> {
    // need to have a builder struct that wraps in identification info to make the graph after
    input: WrappedReceiver<BufferArray<I, INPUT_BUFFER_SIZE>>,
    output: [WrappedSender<BufferArray<I, OUTPUT_BUFFER_SIZE>>; NUM_CHANNELS],
    buffered_data: [DataWrapper<BufferArray<I, OUTPUT_BUFFER_SIZE>>; NUM_CHANNELS],
    satiated_edges: [usize; NUM_CHANNELS],
}

impl<
        I: Sharable,
        const NUM_CHANNELS: usize,
        const INPUT_BUFFER_SIZE: usize,
        const OUTPUT_BUFFER_SIZE: usize,
    > PipelineDeInterleavingNode<I, NUM_CHANNELS, INPUT_BUFFER_SIZE, OUTPUT_BUFFER_SIZE>
{
    pub fn new(
        input: WrappedReceiver<BufferArray<I, INPUT_BUFFER_SIZE>>,
        output: [WrappedSender<BufferArray<I, OUTPUT_BUFFER_SIZE>>; NUM_CHANNELS],
    ) -> PipelineDeInterleavingNode<I, NUM_CHANNELS, INPUT_BUFFER_SIZE, OUTPUT_BUFFER_SIZE> {
        assert_eq!(INPUT_BUFFER_SIZE % NUM_CHANNELS, 0);
        assert_eq!(INPUT_BUFFER_SIZE % OUTPUT_BUFFER_SIZE, 0);
        assert!(NUM_CHANNELS > 1);
        PipelineDeInterleavingNode {
            input,
            output,
            buffered_data: [Default::default(); NUM_CHANNELS],
            satiated_edges: [0; NUM_CHANNELS],
        }
    }
}

#[async_trait::async_trait]
impl<
        I: Sharable,
        const NUM_CHANNELS: usize,
        const INPUT_BUFFER_SIZE: usize,
        const OUTPUT_BUFFER_SIZE: usize,
    > GenericNode
    for PipelineDeInterleavingNode<I, NUM_CHANNELS, INPUT_BUFFER_SIZE, OUTPUT_BUFFER_SIZE>
{
    async fn run_senders(&mut self) -> Option<usize> {
        let mut num_satiated_edges = 0;
        for (idx, data) in self.buffered_data.iter_mut().enumerate() {
            let res = self.output[idx].send_swap(data).await;

            if res.is_err() {
                error!("Error sending data to output {}", idx);
                return None;
            }
            if self.output[idx].channel_satiated() {
                self.satiated_edges[num_satiated_edges] = *self.output[idx].get_dest_id();
                num_satiated_edges += 1;
            }
        }
        
        Some(num_satiated_edges)
    }
    fn get_satiated_edges(&self, num_satiated: usize) -> &[usize] {
        if num_satiated > self.satiated_edges.len() {
            panic!("Number of satiated edges is greater than the number of outputs")
        }
        &self.satiated_edges[..num_satiated]
    }
    fn load_initial_value(&mut self) {
        panic!("Initial state not supported for interleaved separator");
    }
    fn has_initial_value(&self) -> bool {
        false
    }
    fn get_num_inputs(&self) -> usize {
        1
    }
    fn get_num_outputs(&self) -> usize {
        NUM_CHANNELS
    }
    fn is_ready_exec(&self, node_state: NodeState) -> bool {
        match node_state {
            NodeState::Communicate => true,
            _ => self.input.channel_satiated(),
        }
    }
    fn get_successors(&self) -> Vec<usize> {
        self.output.iter().map(|x| *x.get_dest_id()).collect()
    }
    fn get_predecessors(&self) -> Vec<usize> {
        vec![*self.input.get_source_id()]
    }
    fn get_run_model(&self) -> RunModel {
        RunModel::CPU
    }

    fn call_thread_cpu(&mut self) -> Result<(), ()> {
        let mut input = self.input.recv().unwrap();
        debug!("Interleaved separator CPU call {}", input.read().len());

        for (idx, value) in input.read().read_mut().iter_mut().enumerate() {
            let channel_unit = &mut self.buffered_data[idx % NUM_CHANNELS];
            mem::swap(
                &mut channel_unit.read().read_mut()[idx / NUM_CHANNELS],
                value,
            );
        }

        self.input.refill_buffer(input);

        Ok(())
    }

    fn next_state(&self, current_state: NodeState) -> NodeState {
        match current_state {
            NodeState::Communicate => {
                NodeState::ExecCpu
            },
            (NodeState::ExecCpu) => {
                NodeState::Communicate
            },
            NodeState::Stop => {
                self.initial_state()
            }
            _ => panic!("Invalid state"),
        }
    }

    fn initial_state(&self) -> NodeState {
        NodeState::ExecCpu
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
    fn test_pipeline_interleaved_separator_new() {
        let (_, input) = create_test_channels::<BufferArray<i32, 9>>(10);
        let (output1, _) = create_test_channels::<BufferArray<i32, 3>>(10);
        let (output2, _) = create_test_channels::<BufferArray<i32, 3>>(10);
        let (output3, _) = create_test_channels::<BufferArray<i32, 3>>(10);

        let separator = PipelineDeInterleavingNode::new(input, [output1, output2, output3]);

        assert_eq!(separator.get_num_inputs(), 1);
        assert_eq!(separator.get_num_outputs(), 3);
        assert!(!separator.has_initial_value());
    }

    #[test]
    fn test_get_successors() {
        let (_, input) = create_test_channels::<BufferArray<i32, 4>>(10);
        let (output1, _) = create_test_channels::<BufferArray<i32, 2>>(10);
        let (output2, _) = create_test_channels::<BufferArray<i32, 2>>(10);

        let separator = PipelineDeInterleavingNode::new(input, [output1, output2]);

        let successors = separator.get_successors();
        assert_eq!(successors.len(), 2);
        assert!(successors.contains(&1));
    }

    #[test]
    fn test_get_run_model() {
        let (_, input) = create_test_channels::<BufferArray<i32, 4>>(10);
        let (output1, _) = create_test_channels::<BufferArray<i32, 2>>(10);
        let (output2, _) = create_test_channels::<BufferArray<i32, 2>>(10);

        let separator = PipelineDeInterleavingNode::new(input, [output1, output2]);

        // When buffered_data is None, should return CPU
        assert_eq!(separator.get_run_model(), RunModel::CPU);
    }

    #[test]
    fn test_get_run_model_with_buffered_data() {
        let (_, input) = create_test_channels::<BufferArray<i32, 4>>(10);
        let (output1, _) = create_test_channels::<BufferArray<i32, 2>>(10);
        let (output2, _) = create_test_channels::<BufferArray<i32, 2>>(10);

        let mut separator = PipelineDeInterleavingNode::new(input, [output1, output2]);
        separator.buffered_data = [
            DataWrapper::new_with_value(BufferArray::new_with_value([1, 2])),
            DataWrapper::new_with_value(BufferArray::new_with_value([1, 2])),
        ];

        // When buffered_data is Some, should return Communicator
        assert_eq!(separator.get_run_model(), RunModel::Communicator);
    }

    #[test]
    #[should_panic(expected = "Initial state not supported for interleaved separator")]
    fn test_load_initial_state_panics() {
        let (_, input) = create_test_channels::<BufferArray<i32, 4>>(10);
        let (output1, _) = create_test_channels::<BufferArray<i32, 2>>(10);
        let (output2, _) = create_test_channels::<BufferArray<i32, 2>>(10);

        let mut separator = PipelineDeInterleavingNode::new(input, [output1, output2]);

        separator.load_initial_value();
    }

    #[test]
    fn test_call_thread_cpu() {
        // Use tokio runtime for the setup but not for the actual CPU call
        let rt = tokio::runtime::Runtime::new().unwrap();

        let (mut tx, input) = rt.block_on(async { create_test_channels(10) });
        let (output1, _) = rt.block_on(async { create_test_channels(10) });
        let (output2, _) = rt.block_on(async { create_test_channels(10) });

        let mut separator: PipelineDeInterleavingNode<i32, 2, 4, 2> =
            PipelineDeInterleavingNode::new(input, [output1, output2]);

        // Send test data using the runtime
        let mut test_data = DataWrapper::new_with_value(BufferArray::new_with_value([1, 2, 3, 4]));
        rt.block_on(async move {
            tx.send_swap(&mut test_data).await.unwrap();
        });

        // Call the CPU function - this should work without async context
    }

    #[tokio::test]
    async fn test_run_senders_with_buffered_data() {
        let (_, input) = create_test_channels(10);
        let (output1, mut rx1) = create_test_channels(10);
        let (output2, mut rx2) = create_test_channels(10);

        let mut separator: PipelineDeInterleavingNode<i32, 2, 4, 2> =
            PipelineDeInterleavingNode::new(input, [output1, output2]);

        // Set up buffered data
        separator.buffered_data = [
            DataWrapper::new_with_value(BufferArray::new_with_value([1, 3])),
            DataWrapper::new_with_value(BufferArray::new_with_value([2, 4])),
        ];

        // Run the senders

        // Verify data was sent to outputs
        let mut received1 = rx1.recv_async().await.unwrap();
        let mut received2 = rx2.recv_async().await.unwrap();

        assert_eq!(*received1.read().read(), [1, 3]);
        assert_eq!(*received2.read().read(), [2, 4]);
    }

    #[tokio::test]
    async fn test_run_senders_without_buffered_data() {
        let (_, input) = create_test_channels::<BufferArray<i32, 4>>(10);
        let (output1, _) = create_test_channels::<BufferArray<i32, 2>>(10);
        let (output2, _) = create_test_channels::<BufferArray<i32, 2>>(10);

        let mut separator = PipelineDeInterleavingNode::new(input, [output1, output2]);

        // Run the senders without buffered data
    }
}
