use crate::pipeline::communication_layer::comms_core::{
    iterative_send, WrappedReceiver, WrappedSender,
};
use crate::pipeline::construction_layer::node_types::node_traits::{CollectibleNode, RunModel};
use crate::pipeline::construction_layer::node_types::pipeline_step::PipelineStep;
use crate::pipeline::construction_layer::pipeline_traits::Sharable;
use async_trait::async_trait;
use futures::future::join_all;
use std::fmt::Debug;
use std::array;
use crate::pipeline::communication_layer::data_management::DataWrapper;


pub struct PipelineNode<I: Sharable, O: Sharable, const NI: usize, const NO: usize> {
    // need to have a buuilder struct that wraps in identification info to make the graph after
    input: [WrappedReceiver<I>; NI],
    output: [WrappedSender<O>; NO],
    satiated_edges: [usize; NO],
    step: Box<dyn PipelineStep<I, O, NI>>,
    initial_state: Option<O>,
    buffered_data: DataWrapper<O>,
    output_ready: bool,
    run_model: RunModel,
}
impl<I: Sharable, O: Sharable, const NI: usize, const NO: usize> PipelineNode<I, O, NI, NO> {
    pub fn new(
        step: Box<dyn PipelineStep<I, O, NI>>,
        input: Vec<WrappedReceiver<I>>,
        output: Vec<WrappedSender<O>>,
        initial_state: Option<O>,
        run_model: RunModel,
    ) -> PipelineNode<I, O, NI, NO> {
        assert_eq!(
            input.len(),
            NI,
            "Number of inputs must match the number of inputs in the step"
        );
        assert_eq!(
            output.len(),
            NO,
            "Number of outputs must match the number of outputs in the step"
        );
        let input: [WrappedReceiver<I>; NI] = input.try_into().unwrap();
        let output: [WrappedSender<O>; NO] = output.try_into().unwrap();

        match run_model.clone() {
            RunModel::Communicator => panic!("A node cannot use the communicator compute model"),
            _ => (),
        };
        
        let (output_ready, buffered_data) = match initial_state.clone() {
            Some(val) => (true, DataWrapper::new_with_value(val)),
            None => (false, DataWrapper::new())
        };
        PipelineNode {
            input,
            output,
            step,
            buffered_data,
            initial_state,
            satiated_edges: [0; NO],
            output_ready,
            run_model,
        }
    }

    fn receive_input(&mut self) -> [DataWrapper<I>; NI] {
        let input: [DataWrapper<I>; NI] = std::array::from_fn(|i| {
            self.input[i].recv().unwrap() // error handling later when get work
        });
        input
    }

    async fn receive_input_async(&mut self) -> [DataWrapper<I>; NI] {
        let mut received = [None; NI];
        
        for (idx, receiver) in self.input.iter_mut().enumerate() {
            received[idx] = Some(receiver.recv_async().await);
        }
        
        received.into_iter().map(|x| x.expect("unexpected none")).collect()
    }

    fn execute_pipeline_step(&mut self, mut input_data: [DataWrapper<I>; NI]) -> Result<(), ()> {
        let res = self.step.run_cpu(&mut input_data, &mut self.buffered_data);
        for (receiver, data_wrapper) in self.input.iter_mut().zip(input_data) {
            receiver.refill_buffer(data_wrapper);
        }
        
        res
    }

    async fn execute_pipeline_step_io(&mut self, mut input_data: [DataWrapper<I>; NI]) -> Result<(), ()> {
        let res = self.step.run_io(&mut input_data, &mut self.buffered_data).await;
        for (receiver, data_wrapper) in self.input.iter_mut().zip(input_data) {
            receiver.refill_buffer(data_wrapper);
        }
        
        res
    }
}

#[async_trait]
impl<I: Sharable, O: Sharable, const NI: usize, const NO: usize> CollectibleNode
    for PipelineNode<I, O, NI, NO>
{
    async fn run_senders(&mut self, _id: usize) -> Option<usize> {
        if self.output_ready {
            iterative_send(&mut self.output, &mut self.satiated_edges, &mut self.buffered_data).await.ok()
        }
        else {
            None
        }
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
        match self.initial_state.clone() {
            Some(val) => self.buffered_data = DataWrapper::new_with_value(val),
            None => ()
        }
    }
    fn has_initial_state(&self) -> bool {
        self.initial_state.is_some()
    }
    fn get_num_inputs(&self) -> usize {
        NI
    }
    fn get_num_outputs(&self) -> usize {
        NO
    }

    fn is_ready_exec(&self) -> bool {
        self.input.iter().all(|x| x.channel_satiated())
    }

    fn get_successors(&self) -> Vec<usize> {
        self.output.iter().map(|x| *x.get_dest_id()).collect()
    }

    fn get_run_model(&self) -> RunModel {
        if self.output_ready {
            RunModel::Communicator
        }
        else {
            self.run_model
        }
    }

    fn call_thread_cpu(&mut self, _id: usize) {
        let received_data: [DataWrapper<I>; NI] = self.receive_input();
        let compute_result = self.execute_pipeline_step(received_data);

        self.output_ready = compute_result.is_ok() && !self.is_sink();
    }

    async fn call_thread_io(&mut self, _id: usize) {
        let received_data: [DataWrapper<I>; NI] = self.receive_input_async().await;
        let compute_result = self.execute_pipeline_step_io(received_data).await;

        self.output_ready = compute_result.is_ok() && !self.is_sink();
    }
}
