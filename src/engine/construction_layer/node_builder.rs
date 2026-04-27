use crate::engine::communication_layer::comms_core::{WrappedReceiver, WrappedSender};
use crate::engine::communication_layer::data_management::BufferArray;
use crate::engine::structural::node_types::deconstruct::PipelineDeconstructorNode;
use crate::engine::structural::node_types::interleaving::PipelineDeInterleavingNode;
use crate::engine::structural::generic_pipeline_node::{CollectibleNode, RunModel};
use crate::engine::structural::node_types::standard::PipelineStandardNode;
use crate::engine::structural::generic_node_operation::PipelineNodeOp;
use crate::engine::structural::node_types::reconstruct::PipelineReconstructorNode;
use crate::engine::structural::pipeline_type_traits::Sharable;
use log::{debug, info};
use std::collections::HashMap;
use crate::engine::construction_layer::build_vector::PreparedNode;


pub struct BuildingNode<I: Sharable, O: Sharable, const NI: usize, const NO: usize> {
    id: usize,
    buffer_size: usize,
    name: String,
    step: Option<Box<dyn PipelineNodeOp<I, O, NI>>>,
    inputs: Vec<WrappedReceiver<I>>,
    outputs: Vec<WrappedSender<O>>,
    successors: HashMap<usize, usize>,
    demanded_input_count: usize,
    initial_state: Option<O>,
}
impl<I: Sharable, O: Sharable, const NI: usize, const NO: usize> BuildingNode<I, O, NI, NO> {
    pub fn new(name: String, id: usize, buffer_size: usize) -> Self {
        Self {
            id,
            buffer_size,
            name,
            step: None,
            inputs: Vec::new(),
            outputs: Vec::new(),
            successors: HashMap::new(),
            demanded_input_count: 1,
            initial_state: None,
        }
    }
    pub fn add_input(&mut self, receiver: WrappedReceiver<I>) {
        self.inputs.push(receiver)
    }

    pub fn add_output(&mut self, sender: WrappedSender<O>) {
        self.outputs.push(sender)
    }

    pub fn clear_outputs(&mut self) {
        self.outputs.clear();
    }

    pub fn attach_step(&mut self, step: Box<dyn PipelineNodeOp<I, O, NI>>) {
        self.step = Some(step);
    }

    pub fn set_required_input_count(&mut self, count: usize) {
        self.demanded_input_count = count;
    }

    pub fn get_id(&self) -> usize {
        self.id
    }
    pub fn get_name(&self) -> String {
        self.name.clone()
    }

    pub fn add_initial_state(&mut self, initial_state: O) {
        self.initial_state = Some(initial_state);
    }

    fn generate_node(&mut self, run_model: RunModel) -> PipelineStandardNode<I, O, NI, NO> {
        if self.step.is_none() {
            panic!("Cannot convert BuildingNode into CollectibleThread without a step attached")
        }
        let step = self.step.take().unwrap();

        debug!("Building node {}, {}", self.id, self.name);
        let new_node: PipelineStandardNode<I, O, NI, NO> = PipelineStandardNode::new(
            step,
            self.inputs.drain(..).collect(),
            self.outputs.drain(..).collect(),
            self.initial_state.clone(),
            run_model,
        );

        new_node
    }
}

impl<I: Sharable, O: Sharable, const NI: usize, const NO: usize> BuildingNode<I, O, NI, NO> {
    pub fn build_io_node(&mut self) -> PreparedNode {
        let id = self.id;
        let name = self.name.clone();
        let new_node = self.generate_node(RunModel::IO);

        PreparedNode::new(Box::new(new_node), id, name)
    }

    pub fn build_cpu_node(&mut self) -> PreparedNode {
        let id = self.id;
        let name = self.name.clone();
        let new_node = self.generate_node(RunModel::CPU);

        PreparedNode::new(Box::new(new_node), id, name)
    }
}

impl<T: Sharable, const NO: usize, const IBS: usize, const OBS: usize>
    BuildingNode<BufferArray<T, IBS>, BufferArray<T, OBS>, 1, NO>
where
    [(); (IBS % NO == 0) as usize - 1]:,
    [(); (IBS % OBS == 0) as usize - 1]:, // input buffer size should be perfectly divisible by NUM_CHANNELS
{
    fn into_interleaved_separator(mut self) -> PreparedNode {
        if self.step.is_some() {
            panic!("Cannot convert BuildingNode into Interleaver with a step attached. Read documentation on interleaver usage");
        }
        if self.initial_state.is_some() {
            panic!("Cannot convert BuildingNode into Interleaver with an initial state");
        }
        let interleaved_separator: PipelineDeInterleavingNode<T, NO, IBS, OBS> =
            PipelineDeInterleavingNode::new(
                self.inputs.remove(0),
                match self.outputs.try_into() {
                    Ok(out) => out,
                    Err(_) => panic!(
                        "Incorrect number of outputs for BuildingNode ID {}",
                        self.id
                    ),
                },
            );

        (self.id, self.name, Box::new(interleaved_separator))
    }

    pub fn build_interleave(self) -> PreparedNode {
        if self.inputs.len() != 1 {
            panic!("Incorrect number of inputs for BuildingNode ID {}", self.id);
        }
        self.into_interleaved_separator()
    }
}
impl<T: Sharable, const NO: usize, const ND: usize> BuildingNode<BufferArray<T, ND>, T, 1, NO> {
    fn into_series_deconstructor(mut self) -> PreparedNode {
        assert!(ND <= self.buffer_size, "WARNING! THE DECONSTRUCTOR RELIES ON MPSC BUFFER SIZE! IF IT IS TOO SMALL DECONSTRUCTOR WILL LOCK UP! INCREASE YOUR PIPELINE BUFFER SIZE!");
        if self.step.is_some() {
            panic!("Cannot convert BuildingNode into Interleaver with a step attached. Read documentation on interleaver usage");
        }
        if self.initial_state.is_some() {
            panic!("Cannot convert BuildingNode into Interleaver with an initial state");
        }
        let deconstructor_separator: PipelineDeconstructorNode<T, NO, ND> =
            PipelineDeconstructorNode::new(
                self.inputs.remove(0),
                match self.outputs.try_into() {
                    Ok(out) => out,
                    Err(_) => panic!(
                        "Incorrect number of outputs for BuildingNode ID {}",
                        self.id
                    ),
                },
            );

        (self.id, self.name, Box::new(deconstructor_separator))
    }

    pub fn build_deconstruct(self) -> PreparedNode {
        if self.inputs.len() != 1 {
            panic!("Incorrect number of inputs for BuildingNode ID {}", self.id);
        }
        self.into_series_deconstructor()
    }
}

impl<T: Sharable, const NO: usize, const NR: usize> BuildingNode<T, BufferArray<T, NR>, 1, NO> {
    fn generate_series_reconstructor(&mut self) -> PreparedNode {
        if self.step.is_some() {
            panic!("Cannot convert BuildingNode into Interleaver with a step attached. Read documentation on interleaver usage");
        }
        if self.initial_state.is_some() {
            panic!("Cannot convert BuildingNode into Interleaver with an initial state");
        }
        let intermediate = self.outputs.drain(..).collect();
        let reconstructor_separator: PipelineReconstructorNode<T, NO, NR> =
            PipelineReconstructorNode::new(
                self.inputs.remove(0),
                match intermediate.try_into() {
                    Ok(out) => out,
                    Err(_) => panic!(
                        "Incorrect number of outputs for BuildingNode ID {}",
                        self.id
                    ),
                },
            );

        (self.id, self.name.clone(), Box::new(reconstructor_separator))
    }

    pub fn build_reconstruct(self) -> PreparedNode {
        if self.inputs.len() != 1 {
            panic!("Incorrect number of inputs for BuildingNode ID {}", self.id);
        }
        self.into_series_reconstructor()
    }
}
