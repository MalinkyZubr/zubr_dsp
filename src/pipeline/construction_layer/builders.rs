use crate::pipeline::api::PipelineParameters;
use crate::pipeline::communication_layer::comms_core::{
    channel_wrapped, WrappedReceiver, WrappedSender,
};
use crate::pipeline::communication_layer::formats::ODFormat;
use crate::pipeline::construction_layer::pipeline_node::{BuildingNode, CollectibleThread, CollectibleThreadPrecursor, PipelineNode};
use crate::pipeline::construction_layer::pipeline_step::PipelineStep;
use crate::pipeline::construction_layer::pipeline_traits::{HasID, Sharable, Sink, Source, Unit};
use crate::pipeline::orchestration_layer::pipeline::ConstructionQueue;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::AtomicUsize;

static NODE_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

pub struct PipelineBuildVector {
    nodes: Vec<Box<dyn CollectibleThread>>,
    parameters: PipelineParameters,
}
impl PipelineBuildVector {
    pub fn new(pipeline_parameters: PipelineParameters) -> Self {
        PipelineBuildVector {
            nodes: Vec::new(),
            parameters: pipeline_parameters,
        }
    }
    pub fn add_node<I: Sharable, O: Sharable>(&mut self, node: BuildingNode<I, O>) {
        self.nodes.push(node);
    }

    pub fn consume(self) -> Vec<Box<dyn CollectibleThreadPrecursor>> {
        self.nodes
    }

    pub fn get_node(&mut self, id: usize) -> &mut Box<dyn CollectibleThreadPrecursor> {
        self.nodes.get_mut(id).unwrap()
    }

    pub fn get_parameters(&self) -> &PipelineParameters {
        &self.parameters
    }
}


pub struct NodeBuilder<I: Sharable, O: Sharable> {
    node_predecessor: usize,
    build_vector: Rc<RefCell<PipelineBuildVector>>,
}
impl<I: Sharable, O: Sharable> NodeBuilder<I, O> {
    pub fn get_node_id(&self) -> usize {
        self.node_predecessor
    }
    pub fn attach<F: Sharable>(
        &mut self,
        name: String,
        step: impl PipelineStep<I, O> + 'static,
    ) -> NodeBuilder<O, F> {
        // attach a step to the selected node (self) and create a thread
        // produce a successor node to continue the pipeline
        let (mut sender, mut receiver) = channel_wrapped::<O>(
            self.build_vector
                .get_mut()
                .get_parameters()
                .backpressure_val,
        );
        let mut new_node = BuildingNode::new();
        sender.set_dest_id(new_node.get_id());
        receiver.set_source_id(self.node_predecessor);

        new_node.set_name(name);
        new_node.attach_step(Box::new(step));

        self.build_vector
            .get_mut()
            .get_node(self.node_predecessor)
            .add_output(sender);
        new_node.add_input(receiver);

        let new_id = new_node.get_id();
        self.build_vector.get_mut().add(new_node);

        NodeBuilder {
            node_predecessor: new_id,
            build_vector: self.build_vector,
        }
    }

    pub fn add_pipeline_sink(
        mut self,
        name: String,
        step: impl PipelineStep<I, O> + 'static + Sink,
    ) -> Self where
        O: Unit,
    {
        // End a linear pipeline branch, allowing the step itself to handle output to other parts of the program
        let (mut sender, mut receiver) = channel_wrapped::<O>(
            self.build_vector
                .get_mut()
                .get_parameters()
                .backpressure_val,
        );
        let mut new_node = BuildingNode::new();
        sender.set_dest_id(new_node.get_id());
        receiver.set_source_id(self.node_predecessor);
        new_node.attach_step(Box::new(step));

        new_node.set_name(name);

        self.build_vector
            .get_mut()
            .get_node(self.node_predecessor)
            .add_output(sender);
        new_node.add_input(receiver);

        self.build_vector.get_mut().add_node(Box::new(new_node));
        self
    }

    pub fn add_pipeline_source<F: Sharable>(
        name: String,
        source_step: impl PipelineStep<I, O> + 'static + Source,
        mut build_vector: Rc<RefCell<PipelineBuildVector>>,
    ) -> NodeBuilder<O, F>
    where
        I: Unit,
    {
        // start a pipeline, allowing the step itself to handle input from other parts of the program
        let mut start_node = BuildingNode::new();
        start_node.set_name(name);
        start_node.attach_step(Box::new(source_step));

        let node_id = start_node.get_id();
        build_vector.get_mut().add_node(Box::new(start_node));

        NodeBuilder {
            node_predecessor: node_id,
            build_vector,
        }
    }
    pub fn create_joint_node(
        name: String,
        step: impl PipelineStep<I, O> + 'static,
        mut build_vector: Rc<RefCell<PipelineBuildVector>>,
    ) -> NodeBuilder<I, O> {
        let mut new_node = BuildingNode::new();
        new_node.set_name(name);
        new_node.attach_step(Box::new(step));

        let new_id = new_node.get_id();
        build_vector.get_mut().add_node(Box::new(new_node));

        NodeBuilder {
            node_predecessor: new_id,
            build_vector,
        }
    }
    pub fn feed_into(mut self, joint_builder: &mut NodeBuilder<I, O>) -> Self {
        let (mut sender, mut receiver) = channel_wrapped::<O>(
            self.build_vector
                .get_mut()
                .get_parameters()
                .backpressure_val,
        );
        sender.set_dest_id(joint_builder.get_node_id());
        receiver.set_source_id(self.node_predecessor);

        self.build_vector
            .get_mut()
            .get_node(self.node_predecessor)
            .add_output(sender);
        self.build_vector
            .get_mut()
            .get_node(joint_builder.get_node_id())
            .add_input(receiver);

        self
    }

    pub fn add_initial_state(mut self, initial_state: O) -> Self {
        // must perform loop detection at construction time to verify that all loops produce a base value to avoid starting lockups
        self.build_vector[self.node_predecessor].add_initial_state(initial_state);
        self
    }
}

pub trait PipelineRecipe<I: Sharable, O: Sharable> {
    // allow to save and standardize macro-scale components that you dont wan tto repeatedly redefine
    fn construct<FI: Sharable, FO: Sharable>(
        origin_node: PipelineNode<FI, I>,
        construction_queue: ConstructionQueue,
    ) -> PipelineNode<O, FO>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Intermediary {}
    impl PipelineStep<u32, u32> for Intermediary {
        fn run_SISO(&mut self, input: u32) -> Result<ODFormat<u32>, ()> {
            Ok(ODFormat::Standard(input + 1))
        }
    }

    struct SourceD {}
    impl PipelineStep<(), u32> for SourceD {
        fn run_DISO(&mut self) -> Result<ODFormat<u32>, ()> {
            Ok(ODFormat::Standard(1))
        }
    }
    impl Source for SourceD {}

    struct SinkD {}
    impl PipelineStep<u32, ()> for SinkD {
        fn run_SIDO(&mut self, input: u32) -> Result<ODFormat<()>, ()> {
            Ok(ODFormat::Standard(()))
        }
    }
    impl Sink for SinkD {}

    #[test]
    fn test_basic_source_sink() {
        let mut build_vector = Rc::new(RefCell::new(PipelineBuildVector::new(
            PipelineParameters::new(1, 1, 128, 1, 1, 1),
        )));

        NodeBuilder::add_pipeline_source(
            String::from("TestSource"),
            SourceD {},
            build_vector.clone(),
        ).add_pipeline_sink(String::from("TestSink"), SinkD {});

        let build_vector = build_vector.into_inner();
        let vector_internal = build_vector.consume();
        assert_eq!(vector_internal.len(), 2);

        // Check internal values for the two nodes (source then sink)
        let source_node: &BuildingNode<(), u32> = unsafe {
            &*(((vector_internal[0].as_ref() as *const dyn CollectibleThreadPrecursor) as *const ())
                as *const BuildingNode<(), u32>)
        };
        let sink_node: &BuildingNode<u32, ()> = unsafe {
            &*(((vector_internal[1].as_ref() as *const dyn CollectibleThreadPrecursor) as *const ())
                as *const BuildingNode<u32, ()>)
        };

        assert_eq!(source_node.name, "TestSource");
        assert!(source_node.step.is_some());
        assert_eq!(source_node.inputs.len(), 0);
        assert_eq!(source_node.outputs.len(), 1);
        assert_eq!(source_node.input_count, 1);
        assert!(source_node.successors.is_empty());
        assert!(source_node.initial_state.is_none());

        assert_eq!(sink_node.name, "TestSink");
        assert!(sink_node.step.is_some());
        assert_eq!(sink_node.inputs.len(), 1);
        assert_eq!(sink_node.outputs.len(), 0);
        assert_eq!(sink_node.input_count, 1);
        assert!(sink_node.successors.is_empty());
        assert!(sink_node.initial_state.is_none());

        assert_eq!(*source_node.outputs[0].get_dest_id(), sink_node.id);
        assert_eq!(sink_node.inputs[0].get_source_id(), source_node.id);
    }
}
