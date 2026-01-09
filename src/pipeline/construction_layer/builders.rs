use crate::pipeline::communication_layer::comms_core::{WrappedReceiver, channel_wrapped, ChannelMetadata};
use crate::pipeline::communication_layer::comms_implementations::linear_comms::{
    SingleReceiver, SingleSender,
};
use crate::pipeline::communication_layer::comms_implementations::multichannel_comms::{
    MultichannelReceiver, MultichannelSender, Reassembler,
};
use crate::pipeline::communication_layer::comms_implementations::multiplexed_comms::{
    Demultiplexer, Multiplexer,
};
use crate::pipeline::communication_layer::comms_top_level::{NodeReceiver, NodeSender};
use crate::pipeline::interfaces::PipelineStep;
use crate::pipeline::orchestration_layer::pipeline::ConstructionQueue;
use crate::pipeline::pipeline_traits::{HasID, Sharable, Sink, Source, Unit};
use std::sync::atomic::{AtomicU8, AtomicUsize};
use std::sync::{mpsc, Arc};
use std::sync::mpmc::channel;
use crate::pipeline::api::{ConstructingPipeline, PipelineParameters, PipelineThread, ThreadStateSpace};
use crate::pipeline::construction_layer::pipeline_node::PipelineNode;

pub struct NodeBuilder<I: Sharable, O: Sharable> {
    node: PipelineNode<I, O>,
    construction_queue: ConstructionQueue,
    parameters: PipelineParameters,
    state: (Arc<AtomicU8>, mpsc::Sender<ThreadStateSpace>),
}
impl<I: Sharable, O: Sharable> NodeBuilder<I, O> {
    pub fn attach<F: Sharable>(
        mut self,
        id: &'static str,
        step: impl PipelineStep<I, O> + 'static,
        critical_channel: bool,
    ) -> NodeBuilder<O, F> {
        // attach a step to the selected node (self) and create a thread
        // produce a successor node to continue the pipeline
        let channel_metadata = ChannelMetadata::new(self.node.get_id(), critical_channel);
        let (sender, receiver) = channel_wrapped(self.parameters.backpressure_val, channel_metadata);
        let mut successor: PipelineNode<O, F> = PipelineNode::new();

        self.node.set_id(id); 

        self.node.output = NodeSender::SO(SingleSender::new(sender));
        successor.input = NodeReceiver::SI(SingleReceiver::new(
            receiver,
            self.parameters.timeout,
            self.parameters.retries,
        ));

        let new_thread =
            PipelineThread::new(step, self.node, self.parameters.clone());
        self.construction_queue.push(new_thread);

        NodeBuilder {
            node: successor,
            construction_queue: self.construction_queue,
            parameters: self.parameters,
            state: self.state,
        }
    }

    pub fn cap_pipeline(mut self, id: &'static str, step: impl PipelineStep<I, O> + 'static + Sink)
    where
        O: Unit,
    {
        // End a linear pipeline branch, allowing the step itself to handle output to other parts of the program
        self.node.set_id(id);

        self.node.output = NodeSender::Dummy;

        let new_thread = PipelineThread::new(step, self.node, self.parameters.clone());
        self.construction_queue.push(new_thread);
    }

    pub fn start_pipeline<F: Sharable>(
        start_id: &str,
        source_step: impl PipelineStep<I, O> + 'static + Source,
        pipeline: &ConstructingPipeline,
    ) -> NodeBuilder<O, F>
    where
        I: Unit,
    {
        let parameters = pipeline.get_cloned_parameters();
        let channel_metadata: ChannelMetadata = ChannelMetadata::new("START".to_string(), true);
        // start a pipeline, allowing the step itself to handle input from other parts of the program
        let (sender, receiver) = channel_wrapped(parameters.backpressure_val, channel_metadata);

        let start_node: PipelineNode<I, O> = PipelineNode {
            input: NodeReceiver::Dummy,
            output: NodeSender::SO(SingleSender::new(sender)),
            id: start_id.to_string(),
            tap: None,
        };

        let mut successor: PipelineNode<O, F> = PipelineNode::new();
        successor.input = NodeReceiver::SI(SingleReceiver::new(
            receiver,
            parameters.timeout,
            parameters.retries,
        ));

        let new_thread = PipelineThread::new(
            source_step,
            start_node,
            parameters.clone(),
        );
        pipeline.get_nodes().push(new_thread);

        NodeBuilder {
            node: successor,
            parameters,
            construction_queue: pipeline.get_nodes(),
            state: pipeline.get_state_communicators(),
        }
    }

    pub fn split_begin(mut self, id: &'static str) -> SplitBuilder<I, O> {
        // take the node outputted by a previous step in the builder and declare it as multiple out
        // allows the node to have multiple outputs appended
        self.node.set_id(id);
        let sender: MultichannelSender<O> = MultichannelSender::new();
        self.node.output = NodeSender::MO(sender);

        SplitBuilder {
            node: self.node,
            parameters: self.parameters,
            construction_queue: self.construction_queue,
            state: self.state.clone(),
        }
    }

    pub fn mutltiplexer_begin(
        mut self,
        id: &'static str,
        channel_selector: Arc<AtomicUsize>,
    ) -> MultiplexerBuilder<I, O> {
        self.node.set_id(id);
        let sender: Multiplexer<O> = Multiplexer::new(channel_selector);
        self.node.output = NodeSender::MUO(sender);

        MultiplexerBuilder {
            node: self.node,
            parameters: self.parameters,
            construction_queue: self.construction_queue,
            state: self.state,
        }
    }

    pub fn branch_end(mut self, joint_builder: &mut JointBuilder<I, O>) {
        match self.node.input {
            NodeReceiver::SI(receiver) => joint_builder.joint_add(receiver.extract_receiver()),
            NodeReceiver::Dummy => panic!("Cannot end branch with Dummy"),
            _ => panic!("Must end branch with single. This should be automatic behavior"),
        }
    }

    pub fn multiplex_branch_end(mut self, demultiplexer_builder: &mut DemultiplexerBuilder<I, O>) {
        match self.node.input {
            NodeReceiver::SI(receiver) => {
                demultiplexer_builder.demultiplexer_add(receiver.extract_receiver())
            }
            NodeReceiver::Dummy => panic!("Cannot end multiplexed branch with Dummy"),
            _ => panic!("Must end branch with single. This should be automatic behavior"),
        }
    }

    pub fn add_reassembler(mut self, reassemble_quantity: usize) -> Self {
        match self.node.input {
            NodeReceiver::SI(receiver) => {
                self.node.input = NodeReceiver::REA(Reassembler::new(
                    receiver.extract_receiver(),
                    reassemble_quantity,
                    self.parameters.timeout,
                    self.parameters.retries,
                ))
            }
            _ => panic!("Cannot set reassembler on this node, must be a single receiver node"),
        }

        self
    }
}

pub fn demultiplexer_begin<JI: Sharable, JO: Sharable>(
    id: &str,
    channel_selector: Arc<AtomicUsize>,
    pipeline: &ConstructingPipeline,
) -> DemultiplexerBuilder<JI, JO> {
    // create a node marked as a join which can take multiple input receivers. used to join multiple sub branches together (eg adder or something)
    let mut demultiplexer_node: PipelineNode<JI, JO> = PipelineNode {
        input: NodeReceiver::Dummy,
        output: NodeSender::Dummy,
        id: id.to_string(),
        tap: None,
    };
    let parameters = pipeline.get_cloned_parameters();
    demultiplexer_node.input = NodeReceiver::DMI(Demultiplexer::new(
        channel_selector,
        parameters.timeout,
        parameters.retries,
    ));

    DemultiplexerBuilder {
        node: demultiplexer_node,
        parameters,
        construction_queue: pipeline.get_nodes(),
        state: pipeline.get_state_communicators(),
    }
}

pub fn joint_begin<JI: Sharable, JO: Sharable>(
    id: &str,
    pipeline: &ConstructingPipeline,
) -> JointBuilder<JI, JO> {
    // create a node marked as a joint which can take multiple input receivers. used to join multiple sub branches together (eg adder or something)
    let mut joint_node: PipelineNode<JI, JO> = PipelineNode {
        input: NodeReceiver::Dummy,
        output: NodeSender::Dummy,
        id: id.to_string(),
        tap: None,
    };
    let parameters = pipeline.get_cloned_parameters();
    joint_node.input = NodeReceiver::MI(MultichannelReceiver::new(
        parameters.timeout,
        parameters.retries,
    ));

    JointBuilder {
        node: joint_node,
        parameters,
        construction_queue: pipeline.get_nodes(),
        state: pipeline.get_state_communicators(),
    }
}

pub fn joint_feedback_begin<I: Sharable, O: Sharable>(
    id: &str,
    pipeline: &ConstructingPipeline,
) -> JointBuilder<I, O> {
    // Since there is no convenient origin point for a joint used in feedback in the pattern, a standalone function is needed to support type inference
    let mut joint_node: PipelineNode<I, O> = PipelineNode {
        input: NodeReceiver::Dummy,
        output: NodeSender::Dummy,
        id: id.to_string(),
        tap: None,
    };
    let parameters = pipeline.get_cloned_parameters();
    joint_node.input = NodeReceiver::MI(MultichannelReceiver::new(
        parameters.timeout,
        parameters.retries,
    ));

    JointBuilder {
        node: joint_node,
        parameters,
        construction_queue: pipeline.get_nodes(),
        state: pipeline.get_state_communicators(),
    }
}

pub struct SplitBuilder<I: Sharable, O: Sharable> {
    node: PipelineNode<I, O>,
    construction_queue: ConstructionQueue,
    parameters: PipelineParameters,
    state: (Arc<AtomicU8>, mpsc::Sender<ThreadStateSpace>),
}
impl<I: Sharable, O: Sharable> SplitBuilder<I, O> {
    pub fn split_add<F: Sharable>(&mut self, critical_channel: bool) -> NodeBuilder<O, F> {
        // equivalent of start_pipeline for a subbranch of a flow diagram. generates an entry in the split for the branch
        // returns the head of the new branch which can be attached to like a normal linear pipeline
        match &mut self.node.output {
            NodeSender::MO(node_sender) => {
                let (split_sender, split_receiver) =
                    channel_wrapped(self.parameters.backpressure_val, ChannelMetadata::new(self.node.id.clone(), critical_channel));
                node_sender.add_sender(split_sender);

                let mut successor: PipelineNode<O, F> = PipelineNode::new();

                successor.input = NodeReceiver::SI(SingleReceiver::new(
                    split_receiver, // super bumpo <3
                    self.parameters.timeout,
                    self.parameters.retries,
                ));

                NodeBuilder {
                    node: successor,
                    parameters: self.parameters.clone(),
                    construction_queue: self.construction_queue.clone(),
                    state: self.state.clone(),
                }
            }
            _ => panic!(
                "To add a split branch you must declare a node as a splitter with split_begin!"
            ),
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
    node: PipelineNode<I, O>,
    construction_queue: ConstructionQueue,
    parameters: PipelineParameters,
    state: (Arc<AtomicU8>, mpsc::Sender<ThreadStateSpace>),
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
    node: PipelineNode<I, O>,
    construction_queue: ConstructionQueue,
    parameters: PipelineParameters,
    state: (Arc<AtomicU8>, mpsc::Sender<ThreadStateSpace>),
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
        critical_channel: bool
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
                
            },
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

pub struct MultiplexerBuilder<I: Sharable, O: Sharable> {
    node: PipelineNode<I, O>,
    construction_queue: ConstructionQueue,
    parameters: PipelineParameters,
    state: (Arc<AtomicU8>, mpsc::Sender<ThreadStateSpace>),
}
impl<I: Sharable, O: Sharable> MultiplexerBuilder<I, O> {
    pub fn multiplexer_add<F: Sharable>(&mut self, critical_channel: bool) -> NodeBuilder<O, F> {
        // equivalent of start_pipeline for a subbranch of a flow diagram. generates an entry in the split for the branch
        // returns the head of the new branch which can be attached to like a normal linear pipeline
        match &mut self.node.output {
            NodeSender::MUO(node_sender) => {
                let channel_metadata = ChannelMetadata::new(self.node.id.clone(), critical_channel);
                let (multiplexer_sender, multiplexer_receiver) = channel_wrapped(self.parameters.backpressure_val, channel_metadata);
                node_sender.add_sender(multiplexer_sender);

                let mut successor: PipelineNode<O, F> = PipelineNode::new();

                successor.input = NodeReceiver::SI(SingleReceiver::new(multiplexer_receiver, self.parameters.timeout, self.parameters.retries));
                NodeBuilder { node: successor, parameters: self.parameters.clone(), construction_queue: self.construction_queue.clone(), state: self.state.clone() }
            }
            _ => panic!("To add a multiplexer branch you must declare it as a multiplexer with multiplexer_start")
        }
    }
    pub fn multiplexer_lock(self, step: impl PipelineStep<I, O> + 'static) {
        // submit the split to the thread pool, preventing any more branches from being added and making it computable
        let new_thread =
            PipelineThread::new(step, self.node, self.parameters.clone());
        self.construction_queue.push(new_thread);
    }
}

pub struct DemultiplexerBuilder<I: Sharable, O: Sharable> {
    node: PipelineNode<I, O>,
    construction_queue: ConstructionQueue,
    parameters: PipelineParameters,
    state: (Arc<AtomicU8>, mpsc::Sender<ThreadStateSpace>),
}
impl<I: Sharable, O: Sharable> DemultiplexerBuilder<I, O> {
    fn demultiplexer_add(&mut self, receiver: WrappedReceiver<I>) {
        // attach an input to a joint
        match &mut self.node.input {
            NodeReceiver::DMI(node_receiver) => { node_receiver.add_receiver(receiver) }
            _ => panic!("Cannot add a demultiplexer input to a node which was not declared as a demultiplexer with demultiplexer_begin")
        }
    }

    pub fn demultiplexer_lock<F: Sharable>(
        mut self,
        step: impl PipelineStep<I, O> + 'static,
        critical_channel: bool
    ) -> NodeBuilder<O, F> {
        match &mut self.node.input {
            NodeReceiver::DMI(_) => {
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
}

pub trait PipelineRecipe<I: Sharable, O: Sharable> {
    // allow to save and standardize macro-scale components that you dont wan tto repeatedly redefine
    fn construct<FI: Sharable, FO: Sharable>(
        origin_node: PipelineNode<FI, I>,
        construction_queue: ConstructionQueue,
    ) -> PipelineNode<O, FO>;
}
