use crate::pipeline::communication_layer::comms_core::channel_wrapped;
use crate::pipeline::construction_layer::node_builder::{BuildingNode, IntoWhat, PipelineBuildVector};
use crate::pipeline::construction_layer::node_types::pipeline_step::PipelineStep;
use crate::pipeline::construction_layer::pipeline_traits::{HasID, Sharable, Sink, Source, Unit};
use std::cell::RefCell;
use std::rc::Rc;
use crate::pipeline::construction_layer::node_types::reconstruct::PipelineSeriesReconstructor;
use crate::pipeline::construction_layer::node_types::deconstruct::PipelineSeriesDeconstructor;
use std::sync::atomic::AtomicUsize;


pub struct NodeBuilder<I: Sharable, O: Sharable, const NI: usize, const NO: usize, const Variant: IntoWhat> {
    node_predecessor: BuildingNode<I, O, NI, NO, Variant>,
    build_vector: Rc<RefCell<PipelineBuildVector>>,
}
impl<I: Sharable, O: Sharable, const NI: usize, const NO: usize, const Variant: IntoWhat> NodeBuilder<I, O, NI, NO, Variant> {
    pub fn get_node_id(&self) -> usize {
        self.node_predecessor.get_id()
    }

    pub fn attach_standard_io<F: Sharable, const NIN: usize, const NON: usize>(
        &mut self,
        name: String,
        step: impl PipelineStep<O, F, NIN> + 'static,
    ) -> NodeBuilder<O, F, NIN, NON, { IntoWhat::IO_NODE }> {
        self.attach_standard(name, step)
    }

    pub fn attach_standard_cpu<F: Sharable, const NIN: usize, const NON: usize>(
        &mut self,
        name: String,
        step: impl PipelineStep<O, F, NIN> + 'static,
    ) -> NodeBuilder<O, F, NIN, NON, { IntoWhat::CPU_NODE }> {
        self.attach_standard(name, step)
    }

    fn attach_standard<F: Sharable, const NIN: usize, const NON: usize, const NVariant: IntoWhat>(
        &mut self,
        name: String,
        step: impl PipelineStep<O, F, NIN> + 'static,
    ) -> NodeBuilder<O, F, NIN, NON, NVariant> {
        // attach a step to the selected node (self) and create a thread
        // produce a successor node to continue the pipeline
        let (mut sender, mut receiver) = channel_wrapped::<O>(
            self.build_vector
                .borrow_mut()
                .get_parameters()
                .backpressure_val,
        );
        let mut new_node = BuildingNode::new(name);
        sender.set_dest_id(new_node.get_id());
        receiver.set_source_id(self.node_predecessor.get_id());

        new_node.attach_step(Box::new(step));

        self.node_predecessor.add_output(sender);
        new_node.add_input(receiver);

        NodeBuilder {
            node_predecessor: new_node,
            build_vector: self.build_vector.clone(),
        }
    }

    pub fn add_io_pipeline_sink(mut self, name: String, step: impl PipelineStep<O, (), 1> + Sink + 'static) -> Self {
        let new_sink = self.add_pipeline_sink(name, step);
        self.build_vector.borrow_mut().add_node(new_sink.build_io_node());
        self
    }

    pub fn add_cpu_pipeline_sink(mut self, name: String, step: impl PipelineStep<O, (), 1> + Sink + 'static) -> Self {
        let new_sink = self.add_pipeline_sink(name, step);
        self.build_vector.borrow_mut().add_node(new_sink.build_cpu_node());
        self
    }

    fn add_pipeline_sink<const NVariant: IntoWhat>(
        &mut self,
        name: String,
        step: impl PipelineStep<O, (), 1> + 'static + Sink,
    ) -> BuildingNode<O, (), 1, 0, NVariant>
    {
        // End a linear pipeline branch, allowing the step itself to handle output to other parts of the program
        let (mut sender, mut receiver) = channel_wrapped::<O>(
            self.build_vector
                .borrow_mut()
                .get_parameters()
                .backpressure_val,
        );
        let mut new_node = BuildingNode::new(name);
        sender.set_dest_id(new_node.get_id());
        receiver.set_source_id(self.node_predecessor.get_id());
        new_node.attach_step(Box::new(step));

        self.node_predecessor.add_output(sender);
        new_node.add_input(receiver);

        new_node
    }

    pub fn add_io_pipeline_source<F: Sharable, const NON: usize>(
        name: String,
        source_step: impl PipelineStep<(), F, 0> + Source + 'static,
        build_vector: Rc<RefCell<PipelineBuildVector>>,
    ) -> NodeBuilder<(), F, 0, NON, { IntoWhat::IO_NODE }> {
        NodeBuilder::<(), F, 0, NON, { IntoWhat::IO_NODE }>::add_pipeline_source(name, source_step, build_vector)
    }

    pub fn add_cpu_pipeline_source<F: Sharable, const NON: usize>(
        name: String,
        source_step: impl PipelineStep<(), F, 0> + Source + 'static,
        build_vector: Rc<RefCell<PipelineBuildVector>>,
    ) -> NodeBuilder<(), F, 0, NON, { IntoWhat::CPU_NODE }> {
        NodeBuilder::<(), F, 0, NON, { IntoWhat::CPU_NODE }>::add_pipeline_source(name, source_step, build_vector)
    }

    fn add_pipeline_source<F: Sharable, const NON: usize, const NVariant: IntoWhat>(
        name: String,
        source_step: impl PipelineStep<(), F, 0> + 'static + Source,
        build_vector: Rc<RefCell<PipelineBuildVector>>,
    ) -> NodeBuilder<(), F, 0, NON, NVariant>
    where
        I: Unit,
    {
        // start a pipeline, allowing the step itself to handle input from other parts of the program
        let mut start_node = BuildingNode::new(name);
        start_node.attach_step(Box::new(source_step));

        NodeBuilder {
            node_predecessor: start_node,
            build_vector,
        }
    }

    pub fn create_cpu_joint_node<const NIN: usize, const NON: usize>(
        name: String,
        step: impl PipelineStep<I, O, NIN> + 'static,
        build_vector: Rc<RefCell<PipelineBuildVector>>
    ) -> NodeBuilder<I, O, NIN, NON, { IntoWhat::CPU_NODE }> {
        NodeBuilder::<I, O, NIN, NON, { IntoWhat::CPU_NODE }>::create_joint_node(name, step, build_vector)
    }

    fn create_joint_node<const NIN: usize, const NON: usize, const NVariant: IntoWhat>(
        name: String,
        step: impl PipelineStep<I, O, NIN> + 'static,
        build_vector: Rc<RefCell<PipelineBuildVector>>,
    ) -> NodeBuilder<I, O, NIN, NON, NVariant> {
        let mut new_node = BuildingNode::new(name);
        new_node.attach_step(Box::new(step));

        NodeBuilder {
            node_predecessor: new_node,
            build_vector,
        }
    }

    fn feed_into<F: Sharable, const NIF: usize, const NOF: usize, const JVariant: IntoWhat>(mut self, joint_builder: &mut NodeBuilder<O, F, NIF, NOF, JVariant>) -> Self {
        let (mut sender, mut receiver) = channel_wrapped::<O>(
            self.build_vector
                .borrow_mut()
                .get_parameters()
                .backpressure_val,
        );
        sender.set_dest_id(joint_builder.get_node_id());
        receiver.set_source_id(self.node_predecessor.get_id());

        self.node_predecessor.add_output(sender);
        joint_builder.node_predecessor.add_input(receiver);

        self
    }
    pub fn feed_into_io<F: Sharable, const NIF: usize, const NOF: usize>(mut self, joint_builder: &mut NodeBuilder<O, F, NIF, NOF, { IntoWhat::IO_NODE }>) -> Self {
        self.feed_into(joint_builder)
    }

    pub fn feed_into_cpu<F: Sharable, const NIF: usize, const NOF: usize>(mut self, joint_builder: &mut NodeBuilder<O, F, NIF, NOF, { IntoWhat::CPU_NODE }>) -> Self {
        self.feed_into(joint_builder)
    }

    pub fn add_initial_state(mut self, initial_state: O) -> Self {
        // must perform loop detection at construction time to verify that all loops produce a base value to avoid starting lockups
        self.node_predecessor.add_initial_state(initial_state);
        self
    }

    pub fn attach_series_reconstructor<const NON: usize>(&mut self, name: String) -> NodeBuilder<O, Vec<O>, 1, NON, { IntoWhat::RECONSTRUCTOR_NODE }> {
        let (mut sender, mut receiver) = channel_wrapped::<O>(
            self.build_vector
                .borrow_mut()
                .get_parameters()
                .backpressure_val,
        );
        let mut new_node = BuildingNode::new(name);
        sender.set_dest_id(new_node.get_id());
        receiver.set_source_id(self.node_predecessor.get_id());

        self.node_predecessor.add_output(sender);
        new_node.add_input(receiver);

        NodeBuilder {
            node_predecessor: new_node,
            build_vector: self.build_vector.clone(),
        }
    }
}

impl<I: Sharable, O: Sharable, const NI: usize, const NO: usize, const Variant: IntoWhat> NodeBuilder<I, Vec<O>, NI, NO, Variant> {
    pub fn attach_interleaved_separator<const NON: usize>(&mut self, name: String) -> NodeBuilder<Vec<O>, Vec<O>, 1, NON, { IntoWhat::INTERLEAVER_NODE }>
    {
        let (mut sender, mut receiver) = channel_wrapped::<Vec<O>>(
            self.build_vector
                .borrow_mut()
                .get_parameters()
                .backpressure_val,
        );
        let mut new_node = BuildingNode::new(name);
        sender.set_dest_id(new_node.get_id());
        receiver.set_source_id(self.node_predecessor.get_id());

        self.node_predecessor.add_output(sender);
        new_node.add_input(receiver);

        NodeBuilder {
            node_predecessor: new_node,
            build_vector: self.build_vector.clone(),
        }
    }

    pub fn attach_series_deconstructor<const NON: usize>(&mut self, name: String) -> NodeBuilder<Vec<O>, O, 1, NON, { IntoWhat::DECONSTRUCTOR_NODE }> {
        let (mut sender, mut receiver) = channel_wrapped::<Vec<O>>(
            self.build_vector
                .borrow_mut()
                .get_parameters()
                .backpressure_val,
        );
        let mut new_node = BuildingNode::new(name);
        sender.set_dest_id(new_node.get_id());
        receiver.set_source_id(self.node_predecessor.get_id());

        self.node_predecessor.add_output(sender);
        new_node.add_input(receiver);

        NodeBuilder {
            node_predecessor: new_node,
            build_vector: self.build_vector.clone(),
        }
    }
}

impl<I: Sharable, O: Sharable, const NI: usize, const NO: usize> NodeBuilder<I, O, NI, NO, { IntoWhat::CPU_NODE }> {
    pub fn submit_cpu(mut self) {
        self.build_vector.borrow_mut().add_node(self.node_predecessor.build_cpu_node())
    }
}

impl<I: Sharable, O: Sharable, const NI: usize, const NO: usize> NodeBuilder<I, O, NI, NO, { IntoWhat::IO_NODE }> {
    pub fn submit_io(mut self) {
        self.build_vector.borrow_mut().add_node(self.node_predecessor.build_io_node())
    }
}

impl<T: Sharable, const NO: usize> NodeBuilder<T, Vec<T>, 1, NO, { IntoWhat::RECONSTRUCTOR_NODE }> {
    pub fn submit_series_reconstructor(mut self) {
        self.build_vector.borrow_mut().add_node(self.node_predecessor.build_reconstruct())
    }
}

impl<T: Sharable, const NO: usize> NodeBuilder<Vec<T>, Vec<T>, 1, NO, { IntoWhat::INTERLEAVER_NODE }> {
    pub fn submit_interleaved_separator(mut self) {
        self.build_vector.borrow_mut().add_node(self.node_predecessor.build_interleave())
    }
}

impl<T: Sharable, const NO: usize> NodeBuilder<Vec<T>, T, 1, NO, { IntoWhat::DECONSTRUCTOR_NODE }> {
    pub fn submit_series_deconstructor(mut self) {
        self.build_vector.borrow_mut().add_node(self.node_predecessor.build_deconstruct())
    }
}


// pub trait PipelineRecipe<I: Sharable, O: Sharable> {
//     // allow to save and standardize macro-scale components that you dont wan tto repeatedly redefine
//     fn construct<FI: Sharable, FO: Sharable>(
//         origin_node: PipelineNode<FI, I>,
//         construction_queue: ConstructionQueue,
//     ) -> PipelineNode<O, FO>;
// }

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::pipeline::construction_layer::node_types::pipeline_step::PipelineStep;
//
//     struct Intermediary {}
//     impl PipelineStep<u32, u32> for Intermediary {
//         fn run(&mut self, input: u32) -> Result<ODFormat<u32>, ()> {
//             Ok(ODFormat::Standard(input + 1))
//         }
//     }
//
//     struct SourceD {}
//     impl PipelineStep<(), u32> for SourceD {
//         fn run(&mut self) -> Result<ODFormat<u32>, ()> {
//             Ok(ODFormat::Standard(1))
//         }
//     }
//     impl Source for SourceD {}
//
//     struct SinkD {}
//     impl PipelineStep<u32, ()> for SinkD {
//         fn run(&mut self, input: u32) -> Result<ODFormat<()>, ()> {
//             Ok(ODFormat::Standard(()))
//         }
//     }
//     impl Sink for SinkD {}
//
//     #[test]
//     fn test_basic_source_sink() {
//         let mut build_vector = Rc::new(RefCell::new(PipelineBuildVector::new(
//             PipelineParameters::new(1, 1, 128, 1, 1, 1),
//         )));
//
//         NodeBuilder::add_pipeline_source(
//             String::from("TestSource"),
//             SourceD {},
//             build_vector.clone(),
//         ).add_pipeline_sink(String::from("TestSink"), SinkD {});
//
//         let build_vector = build_vector.into_inner();
//         let vector_internal = build_vector.consume();
//         assert_eq!(vector_internal.len(), 2);
//
//         // Check internal values for the two nodes (source then sink)
//         let source_node: &BuildingNode<(), u32> = unsafe {
//             &*(((vector_internal[0].as_ref() as *const dyn CollectibleThreadPrecursor) as *const ())
//                 as *const BuildingNode<(), u32>)
//         };
//         let sink_node: &BuildingNode<u32, ()> = unsafe {
//             &*(((vector_internal[1].as_ref() as *const dyn CollectibleThreadPrecursor) as *const ())
//                 as *const BuildingNode<u32, ()>)
//         };
//
//         assert_eq!(source_node.name, "TestSource");
//         assert!(source_node.step.is_some());
//         assert_eq!(source_node.inputs.len(), 0);
//         assert_eq!(source_node.outputs.len(), 1);
//         assert_eq!(source_node.input_count, 1);
//         assert!(source_node.successors.is_empty());
//         assert!(source_node.initial_state.is_none());
//
//         assert_eq!(sink_node.name, "TestSink");
//         assert!(sink_node.step.is_some());
//         assert_eq!(sink_node.inputs.len(), 1);
//         assert_eq!(sink_node.outputs.len(), 0);
//         assert_eq!(sink_node.input_count, 1);
//         assert!(sink_node.successors.is_empty());
//         assert!(sink_node.initial_state.is_none());
//
//         assert_eq!(*source_node.outputs[0].get_dest_id(), sink_node.id);
//         assert_eq!(sink_node.inputs[0].get_source_id(), source_node.id);
//     }
// }
