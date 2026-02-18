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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::communication_layer::comms_core::channel_wrapped;
    use crate::pipeline::construction_layer::node_types::pipeline_node::{CPUCollectibleThread, CollectibleThread, IOCollectibleThread, PipelineNode};

    struct AddOneStep;

    #[async_trait::async_trait]
    impl PipelineStep<i32, i32, 1> for AddOneStep {
        fn run_cpu(&mut self, input: [i32; 1]) -> Result<i32, ()> {
            Ok(input[0] + 1)
        }
    }

    #[test]
    fn pipeline_node_cpu_call_then_run_senders_sends_to_all_outputs() {
        let rt = tokio::runtime::Runtime::new().unwrap();

        let (mut in_tx, in_rx) = channel_wrapped::<i32>(8);

        let (out_tx0, mut out_rx0) = channel_wrapped::<i32>(8);
        let (out_tx1, mut out_rx1) = channel_wrapped::<i32>(8);

        let step: Box<dyn PipelineStep<i32, i32, 1>> = Box::new(AddOneStep);

        let mut node: PipelineNode<i32, i32, 1, 2> = PipelineNode::new(
            step,
            vec![in_rx],
            vec![out_tx0, out_tx1],
            None,
        );

        rt.block_on(async {
            in_tx.send(41).await.unwrap();
        });

        // IMPORTANT: this uses blocking_recv internally, so it must NOT run on a Tokio runtime thread.
        node.call_thread_cpu(0);

        let mut inc = 0usize;
        rt.block_on(async {
            node.run_senders(0, &mut inc).await;
        });

        assert_eq!(inc, 1);

        // Also blocking_recv internally -> keep outside runtime context.
        assert_eq!(out_rx0.recv().unwrap(), 42);
        assert_eq!(out_rx1.recv().unwrap(), 42);
    }

    #[test]
    fn pipeline_node_initial_state_is_reported_and_loadable() {
        let (_in_tx, in_rx) = channel_wrapped::<i32>(8);
        let (out_tx, _out_rx) = channel_wrapped::<i32>(8);

        let step: Box<dyn PipelineStep<i32, i32, 1>> = Box::new(AddOneStep);

        let mut node: PipelineNode<i32, i32, 1, 1> = PipelineNode::new(
            step,
            vec![in_rx],
            vec![out_tx],
            Some(123),
        );

        assert!(node.has_initial_state());

        // `new()` already primes `buffered_data` with the initial state, but `load_initial_state`
        // should also be safe to call and restore it.
        node.load_initial_state();
    }
}

