use crate::engine::communication_layer::comms_core::{
    channel_wrapped, WrappedReceiver, WrappedSender,
};
use crate::engine::communication_layer::data_management::BufferArray;
use crate::engine::construction_layer::node_build_vector::{NodeSubmissionClosure, PipelineBuildVector};
use crate::engine::construction_layer::unfinished_node::UnfinishedNode;
use crate::engine::structural::generic_node_operation::{
    PipelineNodeOp, PipelineSink, PipelineSource,
};
use crate::engine::structural::generic_pipeline_node::RunModel;
use crate::engine::structural::pipeline_type_traits::{Sharable, Unit};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone)]
pub struct PipelineParameters {
    pub max_in_flight: usize,
    pub num_compute_threads: usize
}
impl PipelineParameters {
    pub fn new(max_in_flight: usize, num_compute_threads: usize) -> Self {
        Self {
            max_in_flight,
            num_compute_threads,
        }
    }
}

pub struct UnfinishedNodeBuilder<I: Sharable, O: Sharable, const NI: usize, const NO: usize> {
    node_predecessor: Rc<RefCell<Option<UnfinishedNode<I, O, NI, NO>>>>,
    build_vector: Rc<RefCell<PipelineBuildVector>>,
    pipeline_parameters: PipelineParameters,
}

fn node_func_wrap<I: Sharable, O: Sharable, RV, const NI: usize, const NO: usize>(
    node: Rc<RefCell<Option<UnfinishedNode<I, O, NI, NO>>>>,
    func: impl FnOnce(&UnfinishedNode<I, O, NI, NO>) -> RV,
) -> RV {
    match node.borrow().as_ref() {
        Some(node_internal) => func(node_internal),
        None => panic!("Node already consumed by another submission!"),
    }
}

fn node_func_wrap_mut<I: Sharable, O: Sharable, RV, const NI: usize, const NO: usize>(
    node: Rc<RefCell<Option<UnfinishedNode<I, O, NI, NO>>>>,
    func: impl FnOnce(&mut UnfinishedNode<I, O, NI, NO>) -> RV,
) -> RV {
    match node.borrow_mut().as_mut() {
        Some(node_internal) => func(node_internal),
        None => panic!("Node already consumed by another submission!"),
    }
}

impl<I: Sharable, O: Sharable, const NI: usize, const NO: usize> UnfinishedNodeBuilder<I, O, NI, NO> {
    pub fn get_parameters(&self) -> PipelineParameters {
        self.pipeline_parameters.clone()
    }
    
    fn internal_channel_generate<T: Sharable>(
        &self,
        src: usize,
        dst: usize,
    ) -> (WrappedSender<T>, WrappedReceiver<T>) {
        channel_wrapped::<T>(self.pipeline_parameters.max_in_flight, src, dst)
    }

    fn clone_self<IF: Sharable, OF: Sharable, const NIN: usize, const NON: usize>(
        &self,
        new_predecessor: Rc<RefCell<Option<UnfinishedNode<IF, OF, NIN, NON>>>>,
    ) -> UnfinishedNodeBuilder<IF, OF, NIN, NON> {
        UnfinishedNodeBuilder {
            node_predecessor: new_predecessor,
            build_vector: self.build_vector.clone(),
            pipeline_parameters: self.pipeline_parameters.clone(),
        }
    }

    fn channel_predecessor_attach(&self, dst: usize) -> WrappedReceiver<O> {
        let receiver = node_func_wrap_mut(self.node_predecessor.clone(), |predecessor| {
            let (sender, receiver) = channel_wrapped::<O>(
                self.pipeline_parameters.max_in_flight,
                predecessor.get_id(),
                dst,
            );

            predecessor.add_output(sender);

            return receiver;
        });

        receiver
    }

    pub fn get_node_id(&self) -> usize {
        node_func_wrap(self.node_predecessor.clone(), move |predecessor_internal| {
            predecessor_internal.get_id()
        })
    }

    pub fn get_build_vector(&self) -> Rc<RefCell<PipelineBuildVector>> {
        self.build_vector.clone()
    }

    pub fn attach_to_recipe_input(&mut self, dest_id: usize) -> WrappedReceiver<O> {
        self.channel_predecessor_attach(dest_id)
    }

    pub fn attach_to_recipe_output(&mut self, receiver: WrappedReceiver<I>) {
        node_func_wrap_mut(self.node_predecessor.clone(), |predecessor| {
            predecessor.add_input(receiver);
        });
    }

    // pub(in crate::engine::construction_layer::builders) fn attach_to_recipe_output(&mut self, new_id: usize) -> WrappedReceiver<O> {
    //
    // }

    fn generate_attach_std_closure<
        IF: Sharable,
        OF: Sharable,
        const NIN: usize,
        const NON: usize,
    >(
        node: Rc<RefCell<Option<UnfinishedNode<IF, OF, NIN, NON>>>>,
        run_model: RunModel,
    ) -> NodeSubmissionClosure {
        Box::new(move || match node.borrow_mut().take() {
            Some(node) => match run_model {
                RunModel::CPU => node.build_cpu_node(),
                RunModel::IO => node.build_io_node(),
                _ => panic!("Invalid run model for standard node"),
            },
            None => panic!("Node already consumed by another submission!"),
        })
    }
    
    pub fn attach_standard<F: Sharable, const NIN: usize, const NON: usize>(
        &mut self,
        name: String,
        step: impl PipelineNodeOp<O, F, NIN> + 'static,
        run_model: RunModel,
    ) -> UnfinishedNodeBuilder<O, F, NIN, NON> {
        // attach a step to the selected node (self) and create a thread
        // produce a successor node to continue the engine
        let new_id = self.build_vector.borrow_mut().get_new_id();

        let new_node = Rc::new(RefCell::new(Some(UnfinishedNode::<O, F, NIN, NON>::new(
            name,
            new_id,
            self.pipeline_parameters.max_in_flight,
        ))));

        let receiver = self.channel_predecessor_attach(new_id);

        let new_node_clone = new_node.clone();
        node_func_wrap_mut(new_node_clone, |new_node_in| {
            new_node_in.add_input(receiver);
            new_node_in.attach_step(Box::new(step));
        });

        self.build_vector.borrow_mut().promise_node(
            UnfinishedNodeBuilder::<O, F, NI, NO>::generate_attach_std_closure(new_node.clone(), run_model),
        );

        self.clone_self(new_node)
    }

    pub fn add_pipeline_sink(
        mut self,
        name: String,
        step: impl PipelineSink<O, 1> + 'static,
        running_model: RunModel,
    ) -> Self {
        // End a linear engine branch, allowing the step itself to handle output to other parts of the program
        let new_id = self.build_vector.borrow_mut().get_new_id();

        let mut new_node: Rc<RefCell<Option<UnfinishedNode<O, (), 1, 0>>>> =
            Rc::new(RefCell::new(Some(UnfinishedNode::new(
                name,
                new_id,
                self.pipeline_parameters.max_in_flight,
            ))));

        let receiver = self.channel_predecessor_attach(new_id);
        let new_node_clone = new_node.clone();
        node_func_wrap_mut(new_node_clone, |new_node_in| {
            new_node_in.add_input(receiver);
            new_node_in.attach_step(Box::new(step));
        });

        self.build_vector
            .borrow_mut()
            .promise_node(Self::generate_attach_std_closure(new_node, running_model));

        self
    }

    pub fn create_standalone_sink<T: Sharable>(
        name: String,
        step: impl PipelineSink<T, 1> + 'static,
        build_vector: Rc<RefCell<PipelineBuildVector>>,
        pipeline_parameters: PipelineParameters,
        running_model: RunModel,
    ) -> UnfinishedNodeBuilder<T, (), 1, 0> {
        let new_id = build_vector.borrow_mut().get_new_id();

        let mut new_node = Rc::new(RefCell::new(Some(UnfinishedNode::new(name, new_id, pipeline_parameters.max_in_flight))));
        let new_node_clone = new_node.clone();
        node_func_wrap_mut(new_node_clone, |new_node_in| {
            new_node_in.attach_step(Box::new(step));
        });

        build_vector
            .borrow_mut()
            .promise_node(Self::generate_attach_std_closure(
                new_node.clone(),
                running_model,
            ));

        UnfinishedNodeBuilder {
            node_predecessor: new_node,
            build_vector,
            pipeline_parameters,
        }
    }

    pub fn add_pipeline_source<F: Sharable, const NON: usize>(
        name: String,
        source_step: impl PipelineSource<F>,
        build_vector: Rc<RefCell<PipelineBuildVector>>,
        pipeline_parameters: PipelineParameters,
        running_model: RunModel,
    ) -> UnfinishedNodeBuilder<(), F, 0, NON>
    where
        I: Unit,
    {
        // start a engine, allowing the step itself to handle input from other parts of the program
        let new_id = build_vector.borrow_mut().get_new_id();
        let mut start_node = Rc::new(RefCell::new(Some(UnfinishedNode::new(name, new_id, pipeline_parameters.max_in_flight))));

        let new_node_clone = start_node.clone();
        node_func_wrap_mut(new_node_clone, |new_node_in| {
            new_node_in.attach_step(Box::new(source_step));
        });


        build_vector
            .borrow_mut()
            .promise_node(Self::generate_attach_std_closure(
                start_node.clone(),
                running_model,
            ));

        UnfinishedNodeBuilder {
            node_predecessor: start_node,
            pipeline_parameters,
            build_vector,
        }
    }

    pub fn create_joint_node<const NIN: usize, const NON: usize>(
        name: String,
        step: impl PipelineNodeOp<I, O, NIN> + 'static,
        build_vector: Rc<RefCell<PipelineBuildVector>>,
        pipeline_parameters: PipelineParameters,
        run_model: RunModel,
    ) -> UnfinishedNodeBuilder<I, O, NIN, NON>
    where
        [(); NIN - 2]: Sized,
    {
        // where statement requires that NIN is greater than 1
        let new_id = build_vector.borrow_mut().get_new_id();
        let mut new_node = Rc::new(RefCell::new(Some(UnfinishedNode::new(name, new_id, pipeline_parameters.max_in_flight))));

        let new_node_clone = new_node.clone();
        node_func_wrap_mut(new_node_clone, |new_node_in| {
            new_node_in.attach_step(Box::new(step));
        });

        build_vector
            .borrow_mut()
            .promise_node(Self::generate_attach_std_closure(
                new_node.clone(),
                run_model,
            ));

        UnfinishedNodeBuilder {
            node_predecessor: new_node,
            pipeline_parameters,
            build_vector,
        }
    }

    pub fn create_standalone_node(
        name: String,
        step: impl PipelineNodeOp<I, O, 1> + 'static,
        build_vector: Rc<RefCell<PipelineBuildVector>>,
        pipeline_parameters: PipelineParameters,
        run_model: RunModel,
    ) -> UnfinishedNodeBuilder<I, O, 1, 1> {
        let new_id = build_vector.borrow_mut().get_new_id();

        let new_node = Rc::new(RefCell::new(Some(UnfinishedNode::new(name, new_id, pipeline_parameters.max_in_flight))));
        let new_node_clone = new_node.clone();
        node_func_wrap_mut(new_node_clone, |new_node_in| {
            new_node_in.attach_step(Box::new(step));
        });

        build_vector
            .borrow_mut()
            .promise_node(Self::generate_attach_std_closure(
                new_node.clone(),
                run_model,
            ));

        UnfinishedNodeBuilder {
            node_predecessor: new_node,
            build_vector: build_vector.clone(),
            pipeline_parameters,
        }
    }

    pub fn feed_into<F: Sharable, const NIF: usize, const NOF: usize>(
        mut self,
        standalone_builder: &mut UnfinishedNodeBuilder<O, F, NIF, NOF>,
    ) -> Self {
        let receiver = self.channel_predecessor_attach(standalone_builder.get_node_id());

        node_func_wrap_mut(standalone_builder.node_predecessor.clone(), |standalone_node| {
            standalone_node.add_input(receiver);
        });

        self
    }

    pub fn add_initial_state(mut self, initial_state: O) -> Self {
        // must perform loop detection at construction time to verify that all loops produce a base value to avoid starting lockups
        node_func_wrap_mut(self.node_predecessor.clone(), |node| {
            node.add_initial_state(initial_state);
        });
        self
    }

    pub fn attach_series_reconstructor<const NON: usize, const ND: usize>(
        &mut self,
        name: String,
    ) -> UnfinishedNodeBuilder<O, BufferArray<O, ND>, 1, NON> {
        let new_id = self.build_vector.borrow_mut().get_new_id();

        let new_node = Rc::new(RefCell::new(Some(UnfinishedNode::new(name, new_id, self.pipeline_parameters.max_in_flight))));
        let receiver = self.channel_predecessor_attach(new_id);

        let new_node_clone = new_node.clone();
        node_func_wrap_mut(new_node_clone, |new_node_in| {
            new_node_in.add_input(receiver);
        });

        let new_node_clone = new_node.clone();
        self.build_vector.borrow_mut().promise_node(
            Box::new(move || {
                match new_node_clone.borrow_mut().take() {
                    Some(this_node) => this_node.build_reconstruct(),
                    None => panic!("Node already consumed by another submission!"),
                }
            })
        );

        self.clone_self(new_node)
    }
}

impl<I: Sharable, O: Sharable, const NI: usize, const NO: usize, const IBS: usize>
    UnfinishedNodeBuilder<I, BufferArray<O, IBS>, NI, NO>
{
    pub fn attach_interleaved_separator<const NON: usize, const OBS: usize>(
        &mut self,
        name: String,
    ) -> UnfinishedNodeBuilder<BufferArray<O, IBS>, BufferArray<O, OBS>, 1, NON>
    where
        [(); (IBS % NON == 0) as usize - 1]:,
        [(); (IBS % OBS == 0) as usize - 1]:
    {
        let new_id = self.build_vector.borrow_mut().get_new_id();
        
        let mut new_node = Rc::new(RefCell::new(Some(UnfinishedNode::new(name, new_id, self.pipeline_parameters.max_in_flight))));
        let receiver = self.channel_predecessor_attach(new_id);

        let new_node_clone = new_node.clone();
        node_func_wrap_mut(new_node_clone, |new_node_in| {
            new_node_in.add_input(receiver)
        });

        let new_node_clone = new_node.clone();
        self.build_vector.borrow_mut().promise_node(
            Box::new(move || {
                match new_node_clone.borrow_mut().take() {
                    Some(this_node) => {
                        let this_node: UnfinishedNode<BufferArray<O, IBS>, BufferArray<O, OBS>, 1, NON> = this_node;
                        this_node.build_interleave()
                    },
                    None => panic!("Node already consumed by another submission!"),
                }
            })
        );

        self.clone_self(new_node)
    }

    pub fn attach_series_deconstructor<const NON: usize>(
        &mut self,
        name: String,
    ) -> UnfinishedNodeBuilder<BufferArray<O, IBS>, O, 1, NON> {
        let new_id = self.build_vector.borrow_mut().get_new_id();

        let new_node = Rc::new(RefCell::new(Some(UnfinishedNode::new(name, new_id, self.pipeline_parameters.max_in_flight))));
        let receiver = self.channel_predecessor_attach(new_id);

        let new_node_clone = new_node.clone();
        node_func_wrap_mut(new_node_clone, |new_node_in| {
            new_node_in.add_input(receiver)
        });

        let new_node_clone = new_node.clone();
        self.build_vector.borrow_mut().promise_node(
            Box::new(move || {
                match new_node_clone.borrow_mut().take() {
                    Some(this_node) => this_node.build_deconstruct(),
                    None => panic!("Node already consumed by another submission!"),
                }
            })
        );

        self.clone_self(new_node.clone())
    }
}