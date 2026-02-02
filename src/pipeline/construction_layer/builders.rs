use crate::pipeline::api::{ConstructingPipeline, PipelineParameters};
use crate::pipeline::communication_layer::comms_core::{channel_wrapped, WrappedReceiver, WrappedSender};
use crate::pipeline::construction_layer::pipeline_node::{CollectibleThread, CollectibleThreadPrecursor, PipelineNode};
use crate::pipeline::construction_layer::pipeline_step::PipelineStep;
use crate::pipeline::construction_layer::pipeline_tap::PipelineTap;
use crate::pipeline::construction_layer::pipeline_traits::{HasID, Sharable, Sink, Source, Unit};
use crate::pipeline::orchestration_layer::pipeline::ConstructionQueue;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicU8, AtomicUsize};
use std::sync::{mpsc, Arc};

static NODE_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);


pub struct BuildingNode<I: Sharable, O: Sharable> {
    pub id: usize,
    pub name: String,
    pub step: Option<Box<dyn PipelineStep<I, O>>>,
    pub inputs: Vec<WrappedReceiver<I>>,
    pub outputs: Vec<WrappedSender<O>>,
    pub tap: Option<PipelineTap<I>>,
    pub successors: HashMap<usize, usize>, // (id, num executions)
    pub input_count: usize
}
impl<I: Sharable, O: Sharable> BuildingNode<I, O> {
    pub fn new() -> Self {
        BuildingNode {
            id: NODE_ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst),
            name: String::new(),
            step: None,
            inputs: Vec::new(),
            outputs: Vec::new(),
            tap: None,
            input_count: 1,
            successors: HashMap::new(),
        }
    }
}
impl<I: Sharable, O: Sharable> CollectibleThreadPrecursor for BuildingNode<I, O> {
    fn add_input<I: Sharable>(&mut self, receiver: WrappedReceiver<I>) {
        self.inputs.push(receiver)
    }

    fn add_output<O: Sharable>(&mut self, sender: WrappedSender<O>) {
        self.outputs.push(sender)
    }

    fn clear_outputs(&mut self) {
        self.outputs.clear();
    }

    fn attach_step<I: Sharable, O: Sharable>(&mut self, step: impl PipelineStep<I, O>) {
        self.step = Some(Box::new(step));
    }

    fn attach_tap<I: Sharable>(&mut self, tap: PipelineTap<I>) {
        self.tap = Some(tap);
    }

    fn set_required_input_count(&mut self, count: usize) {
        self.input_count = count;
    }

    fn set_name(&mut self, name: String) {
        self.name = name;
    }

    fn get_id(&self) -> usize {
        self.id
    }
    fn into_collectible_thread<I: Sharable, O: Sharable>(&mut self) -> Box<dyn CollectibleThread> {
        if self.step.is_none() { 
            panic!("Cannot convert BuildingNode into CollectibleThread without a step attached")
        }
        let step = self.step.take().unwrap();
        let new_node: PipelineNode<I, O> = PipelineNode::new(step, self.inputs.take(), self.outputs.take(), self.tap.take());
        Box::new(
            
        )
    }
}

fn silly(mut stuff: Vec<Box<dyn CollectibleThreadPrecursor>>) {
    let mut out: Vec<Box<dyn CollectibleThread>> = Vec::new();
    for node in stuff {
        out.push(node.into_collectible_thread());
    }
}

pub struct NodeBuilder<I: Sharable, O: Sharable> {
    node: Box<dyn CollectibleThreadPrecursor>,
    build_vector: Rc<RefCell<Vec<Box<dyn CollectibleThreadPrecursor>>>>,
    parameters: PipelineParameters,
}
impl<I: Sharable, O: Sharable> NodeBuilder<I, O> {
    pub fn attach<F: Sharable>(
        mut self,
        name: String,
        step: impl PipelineStep<I, O> + 'static,
        critical_channel: bool,
    ) -> NodeBuilder<O, F> {
        // attach a step to the selected node (self) and create a thread
        // produce a successor node to continue the pipeline
        let (mut sender, mut receiver) = channel_wrapped::<O>(self.parameters.backpressure_val);
        let mut successor: Box<dyn CollectibleThreadPrecursor> = Box::new(BuildingNode::new());
        sender.set_dest_id(successor.get_id());
        receiver.set_source_id(self.node.get_id());

        self.node.set_name(name);

        self.node.add_output(sender);
        successor.add_input(receiver);

        self.build_vector.push(self.node);

        NodeBuilder {
            node: successor,
            build_vector: self.build_vector,
            parameters: self.parameters
        }
    }

    pub fn cap_pipeline(mut self, name: String, step: impl PipelineStep<I, O> + 'static + Sink)
    where
        O: Unit,
    {
        // End a linear pipeline branch, allowing the step itself to handle output to other parts of the program
        self.node.set_name(name);
        self.node.clear_outputs();

        self.build_vector.push(self.node);
        self.build_vector
    }

    pub fn start_pipeline<F: Sharable>(
        name: String,
        source_step: impl PipelineStep<I, O> + 'static + Source,
        mut build_vector: Rc<RefCell<Vec<Box<dyn CollectibleThreadPrecursor>>>>,
        parameters: PipelineParameters,
    ) -> NodeBuilder<O, F>
    where
        I: Unit,
    {
        // start a pipeline, allowing the step itself to handle input from other parts of the program
        let (sender, receiver) = channel_wrapped::<O>(parameters.backpressure_val);

        let mut start_node = BuildingNode::new();
        start_node.set_name(name);
        start_node.add_output(sender);
        start_node.attach_step(Some(Box::new(source_step)));

        let mut successor: BuildingNode<O, F> = BuildingNode::new();
        successor.add_input(receiver);
        
        build_vector.push(start_node);

        NodeBuilder {
            node: successor,
            parameters,
            build_vector
        }
    }

    pub fn split_begin(mut self) -> SplitBuilder<I, O> {
        SplitBuilder {
            node: self.node,
            parameters: self.parameters,
            build_vector: self.build_vector,
        }
    }

    pub fn branch_end(mut self, joint_builder: &mut JointBuilder<I, O>) {
        match self.node.input {
            NodeReceiver::SI(receiver) => joint_builder.joint_add(receiver.extract_receiver()),
            NodeReceiver::Dummy => panic!("Cannot end branch with Dummy"),
            _ => panic!("Must end branch with single. This should be automatic behavior"),
        }
    }
}

pub fn joint_begin<JI: Sharable, JO: Sharable>(
    name: String,
    build_vector: Rc<RefCell<Vec<Box<dyn CollectibleThreadPrecursor>>>>,
    parameters: PipelineParameters
) -> JointBuilder<JI, JO> {
    // create a node marked as a joint which can take multiple input receivers. used to join multiple sub branches together (eg adder or something)
    let mut join_node: BuildingNode<JI, JO> = BuildingNode::new();
    join_node.set_name(name);
    JointBuilder {
        node: Box::new(join_node),
        build_vector,
        parameters
    }
}

pub struct SplitBuilder<I: Sharable, O: Sharable> {
    node: Box<dyn CollectibleThreadPrecursor>,
    build_vector: Rc<RefCell<Vec<Box<dyn CollectibleThreadPrecursor>>>>,
    parameters: PipelineParameters,
}
impl<I: Sharable, O: Sharable> SplitBuilder<I, O> {
    pub fn split_add<F: Sharable>(&mut self, name: String) -> NodeBuilder<O, F> {
        // equivalent of start_pipeline for a subbranch of a flow diagram. generates an entry in the split for the branch
        // returns the head of the new branch which can be attached to like a normal linear pipeline
        let (sender, receiver) = channel_wrapped::<O>(self.parameters.backpressure_val);

        let mut branch_node = BuildingNode::new();
        branch_node.set_name(name);
        self.node.add_output(sender);
        branch_node.add_input(receiver);

        let 
        successor.add_input(receiver);

        build_vector.push(start_node);

        NodeBuilder {
            node: successor,
            parameters,
            build_vector
        }
    }

    pub fn split_lock(self, step: impl PipelineStep<I, O> + 'static) {
        // submit the split to the thread pool, preventing any more branches from being added and making it computable
        let new_thread =
            PipelineThread::new(step, self.node, self.parameters.clone());
        self.construction_queue.push(new_thread);
    }
}

pub struct LazyJointInputBuilder<I: Sharable, O: Sharable> {
    node: Box<dyn CollectibleThreadPrecursor>,
    build_vector: Rc<RefCell<Vec<Box<dyn CollectibleThreadPrecursor>>>>,
    parameters: PipelineParameters,
}
impl<I: Sharable, O: Sharable> LazyJointInputBuilder<I, O> {
    pub fn joint_link_lazy(
        mut self,
        id: &'static str,
        step: impl PipelineStep<I, O>,
        source_node: NodeBuilder<I, O>,
    ) {
        // takes the final node of a branch and attaches it to a lazy node's input. You must still assign the lazy node input with joint_lazy_finalize
        self.node.set_id(id);

        match source_node.node.input {
            NodeReceiver::SI(receiver) => {
                self.node.input = NodeReceiver::SI(receiver);
                let new_thread = PipelineThread::new(
                    step,
                    self.node,
                    self.parameters.clone(),
                );
                self.construction_queue.push(new_thread);
            }
            _ => panic!("Feedback joint cannot handle multiple input previous node"),
        }
    }
}

pub struct JointBuilder<I: Sharable, O: Sharable> {
    node: Box<dyn CollectibleThreadPrecursor>,
    build_vector: Rc<RefCell<Vec<Box<dyn CollectibleThreadPrecursor>>>>,
    parameters: PipelineParameters,
}
impl<I: Sharable, O: Sharable> JointBuilder<I, O> {
    fn joint_add(&mut self, receiver: WrappedReceiver<I>) {
        // attach an input to a joint
        match &mut self.node.input {
            NodeReceiver::MI(node_receiver) => { node_receiver.add_receiver(receiver) }
            _ => panic!("Cannot add a joint input to a node which was not declared as a joint with joint_begin")
        }
    }

    pub fn joint_lock<F: Sharable>(
        mut self,
        step: impl PipelineStep<I, O> + 'static,
        critical_channel: bool,
    ) -> NodeBuilder<O, F> {
        match &mut self.node.input {
            NodeReceiver::MI(_) => {
                let channel_metadata: ChannelMetadata = ChannelMetadata::new(self.node.id.clone(), critical_channel);
                let (sender, receiver) = channel_wrapped(self.parameters.backpressure_val, channel_metadata);
                let mut successor: PipelineNode<O, F> = PipelineNode::new();

                self.node.output = NodeSender::SO(SingleSender::new(sender));
                successor.input = NodeReceiver::SI(SingleReceiver::new(
                    receiver,
                    self.parameters.timeout,
                    self.parameters.retries,
                ));

                let new_thread = PipelineThread::new(
                    step,
                    self.node,
                    self.parameters.clone(),
                );
                self.construction_queue.push(new_thread);

                NodeBuilder {
                    node: successor,
                    parameters: self.parameters.clone(),
                    construction_queue: self.construction_queue.clone(),
                    state: self.state.clone(),
                }
            }
            _ => panic!("To joint lock a node it must be declared as a joint"),
        }
    }

    pub fn joint_add_lazy<F: Sharable>(&mut self, critical_channel: bool) -> LazyJointInputBuilder<F, I> {
        // creates an empty placeholder node for a joint that can be made concrete later to facilitate feedback architecture
        let channel_metadata: ChannelMetadata = ChannelMetadata::new(self.node.id.clone(), critical_channel);
        let (sender, receiver) = channel_wrapped(self.parameters.backpressure_val, channel_metadata);
        match &mut self.node.input {
            NodeReceiver::MI(node_receiver) => {
                node_receiver.add_receiver(receiver);
            }
            _ => panic!("Cannot add lazy feedback node to a node which was not declared as a joint with joint_begin")
        };

        let mut lazy_node = PipelineNode::new();
        lazy_node.output = NodeSender::SO(SingleSender::new(sender));

        LazyJointInputBuilder {
            node: lazy_node,
            parameters: self.parameters.clone(),
            construction_queue: self.construction_queue.clone(),
            state: self.state.clone(),
        }
    }
}

pub trait PipelineRecipe<I: Sharable, O: Sharable> {
    // allow to save and standardize macro-scale components that you dont wan tto repeatedly redefine
    fn construct<FI: Sharable, FO: Sharable>(
        origin_node: PipelineNode<FI, I>,
        construction_queue: ConstructionQueue,
    ) -> PipelineNode<O, FO>;
}
