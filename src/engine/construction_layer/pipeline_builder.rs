use crate::engine::communication_layer::comms_core::{channel_wrapped, WrappedReceiver};
use crate::engine::communication_layer::data_management::BufferArray;
use crate::engine::construction_layer::build_vector::{NodeSubmissionClosure, PipelineBuildVector};
use crate::engine::construction_layer::node_builder::BuildingNode;
use crate::engine::structural::generic_node_operation::{
    PipelineNodeOp, PipelineSink, PipelineSource,
};
use crate::engine::structural::generic_pipeline_node::RunModel;
use crate::engine::structural::pipeline_type_traits::{Sharable, Unit};
use std::cell::RefCell;
use std::rc::Rc;

pub struct NodeBuilder<I: Sharable, O: Sharable, const NI: usize, const NO: usize> {
    node_predecessor: Rc<RefCell<Option<BuildingNode<I, O, NI, NO>>>>,
    build_vector: Rc<RefCell<PipelineBuildVector>>,
}

fn predecessor_func_wrap<I: Sharable, O: Sharable, RV, const NI: usize, const NO: usize>(
    predecessor: Rc<RefCell<Option<BuildingNode<I, O, NI, NO>>>>,
    func: impl FnOnce(&BuildingNode<I, O, NI, NO>) -> RV,
) -> RV {
    match predecessor.borrow().as_ref() {
        Some(predecessor_internal) => func(predecessor_internal),
        None => panic!("Node already consumed by another submission!"),
    }
}

fn predecessor_func_wrap_mut<I: Sharable, O: Sharable, RV, const NI: usize, const NO: usize>(
    predecessor: Rc<RefCell<Option<BuildingNode<I, O, NI, NO>>>>,
    func: impl FnOnce(&mut BuildingNode<I, O, NI, NO>) -> RV,
) -> RV {
    match predecessor.borrow_mut().as_mut() {
        Some(predecessor_internal) => func(predecessor_internal),
        None => panic!("Node already consumed by another submission!"),   
    }
}

impl<I: Sharable, O: Sharable, const NI: usize, const NO: usize> NodeBuilder<I, O, NI, NO> {
    pub fn get_node_id(&self) -> usize {
        predecessor_func_wrap(
            self.node_predecessor.clone(),
            move |predecessor_internal| {
                predecessor_internal.get_id()
            }
        )
    }

    pub fn get_build_vector(&self) -> Rc<RefCell<PipelineBuildVector>> {
        self.build_vector.clone()
    }

    pub fn attach_to_recipe_input(
        &mut self,
        dest_id: usize,
    ) -> WrappedReceiver<O> {
        
        let (sender, receiver) = channel_wrapped::<O>(
            self.build_vector
                .borrow_mut()
                .get_pipeline_parameters()
                .buff_size,
            self.node_predecessor.get_id(),
            dest_id,
        );
        self.node_predecessor.add_output(sender);

        receiver
    }

    pub fn attach_to_recipe_output(
        &mut self,
        receiver: WrappedReceiver<I>,
    ) {
        self.node_predecessor.add_input(receiver);
    }

    // pub(in crate::engine::construction_layer::builders) fn attach_to_recipe_output(&mut self, new_id: usize) -> WrappedReceiver<O> {
    //
    // }

    fn generate_attach_std_closure<F: Sharable, const NIN: usize, const NON: usize>(
        node: Rc<RefCell<Option<BuildingNode<O, F, NIN, NON>>>>,
        run_model: RunModel,
    ) -> NodeSubmissionClosure {
        Box::new(move || match node.borrow_mut() {
            Some(node) => match run_model {
                RunModel::CPU => node.borrow_mut().build_cpu_node(),
                RunModel::IO => node.borrow_mut().build_io_node(),
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
    ) -> NodeBuilder<O, F, NIN, NON> {
        // attach a step to the selected node (self) and create a thread
        // produce a successor node to continue the engine
        let new_id = self.build_vector.borrow_mut().get_new_id();
        let buff_size = self
            .build_vector
            .borrow()
            .get_pipeline_parameters()
            .buff_size;
        let mut new_node = Rc::new(RefCell::new(BuildingNode::new(name, new_id, buff_size)));
        new_node.attach_step(Box::new(step));

        let new_node_clone = new_node.clone();

        self.build_vector
            .borrow_mut()
            .promise_node(self.generate_attach_std_closure(new_node_clone, run_model));

        let (sender, receiver) = channel_wrapped::<O>(
            self.build_vector
                .borrow_mut()
                .get_pipeline_parameters()
                .buff_size,
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

    pub fn add_pipeline_sink(
        mut self,
        name: String,
        step: impl PipelineSink<O, 1> + 'static,
        running_model: RunModel,
    ) -> Self {
        // End a linear engine branch, allowing the step itself to handle output to other parts of the program
        let new_id = self.build_vector.borrow_mut().get_new_id();
        let buff_size = self
            .build_vector
            .borrow()
            .get_pipeline_parameters()
            .buff_size;
        let mut new_node: Rc<RefCell<BuildingNode<O, (), 1, 0>>> =
            Rc::new(RefCell::new(BuildingNode::new(name, new_id, buff_size)));
        let (sender, receiver) = channel_wrapped::<O>(
            self.build_vector
                .borrow_mut()
                .get_pipeline_parameters()
                .buff_size,
            self.node_predecessor.get_id(),
            new_node.get_id(),
        );
        new_node.attach_step(Box::new(step));

        self.node_predecessor.add_output(sender);
        new_node.add_input(receiver);

        self.build_vector
            .borrow_mut()
            .promise_node(Self::generate_attach_std_closure(new_node, running_model));

        self
    }

    pub fn create_standalone_sink<T: Sharable>(
        name: String,
        step: impl PipelineSink<T, 1> + 'static,
        build_vector: Rc<RefCell<PipelineBuildVector>>,
        running_model: RunModel,
    ) -> NodeBuilder<T, (), 1, 0> {
        let new_id = build_vector.borrow_mut().get_new_id();
        let buff_size = build_vector.borrow().get_pipeline_parameters().buff_size;
        let mut new_node = Rc::new(RefCell::new(BuildingNode::new(name, new_id, buff_size)));
        new_node.attach_step(Box::new(step));

        build_vector
            .borrow_mut()
            .promise_node(Self::generate_attach_std_closure(
                new_node.clone(),
                running_model,
            ));

        NodeBuilder {
            node_predecessor: new_node,
            build_vector,
        }
    }

    pub fn add_pipeline_source<F: Sharable, const NON: usize>(
        name: String,
        source_step: impl PipelineSource<F>,
        build_vector: Rc<RefCell<PipelineBuildVector>>,
        running_model: RunModel,
    ) -> NodeBuilder<(), F, 0, NON>
    where
        I: Unit,
    {
        // start a engine, allowing the step itself to handle input from other parts of the program
        let new_id = build_vector.borrow_mut().get_new_id();
        let buff_size = build_vector.borrow().get_pipeline_parameters().buff_size;
        let mut start_node = Rc::new(RefCell::new(BuildingNode::new(name, new_id, buff_size)));
        start_node.attach_step(Box::new(source_step));

        build_vector
            .borrow_mut()
            .promise_node(Self::generate_attach_std_closure(
                start_node.clone(),
                RunModel::IO,
            ));

        NodeBuilder {
            node_predecessor: start_node,
            build_vector,
        }
    }

    pub fn create_joint_node<const NIN: usize, const NON: usize>(
        name: String,
        step: impl PipelineNodeOp<I, O, NIN> + 'static,
        build_vector: Rc<RefCell<PipelineBuildVector>>,
        run_model: RunModel,
    ) -> NodeBuilder<I, O, NIN, NON>
    where
        [(); NIN - 2]: Sized,
    {
        // where statement requires that NIN is greater than 1
        let new_id = build_vector.borrow_mut().get_new_id();
        let buff_size = build_vector.borrow().get_pipeline_parameters().buff_size;
        let mut new_node = Rc::new(RefCell::new(BuildingNode::new(name, new_id, buff_size)));
        new_node.attach_step(Box::new(step));

        build_vector
            .borrow_mut()
            .promise_node(Self::generate_attach_std_closure(
                new_node.clone(),
                run_model,
            ));

        NodeBuilder {
            node_predecessor: new_node,
            build_vector,
        }
    }

    pub fn create_standalone_node(
        name: String,
        step: impl PipelineNodeOp<I, O, 1> + 'static,
        build_vector: Rc<RefCell<PipelineBuildVector>>,
        run_model: RunModel,
    ) -> NodeBuilder<I, O, 1, 1> {
        let new_id = build_vector.borrow_mut().get_new_id();
        let buff_size = build_vector.borrow().get_pipeline_parameters().buff_size;

        let mut new_node = Rc::new(RefCell::new(BuildingNode::new(name, new_id, buff_size)));
        new_node.attach_step(Box::new(step));

        build_vector
            .borrow_mut()
            .promise_node(Self::generate_attach_std_closure(
                new_node.clone(),
                run_model,
            ));

        NodeBuilder {
            node_predecessor: new_node,
            build_vector: build_vector.clone(),
        }
    }

    pub fn feed_into<F: Sharable, const NIF: usize, const NOF: usize>(
        mut self,
        standalone_builder: &mut NodeBuilder<O, F, NIF, NOF>,
    ) -> Self {
        let (sender, receiver) = channel_wrapped::<O>(
            self.build_vector
                .borrow_mut()
                .get_pipeline_parameters()
                .buff_size,
            self.get_node_id(),
            standalone_builder.get_node_id(),
        );

        self.node_predecessor.add_output(sender);
        standalone_builder.node_predecessor.add_input(receiver);

        self
    }

    pub fn add_initial_state(mut self, initial_state: O) -> Self {
        // must perform loop detection at construction time to verify that all loops produce a base value to avoid starting lockups
        self.node_predecessor
            .borrow_mut()
            .add_initial_state(initial_state);
        self
    }

    pub fn attach_series_reconstructor<const NON: usize, const ND: usize>(
        &mut self,
        name: String,
    ) -> NodeBuilder<O, BufferArray<O, ND>, 1, NON> {
        let new_id = self.build_vector.borrow_mut().get_new_id();
        let buff_size = self
            .build_vector
            .borrow()
            .get_pipeline_parameters()
            .buff_size;

        let mut new_node = BuildingNode::new(name, new_id, buff_size);
        let (sender, receiver) = channel_wrapped::<O>(
            self.build_vector
                .borrow_mut()
                .get_pipeline_parameters()
                .buff_size,
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

impl<I: Sharable, O: Sharable, const NI: usize, const NO: usize, const BS: usize>
    NodeBuilder<I, BufferArray<O, BS>, NI, NO>
{
    pub fn attach_interleaved_separator<const NON: usize>(
        &mut self,
        name: String,
    ) -> NodeBuilder<BufferArray<O, BS>, BufferArray<O, { BS / NON }>, 1, NON> {
        let new_id = self.build_vector.borrow_mut().get_new_id();
        let buff_size = self
            .build_vector
            .borrow()
            .get_pipeline_parameters()
            .buff_size;
        let mut new_node = BuildingNode::new(name, new_id, buff_size);
        let (sender, receiver) = channel_wrapped::<BufferArray<O, BS>>(
            self.build_vector
                .borrow_mut()
                .get_pipeline_parameters()
                .buff_size,
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
    ) -> NodeBuilder<BufferArray<O, BS>, O, 1, NON> {
        let new_id = self.build_vector.borrow_mut().get_new_id();
        let buff_size = self
            .build_vector
            .borrow()
            .get_pipeline_parameters()
            .buff_size;
        let mut new_node = BuildingNode::new(name, new_id, buff_size);
        let (sender, receiver) = channel_wrapped::<BufferArray<O, BS>>(
            self.build_vector
                .borrow_mut()
                .get_pipeline_parameters()
                .buff_size,
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

// impl<I: Sharable, O: Sharable, const NI: usize, const NO: usize> NodeBuilder<I, O, NI, NO> {
//     pub fn submit_io(self) {
//         self.build_vector
//             .borrow_mut()
//             .add_node(self.node_predecessor.build_io_node())
//     }
//
//     pub fn submit_cpu(self) {
//         self.build_vector
//             .borrow_mut()
//             .add_node(self.node_predecessor.build_cpu_node())
//     }
// }
//
// impl<T: Sharable, const NO: usize, const NR: usize> NodeBuilder<T, BufferArray<T, NR>, 1, NO> {
//     pub fn submit_series_reconstructor(self) {
//         self.build_vector
//             .borrow_mut()
//             .add_node(self.node_predecessor.build_reconstruct())
//     }
// }
//
// impl<T: Sharable, const NO: usize, const IBS: usize, const OBS: usize>
// NodeBuilder<BufferArray<T, IBS>, BufferArray<T, OBS>, 1, NO>
// where
//     [(); (IBS % NO == 0) as usize - 1]: Sized,
//     [(); (IBS % OBS == 0) as usize - 1]: Sized, // the input buffer size should be perfectly divisible by NUM_CHANNELS
// {
//     pub fn submit_interleaved_separator(self) {
//         self.build_vector
//             .borrow_mut()
//             .add_node(self.node_predecessor.build_interleave())
//     }
// }
//
// impl<T: Sharable, const NO: usize, const ND: usize> NodeBuilder<BufferArray<T, ND>, T, 1, NO> {
//     pub fn submit_series_deconstructor(self) {
//         self.build_vector
//             .borrow_mut()
//             .add_node(self.node_predecessor.build_deconstruct())
//     }
// }
