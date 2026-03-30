use crate::pipeline::communication_layer::comms_core::{
    iterative_send, WrappedReceiver, WrappedSender,
};
use crate::pipeline::communication_layer::data_management::{BufferArray, DataWrapper};
use crate::pipeline::construction_layer::node_types::node_traits::{CollectibleNode, RunModel};
use crate::pipeline::construction_layer::pipeline_traits::Sharable;


pub struct PipelineSeriesReconstructor<I: Sharable, const NO: usize, const NR: usize> {
    // need to have a buuilder struct that wraps in identification info to make the graph after
    input: WrappedReceiver<I>,
    output: [WrappedSender<BufferArray<I, NR>>; NO],
    receive_demands: usize,
    satiated_edges: [usize; NO],
    buffered_input: DataWrapper<BufferArray<I, NR>>,
}

impl<I: Sharable, const NO: usize, const NR: usize> PipelineSeriesReconstructor<I, NO, NR> {
    pub fn new(
        input: WrappedReceiver<I>,
        output: [WrappedSender<BufferArray<I, NR>>; NO],
        receive_demands: usize,
    ) -> Self {
        Self {
            input,
            output,
            receive_demands,
            satiated_edges: [0; NO],
            buffered_input: DataWrapper::new(),
        }
    }
}


#[async_trait::async_trait]
impl<I: Sharable, const NO: usize, const NR: usize> CollectibleNode for PipelineSeriesReconstructor<I, NO, NR> {
    async fn run_senders(&mut self, _id: usize) -> Option<usize> {
        for idx in 0..self.receive_demands {
            match self.input.recv_async().await {
                Some(mut data) => {
                    data.swap(&mut self.buffered_input.read().read_mut()[idx]);
                    self.input.refill_buffer(data);
                },
                None => return None
            }; // unwrap is okay because this assumes all predecessors are ready
        }
        
        iterative_send(&mut self.output, &mut self.satiated_edges, &mut self.buffered_input).await.ok()
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
        panic!("Series reconstructor does not support initial state")
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
    fn test_pipeline_series_reconstructor_new() {
        let (_, input) = create_test_channels::<i32>(10);
        let (output1, _) = create_test_channels(10);
        let (output2, _) = create_test_channels(10);

        let reconstructor = PipelineSeriesReconstructor::new(input, [output1, output2], 3);

        assert_eq!(reconstructor.receive_demands, 3);
        assert_eq!(reconstructor.get_num_inputs(), 1);
        assert_eq!(reconstructor.get_num_outputs(), 2);
        assert!(!reconstructor.has_initial_state());
        assert_eq!(reconstructor.get_run_model(), RunModel::Communicator);
    }

    #[test]
    fn test_get_successors() {
        let (_, input) = create_test_channels::<i32>(10);
        let (output1, _) = create_test_channels(10);
        let (output2, _) = create_test_channels(10);

        let reconstructor = PipelineSeriesReconstructor::new(input, [output1, output2], 2);

        let successors = reconstructor.get_successors();
        assert_eq!(successors.len(), 2);
        assert!(successors.contains(&1));
    }

    #[test]
    #[should_panic(expected = "Series reconstructor does not support initial state")]
    fn test_load_initial_state_panics() {
        let (_, input) = create_test_channels::<i32>(10);
        let (output1, _) = create_test_channels(10);
        let (output2, _) = create_test_channels(10);

        let mut reconstructor = PipelineSeriesReconstructor::new(input, [output1, output2], 1);

        reconstructor.load_initial_state();
    }

    #[tokio::test]
    async fn test_run_senders() {
        let (mut tx, input) = create_test_channels(10);
        let (output1, mut rx1) = create_test_channels(10);
        let (output2, mut rx2) = create_test_channels(10);

        let mut reconstructor = PipelineSeriesReconstructor::new(input, [output1, output2], 2);

        // Send test data
        tx.send(1).await.unwrap();
        tx.send(2).await.unwrap();

        // Run the reconstructor
        let result = reconstructor.run_senders(0).await;
        assert!(result.is_some());

        // Verify both outputs received the data
        let received1 = rx1.recv_async().await.unwrap();
        let received2 = rx2.recv_async().await.unwrap();

        assert_eq!(received1, vec![1, 2]);
        assert_eq!(received2, vec![1, 2]);
    }
}
