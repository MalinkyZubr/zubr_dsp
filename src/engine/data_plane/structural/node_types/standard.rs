use crate::engine::data_plane::communication_layer::comms_core::{
    iterative_send, WrappedReceiver, WrappedSender,
};
use crate::engine::data_plane::structural::generic_pipeline_node::{GenericNode, NodeState, RunModel};
use crate::engine::data_plane::structural::generic_node_operation::PipelineNodeOp;
use crate::engine::data_plane::structural::pipeline_type_traits::Sharable;
use async_trait::async_trait;
use std::mem;
use log::error;

pub struct PipelineStandardNode<I: Sharable, O: Sharable, const NI: usize, const NO: usize> {
    step: Box<dyn PipelineNodeOp<I, O, NI>>,

    input: [WrappedReceiver<I>; NI],
    output: [WrappedSender<O>; NO],
    satiated_edges: [usize; NO],

    initial_value: Option<O>,
    buffered_output: O,
    buffered_input: [I; NI],
    run_model: RunModel,
}
impl<I: Sharable, O: Sharable, const NI: usize, const NO: usize> PipelineStandardNode<I, O, NI, NO> {
    pub fn new(
        step: Box<dyn PipelineNodeOp<I, O, NI>>,
        input: Vec<WrappedReceiver<I>>,
        output: Vec<WrappedSender<O>>,
        initial_state: Option<O>,
        run_model: RunModel,
    ) -> PipelineStandardNode<I, O, NI, NO> where O: Default, I: Default {
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

        let buffered_data = match initial_state.clone() {
            Some(val) => val,
            None => O::default(),
        };
        PipelineStandardNode {
            input,
            output,
            step,
            buffered_output: buffered_data,
            initial_value: initial_state,
            satiated_edges: [0; NO],
            buffered_input: std::array::from_fn(|_| Default::default()),
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
impl<I: Sharable, O: Sharable, const NI: usize, const NO: usize> GenericNode
    for PipelineStandardNode<I, O, NI, NO>
{
    async fn run_senders(&mut self) -> Option<usize> {
        let result;
        result = iterative_send(
            &mut self.output,
            &mut self.satiated_edges,
            &mut self.buffered_output,
        )
        .await
        .ok();
        result
    }
    fn get_satiated_edges(&self, num_satiated: usize) -> &[usize] {
        if num_satiated > self.satiated_edges.len() {
            panic!("Number of satiated edges is greater than the number of outputs")
        }
        &self.satiated_edges[..num_satiated]
    }
    fn load_initial_value(&mut self) {
        match self.initial_value.clone() {
            Some(val) => self.buffered_output = val,
            None => (),
        }
    }
    fn has_initial_value(&self) -> bool {
        self.initial_value.is_some()
    }
    fn get_num_inputs(&self) -> usize {
        NI
    }
    fn get_num_outputs(&self) -> usize {
        NO
    }

    fn is_ready_exec(&self, state: NodeState) -> bool {
        match state {
            NodeState::Communicate => true,
            _ => self.input.iter().all(|x| x.channel_satiated())
        }
    }

    fn get_successors(&self) -> Vec<usize> {
        self.output.iter().map(|x| *x.get_dest_id()).collect()
    }
    
    fn get_predecessors(&self) -> Vec<usize> {
        self.input.iter().map(|x| *x.get_source_id()).collect()
    }

    fn get_run_model(&self) -> RunModel {
        self.run_model
    }

    fn call_thread_cpu(&mut self) -> Result<(), ()> {
        let _ = self.receive_input();
        let compute_result = self
            .step
            .run_cpu(&mut self.buffered_input, &mut self.buffered_output);

        compute_result
    }

    async fn call_thread_io(&mut self) -> Result<(), ()> {
        let _ = self.receive_input_async().await;
        let compute_result = self
            .step
            .run_io(&mut self.buffered_input, &mut self.buffered_output)
            .await;

        compute_result
    }

    fn next_state(&self, current_state: NodeState) -> NodeState {
        match current_state {
            NodeState::Communicate => {
                self.run_model.to_state()
            },
            NodeState::ExecIo | NodeState::ExecCpu => {
                if self.get_num_outputs() == 0 {
                    self.run_model.to_state()
                }
                else {
                    NodeState::Communicate
                }
            },
            NodeState::Stop => {
                self.initial_state()
            }
            _ => panic!("Invalid state"),
        }
    }

    fn initial_state(&self) -> NodeState {
        if self.has_initial_value() {
            NodeState::Communicate
        }
        else {
            self.run_model.to_state()
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    //
    //
    // fn setup() -> PipelineStandardNode<i32, i32, 1, 1> {
    //     let test_node = PipelineStandardNode::new()
    // }
    //
    #[test]
    fn test_state_machine() {

    }
}