use crate::pipeline::communication_layer::comms_core::{WrappedReceiver, WrappedSender};
use crate::pipeline::communication_layer::data_management::{BufferArray, DataWrapper};
use crate::pipeline::construction_layer::node_types::node_traits::{CollectibleNode, RunModel};
use crate::pipeline::construction_layer::pipeline_traits::Sharable;
use std::mem;
use log::{debug, error, warn};

pub struct PipelineInterleavedSeparator<
    I: Sharable,
    const NUM_CHANNELS: usize,
    const INPUT_BUFFER_SIZE: usize,
    const OUTPUT_BUFFER_SIZE: usize,
> where
    [(); INPUT_BUFFER_SIZE % NUM_CHANNELS]: Sized,
    [(); INPUT_BUFFER_SIZE % OUTPUT_BUFFER_SIZE]: Sized, // input buffer size should be perfectly divisible by NUM_CHANNELS
{
    // need to have a builder struct that wraps in identification info to make the graph after
    input: WrappedReceiver<BufferArray<I, INPUT_BUFFER_SIZE>>,
    output: [WrappedSender<BufferArray<I, OUTPUT_BUFFER_SIZE>>; NUM_CHANNELS],
    buffered_data: [DataWrapper<BufferArray<I, OUTPUT_BUFFER_SIZE>>; NUM_CHANNELS],
    satiated_edges: [usize; NUM_CHANNELS],
    output_ready: bool,
}

impl<
        I: Sharable,
        const NUM_CHANNELS: usize,
        const INPUT_BUFFER_SIZE: usize,
        const OUTPUT_BUFFER_SIZE: usize,
    > PipelineInterleavedSeparator<I, NUM_CHANNELS, INPUT_BUFFER_SIZE, OUTPUT_BUFFER_SIZE>
where
    [(); INPUT_BUFFER_SIZE % NUM_CHANNELS]: Sized,
    [(); INPUT_BUFFER_SIZE % OUTPUT_BUFFER_SIZE]: Sized, // input buffer size should be perfectly divisible by NUM_CHANNELS
{
    pub fn new(
        input: WrappedReceiver<BufferArray<I, INPUT_BUFFER_SIZE>>,
        output: [WrappedSender<BufferArray<I, OUTPUT_BUFFER_SIZE>>; NUM_CHANNELS],
    ) -> PipelineInterleavedSeparator<I, NUM_CHANNELS, INPUT_BUFFER_SIZE, OUTPUT_BUFFER_SIZE> {
        assert!(NUM_CHANNELS > 1);
        PipelineInterleavedSeparator {
            input,
            output,
            buffered_data: [Default::default(); NUM_CHANNELS],
            satiated_edges: [0; NUM_CHANNELS],
            output_ready: false,
        }
    }
}

#[async_trait::async_trait]
impl<
        I: Sharable,
        const NUM_CHANNELS: usize,
        const INPUT_BUFFER_SIZE: usize,
        const OUTPUT_BUFFER_SIZE: usize,
    > CollectibleNode
    for PipelineInterleavedSeparator<I, NUM_CHANNELS, INPUT_BUFFER_SIZE, OUTPUT_BUFFER_SIZE>
where
    [(); INPUT_BUFFER_SIZE % NUM_CHANNELS]: Sized,
    [(); INPUT_BUFFER_SIZE % OUTPUT_BUFFER_SIZE]: Sized, // input buffer size should be perfectly divisible by NUM_CHANNELS
{
    async fn run_senders(&mut self, _id: usize) -> Option<usize> {
        let mut num_satiated_edges = 0;
        for (idx, data) in self.buffered_data.iter_mut().enumerate() {
            let res = self.output[idx].send_swap(data).await;

            if res.is_err() {
                error!("Error sending data to output {}", idx);
                self.output_ready = false;
                return None;
            }
            if self.output[idx].channel_satiated() {
                self.satiated_edges[num_satiated_edges] = *self.output[idx].get_dest_id();
                num_satiated_edges += 1;
            }
        }

        self.output_ready = false;
        Some(num_satiated_edges)
    }
    fn check_nth_satiated_edge_id(&self, edge_index: usize) -> Option<usize> {
        if edge_index < NUM_CHANNELS {
            Some(self.satiated_edges[edge_index])
        } else {
            None
        }
    }
    fn load_initial_state(&mut self) {
        panic!("Initial state not supported for interleaved separator");
    }
    fn has_initial_state(&self) -> bool {
        false
    }
    fn get_num_inputs(&self) -> usize {
        1
    }
    fn get_num_outputs(&self) -> usize {
        NUM_CHANNELS
    }
    fn is_ready_exec(&self) -> bool {
        self.input.channel_satiated()
    }
    fn get_successors(&self) -> Vec<usize> {
        self.output.iter().map(|x| *x.get_dest_id()).collect()
    }
    fn get_run_model(&self) -> RunModel {
        if self.output_ready {
            RunModel::Communicator
        } else {
            RunModel::CPU
        }
    }

    fn call_thread_cpu(&mut self, _id: usize) {
        let mut input = self.input.recv().unwrap();
        debug!("Interleaved separator CPU call {}", input.read().len());

        for (idx, value) in input.read().read_mut().iter_mut().enumerate() {
            let channel_unit = &mut self.buffered_data[idx % NUM_CHANNELS];
            mem::swap(&mut channel_unit.read().read_mut()[idx / NUM_CHANNELS], value);
        }

        self.input.refill_buffer(input);

        self.output_ready = true;
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
            crate::pipeline::communication_layer::comms_core::make_crossbeam_queue_handles(12);

        (
            WrappedSender::new(
                tx,
                1,
                notify.clone(),
                capacity.clone(),
                channel_wrapped_consumer,
            ),
            WrappedReceiver::new(rx, 0, notify, capacity, channel_wrapped_producer),
        )
    }

    #[test]
    fn test_pipeline_interleaved_separator_new() {
        let (_, input) = create_test_channels::<BufferArray<i32, 9>>(10);
        let (output1, _) = create_test_channels::<BufferArray<i32, 3>>(10);
        let (output2, _) = create_test_channels::<BufferArray<i32, 3>>(10);
        let (output3, _) = create_test_channels::<BufferArray<i32, 3>>(10);

        let separator = PipelineInterleavedSeparator::new(input, [output1, output2, output3]);

        assert_eq!(separator.get_num_inputs(), 1);
        assert_eq!(separator.get_num_outputs(), 3);
        assert!(!separator.has_initial_state());
    }

    #[test]
    fn test_get_successors() {
        let (_, input) = create_test_channels::<BufferArray<i32, 4>>(10);
        let (output1, _) = create_test_channels::<BufferArray<i32, 2>>(10);
        let (output2, _) = create_test_channels::<BufferArray<i32, 2>>(10);

        let separator = PipelineInterleavedSeparator::new(input, [output1, output2]);

        let successors = separator.get_successors();
        assert_eq!(successors.len(), 2);
        assert!(successors.contains(&1));
    }

    #[test]
    fn test_get_run_model() {
        let (_, input) = create_test_channels::<BufferArray<i32, 4>>(10);
        let (output1, _) = create_test_channels::<BufferArray<i32, 2>>(10);
        let (output2, _) = create_test_channels::<BufferArray<i32, 2>>(10);

        let separator = PipelineInterleavedSeparator::new(input, [output1, output2]);

        // When buffered_data is None, should return CPU
        assert_eq!(separator.get_run_model(), RunModel::CPU);
    }

    #[test]
    fn test_get_run_model_with_buffered_data() {
        let (_, input) = create_test_channels::<BufferArray<i32, 4>>(10);
        let (output1, _) = create_test_channels::<BufferArray<i32, 2>>(10);
        let (output2, _) = create_test_channels::<BufferArray<i32, 2>>(10);

        let mut separator = PipelineInterleavedSeparator::new(input, [output1, output2]);
        separator.buffered_data = [
            DataWrapper::new_with_value(BufferArray::new_with_value([1, 2])),
            DataWrapper::new_with_value(BufferArray::new_with_value([1, 2])),
        ];
        separator.output_ready = true;

        // When buffered_data is Some, should return Communicator
        assert_eq!(separator.get_run_model(), RunModel::Communicator);
    }

    #[test]
    #[should_panic(expected = "Initial state not supported for interleaved separator")]
    fn test_load_initial_state_panics() {
        let (_, input) = create_test_channels::<BufferArray<i32, 4>>(10);
        let (output1, _) = create_test_channels::<BufferArray<i32, 2>>(10);
        let (output2, _) = create_test_channels::<BufferArray<i32, 2>>(10);

        let mut separator = PipelineInterleavedSeparator::new(input, [output1, output2]);

        separator.load_initial_state();
    }

    #[test]
    fn test_call_thread_cpu() {
        // Use tokio runtime for the setup but not for the actual CPU call
        let rt = tokio::runtime::Runtime::new().unwrap();

        let (mut tx, input) = rt.block_on(async { create_test_channels(10) });
        let (output1, _) = rt.block_on(async { create_test_channels(10) });
        let (output2, _) = rt.block_on(async { create_test_channels(10) });

        let mut separator: PipelineInterleavedSeparator<i32, 2, 4, 2> =
            PipelineInterleavedSeparator::new(input, [output1, output2]);

        // Send test data using the runtime
        let mut test_data = DataWrapper::new_with_value(BufferArray::new_with_value([1, 2, 3, 4]));
        rt.block_on(async move {
            tx.send_swap(&mut test_data).await.unwrap();
        });

        // Call the CPU function - this should work without async context
        separator.call_thread_cpu(0);
    }

    #[tokio::test]
    async fn test_run_senders_with_buffered_data() {
        let (_, input) = create_test_channels(10);
        let (output1, mut rx1) = create_test_channels(10);
        let (output2, mut rx2) = create_test_channels(10);

        let mut separator: PipelineInterleavedSeparator<i32, 2, 4, 2> =
            PipelineInterleavedSeparator::new(input, [output1, output2]);

        // Set up buffered data
        separator.buffered_data = [
            DataWrapper::new_with_value(BufferArray::new_with_value([1, 3])),
            DataWrapper::new_with_value(BufferArray::new_with_value([2, 4])),
        ];

        // Run the senders
        let result = separator.run_senders(0).await;
        assert!(result.is_some());

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

        let mut separator = PipelineInterleavedSeparator::new(input, [output1, output2]);

        // Run the senders without buffered data
        let result = separator.run_senders(0).await;
        assert!(result.is_none());
    }
}
