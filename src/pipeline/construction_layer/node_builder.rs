use crate::pipeline::communication_layer::comms_core::{WrappedReceiver, WrappedSender};
use crate::pipeline::construction_layer::node_types::deconstruct::PipelineSeriesDeconstructor;
use crate::pipeline::construction_layer::node_types::interleaving::PipelineInterleavedSeparator;
use crate::pipeline::construction_layer::node_types::node_traits::{CollectibleNode, RunModel};
use crate::pipeline::construction_layer::node_types::pipeline_node::PipelineNode;
use crate::pipeline::construction_layer::node_types::pipeline_step::PipelineStep;
use crate::pipeline::construction_layer::node_types::reconstruct::PipelineSeriesReconstructor;
use crate::pipeline::construction_layer::pipeline_traits::Sharable;
use log::{debug, info};
use std::collections::HashMap;
use std::marker::ConstParamTy;
use std::sync::atomic::AtomicUsize;

#[derive(Clone)]
pub struct PipelineParameters {
    pub buff_size: usize,
}
impl PipelineParameters {
    pub fn new(buff_size: usize) -> Self {
        Self {
            buff_size: buff_size,
        }
    }
}

pub struct PipelineBuildVector {
    nodes: Vec<(usize, String, Box<dyn CollectibleNode>)>,
    parameters: PipelineParameters,
}
impl PipelineBuildVector {
    pub fn new(pipeline_parameters: PipelineParameters) -> Self {
        PipelineBuildVector {
            nodes: Vec::new(),
            parameters: pipeline_parameters,
        }
    }
    pub fn add_node(&mut self, node: (usize, String, Box<dyn CollectibleNode>)) {
        info!("Adding node {} {:?}", node.0, node.2.get_run_model());
        self.nodes.push(node);
        self.nodes.sort_by(|a, b| a.1.cmp(&b.1))
    }

    pub fn consume(self) -> Vec<(usize, String, Box<dyn CollectibleNode>)> {
        self.nodes
    }

    pub fn get_parameters(&self) -> &PipelineParameters {
        &self.parameters
    }
}

static NODE_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);
pub fn reset_node_id_counter() {
    NODE_ID_COUNTER.store(0, std::sync::atomic::Ordering::SeqCst);
}

#[derive(Clone, PartialEq, Eq, Debug, ConstParamTy)]
pub enum IntoWhat {
    CpuNode,
    IoNode,
    InterleaverNode,
    DeconstructorNode,
    ReconstructorNode,
}
impl IntoWhat {
    pub fn run_model(&self) -> RunModel {
        match self {
            Self::CpuNode | Self::InterleaverNode => RunModel::CPU,
            Self::IoNode => RunModel::IO,
            _ => RunModel::Communicator,
        }
    }
}

pub struct BuildingNode<
    I: Sharable,
    O: Sharable,
    const NI: usize,
    const NO: usize,
    const VARIANT: IntoWhat,
> {
    id: usize,
    name: String,
    step: Option<Box<dyn PipelineStep<I, O, NI>>>,
    inputs: Vec<WrappedReceiver<I>>,
    outputs: Vec<WrappedSender<O>>,
    successors: HashMap<usize, usize>,
    demanded_input_count: usize,
    initial_state: Option<O>,
}
impl<I: Sharable, O: Sharable, const NI: usize, const NO: usize, const VARIANT: IntoWhat>
    BuildingNode<I, O, NI, NO, VARIANT>
{
    pub fn new(name: String) -> Self {
        Self {
            id: NODE_ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst),
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

    pub fn attach_step(&mut self, step: Box<dyn PipelineStep<I, O, NI>>) {
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

    fn into_node(mut self) -> PipelineNode<I, O, NI, NO> {
        if self.step.is_none() {
            panic!("Cannot convert BuildingNode into CollectibleThread without a step attached")
        }
        let step = self.step.take().unwrap();

        debug!("Building node {}, {}", self.id, self.name);
        let new_node: PipelineNode<I, O, NI, NO> = PipelineNode::new(
            step,
            self.inputs,
            self.outputs,
            self.initial_state,
            VARIANT.run_model(),
        );

        new_node
    }
}

impl<I: Sharable, O: Sharable, const NI: usize, const NO: usize>
    BuildingNode<I, O, NI, NO, { IntoWhat::IoNode }>
{
    pub fn build_io_node(self) -> (usize, String, Box<dyn CollectibleNode>) {
        let id = self.id;
        let name = self.name.clone();
        let new_node = self.into_node();
        (id, name, Box::new(new_node))
    }
}

impl<I: Sharable, O: Sharable, const NI: usize, const NO: usize>
    BuildingNode<I, O, NI, NO, { IntoWhat::CpuNode }>
{
    pub fn build_cpu_node(self) -> (usize, String, Box<dyn CollectibleNode>) {
        let id = self.id;
        let name = self.name.clone();
        let new_node = self.into_node();
        (id, name, Box::new(new_node))
    }
}

impl<T: Sharable, const NO: usize>
    BuildingNode<Vec<T>, Vec<T>, 1, NO, { IntoWhat::InterleaverNode }>
{
    fn into_interleaved_separator(mut self) -> (usize, String, Box<dyn CollectibleNode>) {
        if self.step.is_some() {
            panic!("Cannot convert BuildingNode into Interleaver with a step attached. Read documentation on interleaver usage");
        }
        if self.initial_state.is_some() {
            panic!("Cannot convert BuildingNode into Interleaver with an initial state");
        }
        let interleaved_separator: PipelineInterleavedSeparator<T, NO> =
            PipelineInterleavedSeparator::new(
                self.inputs.remove(0),
                self.outputs.try_into().unwrap(),
            );

        (self.id, self.name, Box::new(interleaved_separator))
    }

    pub fn build_interleave(self) -> (usize, String, Box<dyn CollectibleNode>) {
        if self.inputs.len() != 1 {
            panic!("Incorrect number of inputs for BuildingNode ID {}", self.id);
        }
        self.into_interleaved_separator()
    }
}
impl<T: Sharable, const NO: usize> BuildingNode<Vec<T>, T, 1, NO, { IntoWhat::DeconstructorNode }> {
    fn into_series_deconstructor(mut self) -> (usize, String, Box<dyn CollectibleNode>) {
        if self.step.is_some() {
            panic!("Cannot convert BuildingNode into Interleaver with a step attached. Read documentation on interleaver usage");
        }
        if self.initial_state.is_some() {
            panic!("Cannot convert BuildingNode into Interleaver with an initial state");
        }
        let deconstructor_separator: PipelineSeriesDeconstructor<T, NO> =
            PipelineSeriesDeconstructor::new(
                self.inputs.remove(0),
                self.outputs.try_into().unwrap(),
            );

        (self.id, self.name, Box::new(deconstructor_separator))
    }

    pub fn build_deconstruct(self) -> (usize, String, Box<dyn CollectibleNode>) {
        if self.inputs.len() != 1 {
            panic!("Incorrect number of inputs for BuildingNode ID {}", self.id);
        }
        self.into_series_deconstructor()
    }
}

impl<T: Sharable, const NO: usize> BuildingNode<T, Vec<T>, 1, NO, { IntoWhat::ReconstructorNode }> {
    fn into_series_reconstructor(mut self) -> (usize, String, Box<dyn CollectibleNode>) {
        if self.step.is_some() {
            panic!("Cannot convert BuildingNode into Interleaver with a step attached. Read documentation on interleaver usage");
        }
        if self.initial_state.is_some() {
            panic!("Cannot convert BuildingNode into Interleaver with an initial state");
        }
        let reconstructor_separator: PipelineSeriesReconstructor<T, NO> =
            PipelineSeriesReconstructor::new(
                self.inputs.remove(0),
                self.outputs.try_into().unwrap(),
                self.demanded_input_count,
            );

        (self.id, self.name, Box::new(reconstructor_separator))
    }

    pub fn build_reconstruct(self) -> (usize, String, Box<dyn CollectibleNode>) {
        if self.inputs.len() != 1 {
            panic!("Incorrect number of inputs for BuildingNode ID {}", self.id);
        }
        self.into_series_reconstructor()
    }
}
