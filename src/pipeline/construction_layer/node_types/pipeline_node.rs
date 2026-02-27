use crate::pipeline::communication_layer::comms_core::{
    iterative_send, WrappedReceiver, WrappedSender,
};
use crate::pipeline::construction_layer::node_types::node_traits::{CollectibleNode, RunModel};
use crate::pipeline::construction_layer::node_types::pipeline_step::PipelineStep;
use crate::pipeline::construction_layer::pipeline_traits::Sharable;
use async_trait::async_trait;
use std::fmt::Debug;
use futures::future::join_all;

#[derive(Debug)]
pub struct PipelineNode<I: Sharable, O: Sharable, const NI: usize, const NO: usize> {
    // need to have a buuilder struct that wraps in identification info to make the graph after
    input: [WrappedReceiver<I>; NI],
    output: [WrappedSender<O>; NO],
    step: Box<dyn PipelineStep<I, O, NI>>,
    initial_state: Option<O>,
    buffered_data: Option<O>,
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
        let input: [WrappedReceiver<I>; NI] = input.try_into().unwrap();
        let output: [WrappedSender<O>; NO] = output.try_into().unwrap();

        match run_model.clone() {
            RunModel::Communicator => panic!("A node cannot use the communicator compute model"),
            _ => (),
        };

        PipelineNode {
            input,
            output,
            step,
            buffered_data: initial_state.clone(),
            initial_state,
            run_model,
        }
    }

    fn receive_input(&mut self) -> [I; NI] {
        let input: [I; NI] = std::array::from_fn(|i| {
            self.input[i].recv().unwrap() // error handling later when get work
        });
        input
    }

    async fn receive_input_async(&mut self) -> [I; NI] {
        let futures = self.input.iter_mut().map(|r: &mut WrappedReceiver<I>| r.recv_async());
        let vec = join_all(futures).await.into_iter().flatten().collect::<Vec<I>>();

        vec.try_into().ok().unwrap()
    }

    fn execute_pipeline_step(&mut self, input_data: [I; NI]) -> Result<O, ()> {
        self.step.run_cpu(input_data)
    }

    async fn execute_pipeline_step_io(&mut self, input_data: [I; NI]) -> Result<O, ()> {
        self.step.run_io(input_data).await
    }
}

#[async_trait]
impl<I: Sharable, O: Sharable, const NI: usize, const NO: usize> CollectibleNode
    for PipelineNode<I, O, NI, NO>
{
    async fn run_senders(&mut self, id: usize) -> Option<Vec<usize>> {
        match self.buffered_data.take() {
            Some(output_data) => {
                iterative_send(&mut self.output, output_data).await.ok()
            }
            _ => None,
        }
    }

    fn load_initial_state(&mut self) {
        self.buffered_data = self.initial_state.clone()
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
        match &self.buffered_data {
            Some(_) => RunModel::Communicator,
            None => self.run_model.clone(),
        }
    }

    fn call_thread_cpu(&mut self, id: usize) {
        let received_data: [I; NI] = self.receive_input();
        let compute_result = self.execute_pipeline_step(received_data);

        match compute_result {
            Ok(value) => self.buffered_data = Some(value),
            Err(_) => (),
        }
    }

    async fn call_thread_io(&mut self, id: usize) {
        let received_data: [I; NI] = self.receive_input_async().await;
        let compute_result = self.execute_pipeline_step_io(received_data).await;

        match compute_result {
            Ok(value) => self.buffered_data = Some(value),
            Err(_) => (),
        }
    }
}
