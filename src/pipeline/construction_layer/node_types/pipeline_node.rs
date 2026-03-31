use crate::pipeline::communication_layer::comms_core::{
    iterative_send, WrappedReceiver, WrappedSender,
};
use crate::pipeline::communication_layer::data_management::DataWrapper;
use crate::pipeline::construction_layer::node_types::node_traits::{CollectibleNode, RunModel};
use crate::pipeline::construction_layer::node_types::pipeline_step::PipelineStep;
use crate::pipeline::construction_layer::pipeline_traits::Sharable;
use async_trait::async_trait;
use std::mem;


pub struct PipelineNode<I: Sharable, O: Sharable, const NI: usize, const NO: usize> {
    step: Box<dyn PipelineStep<I, O, NI>>,

    input: [WrappedReceiver<I>; NI],
    output: [WrappedSender<O>; NO],
    satiated_edges: [usize; NO],

    initial_state: Option<O>,
    buffered_output: DataWrapper<O>,
    buffered_input: [DataWrapper<I>; NI],

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
        let input: [WrappedReceiver<I>; NI] = match input.try_into() {
            Ok(val) => val,
            Err(_) => panic!("Input type mismatch"),
        };
        let output: [WrappedSender<O>; NO] = match output.try_into() {
            Ok(val) => val,
            Err(_) => panic!("Input type mismatch"),
        };

        match run_model.clone() {
            RunModel::Communicator => panic!("A node cannot use the communicator compute model"),
            _ => (),
        };

        let (output_ready, buffered_data) = match initial_state.clone() {
            Some(val) => (true, DataWrapper::new_with_value(val)),
            None => (false, DataWrapper::new()),
        };
        PipelineNode {
            input,
            output,
            step,
            buffered_output: buffered_data,
            initial_state,
            satiated_edges: [0; NO],
            buffered_input: [DataWrapper::new(); NI],
            output_ready,
            run_model,
        }
    }

    fn receive_input(&mut self) -> Result<(), ()> {
        for (idx, receiver) in self.input.iter_mut().enumerate() {
            let data = receiver.recv();
            if data.is_none() {
                return Err(());
            }
            let mut data_wrapper = data.unwrap();
            mem::swap(&mut self.buffered_input[idx], &mut data_wrapper);
            receiver.refill_buffer(data_wrapper);
        }

        Ok(())
    }

    async fn receive_input_async(&mut self) -> Result<(), ()> {
        for (idx, receiver) in self.input.iter_mut().enumerate() {
            let data = receiver.recv_async().await;
            if data.is_none() {
                return Err(());
            }
            let mut data_wrapper = data.unwrap();
            mem::swap(&mut self.buffered_input[idx], &mut data_wrapper);
            receiver.refill_buffer(data_wrapper);
        }

        Ok(())
    }
}

#[async_trait]
impl<I: Sharable, O: Sharable, const NI: usize, const NO: usize> CollectibleNode
    for PipelineNode<I, O, NI, NO>
{
    async fn run_senders(&mut self, _id: usize) -> Option<usize> {
        let result;
        if self.output_ready {
            result = iterative_send(
                &mut self.output,
                &mut self.satiated_edges,
                &mut self.buffered_output,
            )
            .await
            .ok()
        } else {
            result = None
        }
        self.output_ready = false;
        result
    }
    fn check_nth_satiated_edge_id(&self, edge_index: usize) -> Option<usize> {
        if edge_index < NO {
            Some(self.satiated_edges[edge_index])
        } else {
            None
        }
    }
    fn load_initial_state(&mut self) {
        match self.initial_state.clone() {
            Some(val) => self.buffered_output = DataWrapper::new_with_value(val),
            None => (),
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
        } else {
            self.run_model
        }
    }

    fn call_thread_cpu(&mut self, _id: usize) {
        let _ = self.receive_input();
        let compute_result = self
            .step
            .run_cpu(&mut self.buffered_input, &mut self.buffered_output);

        self.output_ready = compute_result.is_ok() && !self.is_sink();
    }

    async fn call_thread_io(&mut self, _id: usize) {
        let _ = self.receive_input_async().await;
        let compute_result = self
            .step
            .run_io(&mut self.buffered_input, &mut self.buffered_output)
            .await;

        self.output_ready = compute_result.is_ok() && !self.is_sink();
    }
}
