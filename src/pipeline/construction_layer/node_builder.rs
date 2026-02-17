use std::marker::ConstParamTy;
use std::cell::RefCell;
use crate::pipeline::construction_layer::pipeline_traits::Sharable;
use crate::pipeline::communication_layer::comms_core::{WrappedReceiver, WrappedSender};
use crate::pipeline::construction_layer::node_types::pipeline_node::{CPUCollectibleThread, CollectibleThread, IOCollectibleThread, PipelineNode};
use crate::pipeline::construction_layer::node_types::pipeline_step::PipelineStep;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::AtomicUsize;
use crate::pipeline::construction_layer::node_types::deconstruct::PipelineSeriesDeconstructor;
use crate::pipeline::construction_layer::node_types::interleaving::PipelineInterleavedSeparator;
use crate::pipeline::construction_layer::node_types::reconstruct::PipelineSeriesReconstructor;


#[derive(Clone)]
pub struct PipelineParameters {
    pub retries: usize,
    pub timeout: u64,
    pub max_infrastructure_errors: usize,
    pub max_compute_errors: usize,
    pub unchanged_state_time: u64,
    pub backpressure_val: usize,
}
impl PipelineParameters {
    pub fn new(
        retries: usize,
        timeout: u64,
        backpressure_val: usize,
        max_infrastructure_errors: usize,
        max_compute_errors: usize,
        unchanged_state_time: u64,
    ) -> PipelineParameters {
        Self {
            retries,
            timeout,
            backpressure_val,
            max_compute_errors,
            max_infrastructure_errors,
            unchanged_state_time,
        }
    }
}


pub struct PipelineBuildVector {
    nodes: Vec<(PipelineNodeType, HashMap<usize, usize>)>,
    parameters: PipelineParameters,
}
impl PipelineBuildVector {
    pub fn new(pipeline_parameters: PipelineParameters) -> Self {
        PipelineBuildVector {
            nodes: Vec::new(),
            parameters: pipeline_parameters,
        }
    }
    pub fn add_node(&mut self, node: (PipelineNodeType, HashMap<usize, usize>)) {
        self.nodes.push(node);
        self.nodes.sort_by(|a, b| a.0.get_id().cmp(&b.0.get_id()))
    }

    pub fn consume(self) -> Vec<(PipelineNodeType, HashMap<usize, usize>)> {
        self.nodes
    }

    pub fn get_parameters(&self) -> &PipelineParameters {
        &self.parameters
    }
}


static NODE_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);


#[derive(Clone, PartialEq, Eq, Debug)]
#[derive(ConstParamTy)]
pub enum IntoWhat {
    CPU_NODE,
    IO_NODE,
    INTERLEAVER_NODE,
    DECONSTRUCTOR_NODE,
    RECONSTRUCTOR_NODE,
}


pub struct BuildingNode<
    I: Sharable,
    O: Sharable,
    const NI: usize,
    const NO: usize,
    const Variant: IntoWhat,
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
impl<
        I: Sharable,
        O: Sharable,
        const NI: usize,
        const NO: usize,
        const Variant: IntoWhat,
    > BuildingNode<I, O, NI, NO, Variant>
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

    fn into_node(mut self) -> (PipelineNode<I, O, NI, NO>, HashMap<usize, usize>) {
        if self.step.is_none() {
            panic!("Cannot convert BuildingNode into CollectibleThread without a step attached")
        }
        let step = self.step.take().unwrap();

        let new_node: PipelineNode<I, O, NI, NO> = PipelineNode::new(
            step,
            self.inputs,
            self.outputs,
            self.initial_state,
        );

        (new_node, self.successors)
    }
}

impl<I: Sharable, O: Sharable, const NI: usize, const NO: usize> BuildingNode<I, O, NI, NO, {IntoWhat::IO_NODE}> {
    pub fn build_io_node(mut self) -> (PipelineNodeType, HashMap<usize, usize>) {
        let id = self.id;
        let (new_node, successors) = self.into_node();
        (PipelineNodeType::IO(id, Box::new(new_node)), successors)
    }
}

impl<I: Sharable, O: Sharable, const NI: usize, const NO: usize> BuildingNode<I, O, NI, NO, {IntoWhat::CPU_NODE}> {
    pub fn build_cpu_node(mut self) -> (PipelineNodeType, HashMap<usize, usize>) {
        let id = self.id;
        let (new_node, successors) = self.into_node();
        (PipelineNodeType::CPU(id, Box::new(new_node)), successors)
    }
}


impl<T: Sharable, const NO: usize> BuildingNode<Vec<T>, Vec<T>, 1, NO, {IntoWhat::INTERLEAVER_NODE}> {
    fn into_interleaved_separator(mut self) -> (PipelineNodeType, HashMap<usize, usize>) {
        if self.step.is_some() {
            panic!("Cannot convert BuildingNode into Interleaver with a step attached. Read documentation on interleaver usage");
        }
        if self.initial_state.is_some() {
            panic!("Cannot convert BuildingNode into Interleaver with an initial state");
        }
        let interleaved_separator: PipelineInterleavedSeparator<T, NO> = PipelineInterleavedSeparator::new(
            self.inputs.remove(0),
            self.outputs.try_into().unwrap(),
        );

        (PipelineNodeType::CPU(self.id, Box::new(interleaved_separator)), self.successors)
    }
    
    pub fn build_interleave(mut self) -> (PipelineNodeType, HashMap<usize, usize>) {
        if self.inputs.len() != 1 {
            panic!("Incorrect number of inputs for BuildingNode ID {}", self.id);
        }
        self.into_interleaved_separator()
    }
}
impl<
    T: Sharable,
    const NO: usize,
> BuildingNode<Vec<T>, T, 1, NO, {IntoWhat::DECONSTRUCTOR_NODE}> {
    fn into_series_deconstructor(mut self) -> (PipelineNodeType, HashMap<usize, usize>) {
        if self.step.is_some() {
            panic!("Cannot convert BuildingNode into Interleaver with a step attached. Read documentation on interleaver usage");
        }
        if self.initial_state.is_some() {
            panic!("Cannot convert BuildingNode into Interleaver with an initial state");
        }
        let deconstructor_separator: PipelineSeriesDeconstructor<T, NO> = PipelineSeriesDeconstructor::new(
            self.inputs.remove(0),
            self.outputs.try_into().unwrap(),
        );

        (PipelineNodeType::NOOP(self.id, Box::new(deconstructor_separator)), self.successors)
    }
    
    pub fn build_deconstruct(mut self) -> (PipelineNodeType, HashMap<usize, usize>) {
        if self.inputs.len() != 1 {
            panic!("Incorrect number of inputs for BuildingNode ID {}", self.id);
        }
        self.into_series_deconstructor()
    }
}

impl<
T: Sharable,
    const NO: usize,
> BuildingNode<T, Vec<T>, 1, NO, {IntoWhat::RECONSTRUCTOR_NODE}> {
    fn into_series_reconstructor(mut self) -> (PipelineNodeType, HashMap<usize, usize>) {
        if self.step.is_some() {
            panic!("Cannot convert BuildingNode into Interleaver with a step attached. Read documentation on interleaver usage");
        }
        if self.initial_state.is_some() {
            panic!("Cannot convert BuildingNode into Interleaver with an initial state");
        }
        let reconstructor_separator: PipelineSeriesReconstructor<T, NO> = PipelineSeriesReconstructor::new(
            self.inputs.remove(0),
            self.outputs.try_into().unwrap(),
            self.demanded_input_count
        );

        (PipelineNodeType::NOOP(self.id, Box::new(reconstructor_separator)), self.successors)
    }
    
    pub fn build_reconstruct(mut self) -> (PipelineNodeType, HashMap<usize, usize>) {
        if self.inputs.len() != 1 {
            panic!("Incorrect number of inputs for BuildingNode ID {}", self.id);
        }
        self.into_series_reconstructor()
    }
}


pub enum PipelineNodeType {
    CPU(usize, Box<dyn CPUCollectibleThread>),
    IO(usize, Box<dyn IOCollectibleThread>),
    NOOP(usize, Box<dyn CollectibleThread>)
}
impl PipelineNodeType {
    pub fn get_id(&self) -> usize {
        match self {
            PipelineNodeType::CPU(id, val) => *id,
            PipelineNodeType::IO(id, val) => *id,
            PipelineNodeType::NOOP(id, val) => *id
        }
    }
}
