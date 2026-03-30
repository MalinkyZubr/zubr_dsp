use crate::pipeline::communication_layer::comms_core::{iterative_send, WrappedReceiver, WrappedSender};
use crate::pipeline::communication_layer::data_management::{BufferArray, DataWrapper};
use crate::pipeline::construction_layer::node_types::node_traits::{CollectibleNode, RunModel};
use crate::pipeline::construction_layer::pipeline_traits::Sharable;


pub struct PipelineSeriesDeconstructor<I: Sharable, const NO: usize, const ND: usize> {
    // need to have a buuilder struct that wraps in identification info to make the graph after
    input: WrappedReceiver<BufferArray<I, ND>>,
    output: [WrappedSender<I>; NO],
    satiated_edges: [usize; NO],
    buffered_input: [DataWrapper<I>; ND]
}

impl<I: Sharable, const NO: usize, const ND: usize> PipelineSeriesDeconstructor<I, NO, ND> {
    pub fn new(input: WrappedReceiver<BufferArray<I, ND>>, output: [WrappedSender<I>; NO]) -> Self {
        PipelineSeriesDeconstructor { input, output, satiated_edges: [0;NO], buffered_input: [DataWrapper::new(); ND] }
    }
}

#[async_trait::async_trait]
impl<I: Sharable, const NO: usize, const ND: usize> CollectibleNode for PipelineSeriesDeconstructor<I, NO, ND> {
    async fn run_senders(&mut self, _id: usize) -> Option<usize> {
        let mut received = self.input.recv_async().await.unwrap();
        let mut num_received = 0;
        
        for (idx, item) in received.read().read_mut().iter_mut().enumerate() {
            self.buffered_input[idx].swap(item);
            num_received = match iterative_send(
                &mut self.output, &mut self.satiated_edges, &mut self.buffered_input[idx]
            ).await.ok() {
                Some(num) => num,
                None => {self.input.refill_buffer(received); return None},
            }
        }
        
        self.input.refill_buffer(received); // give the received value back to the buffer holder

        Some(num_received)
    }

    fn check_nth_satiated_edge_id(&self, edge_index: usize) -> Option<usize> {
        if edge_index < NO {
            Some(self.satiated_edges[edge_index])
        }
        else {
            None
        }
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

        (
            WrappedSender::new(tx, 1, notify.clone(), capacity.clone()),
            WrappedReceiver::new(rx, 0, notify, capacity),
        )
    }

    #[test]
    fn test_pipeline_series_deconstructor_new() {
        let (_, input) = create_test_channels::<Vec<i32>>(10);
        let (output1, _) = create_test_channels::<i32>(10);
        let (output2, _) = create_test_channels::<i32>(10);

        let deconstructor = PipelineSeriesDeconstructor::new(input, [output1, output2]);

        assert_eq!(deconstructor.get_num_inputs(), 1);
        assert_eq!(deconstructor.get_num_outputs(), 2);
        assert!(!deconstructor.has_initial_state());
        assert_eq!(deconstructor.get_run_model(), RunModel::Communicator);
    }

    #[test]
    fn test_get_successors() {
        let (_, input) = create_test_channels::<Vec<i32>>(10);
        let (output1, _) = create_test_channels(10);
        let (output2, _) = create_test_channels(10);

        let deconstructor = PipelineSeriesDeconstructor::new(input, [output1, output2]);

        let successors = deconstructor.get_successors();
        assert_eq!(successors.len(), 2);
        assert!(successors.contains(&1));
    }

    #[test]
    #[should_panic(expected = "Series deconstructor should not have initial state")]
    fn test_load_initial_state_panics() {
        let (_, input) = create_test_channels::<Vec<i32>>(10);
        let (output1, _) = create_test_channels(10);
        let (output2, _) = create_test_channels(10);

        let mut deconstructor = PipelineSeriesDeconstructor::new(input, [output1, output2]);

        deconstructor.load_initial_state();
    }

    #[tokio::test]
    async fn test_run_senders() {
        let (mut tx, input) = create_test_channels(10);
        let (output1, mut rx1) = create_test_channels(10);
        let (output2, mut rx2) = create_test_channels(10);

        let mut deconstructor = PipelineSeriesDeconstructor::new(input, [output1, output2]);

        // Send test data
        let test_vec = vec![1, 2, 3];
        tx.send(test_vec).await.unwrap();

        // Run the deconstructor
        let result = deconstructor.run_senders(0).await;
        assert!(result.is_some());

        // Verify both outputs received all items
        for expected_value in [1, 2, 3] {
            let received1 = rx1.recv_async().await.unwrap();
            let received2 = rx2.recv_async().await.unwrap();

            assert_eq!(received1, expected_value);
            assert_eq!(received2, expected_value);
        }
    }

    #[tokio::test]
    async fn test_run_senders_empty_vec() {
        let (mut tx, input) = create_test_channels::<Vec<i32>>(10);
        let (output1, mut rx1) = create_test_channels::<i32>(10);
        let (output2, mut rx2) = create_test_channels::<i32>(10);

        let mut deconstructor = PipelineSeriesDeconstructor::new(input, [output1, output2]);

        // Send empty vector
        tx.send(vec![]).await.unwrap();

        // Run the deconstructor
        let result = deconstructor.run_senders(0).await;
        assert!(result.is_some());

        // Verify no data was sent to outputs
        tokio::select! {
            _ = rx1.recv_async() => panic!("Should not receive data"),
            _ = rx2.recv_async() => panic!("Should not receive data"),
            _ = tokio::time::sleep(tokio::time::Duration::from_millis(10)) => {}
        }
    }
}
