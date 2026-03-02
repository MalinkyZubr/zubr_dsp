use crate::pipeline::communication_layer::comms_core::channel_wrapped;
use crate::pipeline::construction_layer::node_builder::{
    BuildingNode, IntoWhat, PipelineBuildVector,
};
use crate::pipeline::construction_layer::node_types::pipeline_step::PipelineStep;
use crate::pipeline::construction_layer::pipeline_traits::{Sharable, Sink, Source, Unit};
use std::cell::RefCell;
use std::rc::Rc;



pub struct NodeBuilder<
    I: Sharable,
    O: Sharable,
    const NI: usize,
    const NO: usize,
    const VARIANT: IntoWhat,
> {
    node_predecessor: BuildingNode<I, O, NI, NO, VARIANT>,
    build_vector: Rc<RefCell<PipelineBuildVector>>
}
impl<I: Sharable, O: Sharable, const NI: usize, const NO: usize, const VARIANT: IntoWhat>
    NodeBuilder<I, O, NI, NO, VARIANT>
{
    pub fn get_node_id(&self) -> usize {
        self.node_predecessor.get_id()
    }

    pub fn attach_standard_io<F: Sharable, const NIN: usize, const NON: usize>(
        &mut self,
        name: String,
        step: impl PipelineStep<O, F, NIN> + 'static,
    ) -> NodeBuilder<O, F, NIN, NON, { IntoWhat::IoNode }> {
        self.attach_standard(name, step)
    }

    pub fn attach_standard_cpu<F: Sharable, const NIN: usize, const NON: usize>(
        &mut self,
        name: String,
        step: impl PipelineStep<O, F, NIN> + 'static,
    ) -> NodeBuilder<O, F, NIN, NON, { IntoWhat::CpuNode }> {
        self.attach_standard(name, step)
    }

    fn attach_standard<
        F: Sharable,
        const NIN: usize,
        const NON: usize,
        const N_VARIANT: IntoWhat,
    >(
        &mut self,
        name: String,
        step: impl PipelineStep<O, F, NIN> + 'static,
    ) -> NodeBuilder<O, F, NIN, NON, N_VARIANT> {
        // attach a step to the selected node (self) and create a thread
        // produce a successor node to continue the pipeline
        let mut new_node = BuildingNode::new(name, self.build_vector.borrow_mut().get_new_id());
        new_node.attach_step(Box::new(step));

        let (sender, receiver) = channel_wrapped::<O>(
            self.build_vector.borrow_mut().get_parameters().buff_size,
            self.node_predecessor.get_id(),
            new_node.get_id(),
        );

        self.node_predecessor.add_output(sender);
        new_node.add_input(receiver);

        NodeBuilder {
            node_predecessor: new_node,
            build_vector: self.build_vector.clone(),
        }
    }

    pub fn add_io_pipeline_sink(
        mut self,
        name: String,
        step: impl PipelineStep<O, (), 1> + Sink + 'static,
    ) -> Self {
        let new_sink = self.add_pipeline_sink::<{ IntoWhat::IoNode }>(name, step);
        self.build_vector
            .borrow_mut()
            .add_node(new_sink.build_io_node());
        self
    }

    pub fn add_cpu_pipeline_sink(
        mut self,
        name: String,
        step: impl PipelineStep<O, (), 1> + Sink + 'static,
    ) -> Self {
        let new_sink = self.add_pipeline_sink::<{ IntoWhat::CpuNode }>(name, step);
        self.build_vector
            .borrow_mut()
            .add_node(new_sink.build_cpu_node());
        self
    }

    fn add_pipeline_sink<const N_VARIANT: IntoWhat>(
        &mut self,
        name: String,
        step: impl PipelineStep<O, (), 1> + 'static + Sink,
    ) -> BuildingNode<O, (), 1, 0, N_VARIANT> {
        // End a linear pipeline branch, allowing the step itself to handle output to other parts of the program
        let mut new_node = BuildingNode::new(name, self.build_vector.borrow_mut().get_new_id());
        let (sender, receiver) = channel_wrapped::<O>(
            self.build_vector.borrow_mut().get_parameters().buff_size,
            self.node_predecessor.get_id(),
            new_node.get_id(),
        );
        new_node.attach_step(Box::new(step));

        self.node_predecessor.add_output(sender);
        new_node.add_input(receiver);

        new_node
    }

    pub fn add_io_pipeline_source<F: Sharable, const NON: usize>(
        name: String,
        source_step: impl PipelineStep<(), F, 0> + Source + 'static,
        build_vector: Rc<RefCell<PipelineBuildVector>>,
    ) -> NodeBuilder<(), F, 0, NON, { IntoWhat::IoNode }> {
        NodeBuilder::<(), F, 0, NON, { IntoWhat::IoNode }>::add_pipeline_source(
            name,
            source_step,
            build_vector,
        )
    }

    pub fn add_cpu_pipeline_source<F: Sharable, const NON: usize>(
        name: String,
        source_step: impl PipelineStep<(), F, 0> + Source + 'static,
        build_vector: Rc<RefCell<PipelineBuildVector>>,
    ) -> NodeBuilder<(), F, 0, NON, { IntoWhat::CpuNode }> {
        NodeBuilder::<(), F, 0, NON, { IntoWhat::CpuNode }>::add_pipeline_source(
            name,
            source_step,
            build_vector,
        )
    }

    fn add_pipeline_source<F: Sharable, const NON: usize, const N_VARIANT: IntoWhat>(
        name: String,
        source_step: impl PipelineStep<(), F, 0> + 'static + Source,
        build_vector: Rc<RefCell<PipelineBuildVector>>,
    ) -> NodeBuilder<(), F, 0, NON, N_VARIANT>
    where
        I: Unit,
    {
        // start a pipeline, allowing the step itself to handle input from other parts of the program
        let mut start_node = BuildingNode::new(name, build_vector.borrow_mut().get_new_id());
        start_node.attach_step(Box::new(source_step));

        NodeBuilder {
            node_predecessor: start_node,
            build_vector,
        }
    }

    pub fn create_cpu_joint_node<const NIN: usize, const NON: usize>(
        name: String,
        step: impl PipelineStep<I, O, NIN> + 'static,
        build_vector: Rc<RefCell<PipelineBuildVector>>,
    ) -> NodeBuilder<I, O, NIN, NON, { IntoWhat::CpuNode }> {
        NodeBuilder::<I, O, NIN, NON, { IntoWhat::CpuNode }>::create_joint_node(
            name,
            step,
            build_vector,
        )
    }

    fn create_joint_node<const NIN: usize, const NON: usize, const N_VARIANT: IntoWhat>(
        name: String,
        step: impl PipelineStep<I, O, NIN> + 'static,
        build_vector: Rc<RefCell<PipelineBuildVector>>,
    ) -> NodeBuilder<I, O, NIN, NON, N_VARIANT> {
        let mut new_node = BuildingNode::new(name, build_vector.borrow_mut().get_new_id());
        new_node.attach_step(Box::new(step));

        NodeBuilder {
            node_predecessor: new_node,
            build_vector,
        }
    }

    fn feed_into<F: Sharable, const NIF: usize, const NOF: usize, const JVARIANT: IntoWhat>(
        mut self,
        joint_builder: &mut NodeBuilder<O, F, NIF, NOF, JVARIANT>,
    ) -> Self {
        let (sender, receiver) = channel_wrapped::<O>(
            self.build_vector.borrow_mut().get_parameters().buff_size,
            self.get_node_id(),
            joint_builder.get_node_id(),
        );

        self.node_predecessor.add_output(sender);
        joint_builder.node_predecessor.add_input(receiver);

        self
    }
    pub fn feed_into_io<F: Sharable, const NIF: usize, const NOF: usize>(
        self,
        joint_builder: &mut NodeBuilder<O, F, NIF, NOF, { IntoWhat::IoNode }>,
    ) -> Self {
        self.feed_into(joint_builder)
    }

    pub fn feed_into_cpu<F: Sharable, const NIF: usize, const NOF: usize>(
        self,
        joint_builder: &mut NodeBuilder<O, F, NIF, NOF, { IntoWhat::CpuNode }>,
    ) -> Self {
        self.feed_into(joint_builder)
    }

    pub fn add_initial_state(mut self, initial_state: O) -> Self {
        // must perform loop detection at construction time to verify that all loops produce a base value to avoid starting lockups
        self.node_predecessor.add_initial_state(initial_state);
        self
    }

    pub fn attach_series_reconstructor<const NON: usize, const ND: usize> (
        &mut self,
        name: String,
    ) -> NodeBuilder<O, Vec<O>, 1, NON, { IntoWhat::ReconstructorNode }> {
        let mut new_node = BuildingNode::new(name, self.build_vector.borrow_mut().get_new_id());
        let (sender, receiver) = channel_wrapped::<O>(
            self.build_vector.borrow_mut().get_parameters().buff_size,
            self.node_predecessor.get_id(),
            new_node.get_id(),
        );

        self.node_predecessor.add_output(sender);
        new_node.add_input(receiver);
        new_node.set_required_input_count(ND);

        NodeBuilder {
            node_predecessor: new_node,
            build_vector: self.build_vector.clone(),
        }
    }
}

impl<I: Sharable, O: Sharable, const NI: usize, const NO: usize, const VARIANT: IntoWhat>
    NodeBuilder<I, Vec<O>, NI, NO, VARIANT>
{
    pub fn attach_interleaved_separator<const NON: usize>(
        &mut self,
        name: String,
    ) -> NodeBuilder<Vec<O>, Vec<O>, 1, NON, { IntoWhat::InterleaverNode }> {
        let mut new_node = BuildingNode::new(name, self.build_vector.borrow_mut().get_new_id());
        let (sender, receiver) = channel_wrapped::<Vec<O>>(
            self.build_vector.borrow_mut().get_parameters().buff_size,
            self.node_predecessor.get_id(),
            new_node.get_id(),
        );

        self.node_predecessor.add_output(sender);
        new_node.add_input(receiver);

        NodeBuilder {
            node_predecessor: new_node,
            build_vector: self.build_vector.clone(),
        }
    }

    pub fn attach_series_deconstructor<const NON: usize>(
        &mut self,
        name: String,
    ) -> NodeBuilder<Vec<O>, O, 1, NON, { IntoWhat::DeconstructorNode }> {
        let mut new_node = BuildingNode::new(name, self.build_vector.borrow_mut().get_new_id());
        let (sender, receiver) = channel_wrapped::<Vec<O>>(
            self.build_vector.borrow_mut().get_parameters().buff_size,
            self.node_predecessor.get_id(),
            new_node.get_id(),
        );

        self.node_predecessor.add_output(sender);
        new_node.add_input(receiver);

        NodeBuilder {
            node_predecessor: new_node,
            build_vector: self.build_vector.clone(),
        }
    }
}

impl<I: Sharable, O: Sharable, const NI: usize, const NO: usize>
    NodeBuilder<I, O, NI, NO, { IntoWhat::CpuNode }>
{
    pub fn submit_cpu(self) {
        self.build_vector
            .borrow_mut()
            .add_node(self.node_predecessor.build_cpu_node())
    }
}

impl<I: Sharable, O: Sharable, const NI: usize, const NO: usize>
    NodeBuilder<I, O, NI, NO, { IntoWhat::IoNode }>
{
    pub fn submit_io(self) {
        self.build_vector
            .borrow_mut()
            .add_node(self.node_predecessor.build_io_node())
    }
}

impl<T: Sharable, const NO: usize> NodeBuilder<T, Vec<T>, 1, NO, { IntoWhat::ReconstructorNode }> {
    pub fn submit_series_reconstructor(self) {
        self.build_vector
            .borrow_mut()
            .add_node(self.node_predecessor.build_reconstruct())
    }
}

impl<T: Sharable, const NO: usize>
    NodeBuilder<Vec<T>, Vec<T>, 1, NO, { IntoWhat::InterleaverNode }>
{
    pub fn submit_interleaved_separator(self) {
        self.build_vector
            .borrow_mut()
            .add_node(self.node_predecessor.build_interleave())
    }
}

impl<T: Sharable, const NO: usize> NodeBuilder<Vec<T>, T, 1, NO, { IntoWhat::DeconstructorNode }> {
    pub fn submit_series_deconstructor(self) {
        self.build_vector
            .borrow_mut()
            .add_node(self.node_predecessor.build_deconstruct())
    }
}
