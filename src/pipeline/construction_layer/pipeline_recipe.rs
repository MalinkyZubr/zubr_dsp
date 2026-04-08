use crate::pipeline::communication_layer::comms_core::{
    channel_wrapped, WrappedReceiver, WrappedSender,
};
use crate::pipeline::construction_layer::builders::NodeBuilder;
use crate::pipeline::construction_layer::node_builder::PipelineBuildVector;
use crate::pipeline::construction_layer::pipeline_traits::Sharable;
use std::cell::RefCell;
use std::rc::Rc;

pub struct RecipeInputMapping<T: Sharable> {
    inputs: Vec<WrappedReceiver<T>>,
    id: usize,
}
impl<T: Sharable> RecipeInputMapping<T> {
    fn new(build_vector: &mut Rc<RefCell<PipelineBuildVector>>) -> Self {
        Self {
            inputs: vec![],
            id: build_vector.borrow_mut().get_new_id(),
        }
    }

    fn push<I: Sharable, const NI: usize, const NO: usize>(
        &mut self,
        builder: &mut NodeBuilder<I, T, NI, NO>,
    ) {
        self.inputs.push(builder.attach_to_recipe_input(self.id));
    }
}

pub struct RecipeOutputMapping<T: Sharable> {
    outputs: Vec<WrappedSender<T>>,
    id: usize,
}
impl<T: Sharable> RecipeOutputMapping<T> {
    fn new(build_vector: &mut Rc<RefCell<PipelineBuildVector>>) -> Self {
        Self {
            outputs: vec![],
            id: build_vector.borrow_mut().get_new_id(),
        } // id must be reserved beforehand
    }

    fn push<F: Sharable, const NI: usize, const NO: usize>(
        &mut self,
        builder: &mut NodeBuilder<T, F, NI, NO>,
    ) {
        let (sender, receiver) = channel_wrapped::<T>(
            builder
                .get_build_vector()
                .borrow_mut()
                .get_parameters()
                .buff_size,
            self.id,
            builder.get_node_id(),
        );
        builder.attach_to_recipe_output(receiver);
        self.outputs.push(sender);
    }
}

type BuildFunction<I: Sharable, O: Sharable, const NI: usize, const NO: usize> = fn(
    [RecipeInputMapping<I>; NI],
    [RecipeOutputMapping<O>; NO],
    &mut Rc<RefCell<PipelineBuildVector>>,
);
pub struct PipelineRecipe<I: Sharable, O: Sharable, const NI: usize, const NO: usize> {
    build_function: BuildFunction<I, O, NI, NO>, // this function builds the pipeline. The parameters represent mappings from inputs to internal nodes. The second maps from internal nodes to outputs
    build_vector: Rc<RefCell<PipelineBuildVector>>,
    inputs: [RecipeInputMapping<I>; NI],
    outputs: [RecipeOutputMapping<O>; NO],
}
impl<I: Sharable, O: Sharable, const NI: usize, const NO: usize> PipelineRecipe<I, O, NI, NO> {
    pub fn new(
        build_function: BuildFunction<I, O, NI, NO>,
        mut build_vector: Rc<RefCell<PipelineBuildVector>>,
    ) -> Self {
        let mut inputs: Vec<RecipeInputMapping<I>> = vec![];
        for _ in 0..NI {
            inputs.push(RecipeInputMapping::new(&mut build_vector));
        }
        let inputs: [RecipeInputMapping<I>; NI] = match inputs.try_into() {
            Ok(x) => x,
            Err(_) => panic!("Could not convert inputs to array"),
        };

        let mut outputs: Vec<RecipeOutputMapping<O>> = vec![];
        for _ in 0..NO {
            outputs.push(RecipeOutputMapping::new(&mut build_vector));
        }
        let outputs = match outputs.try_into() {
            Ok(x) => x,
            Err(_) => panic!("Could not convert outputs to array"),
        };

        Self {
            build_function,
            inputs,
            outputs,
            build_vector,
        }
    }

    pub fn build(mut self) {
        (self.build_function)(self.inputs, self.outputs, &mut self.build_vector)
        // should submit all nodes internally
    }

    pub fn add_input<T: Sharable, const NIN: usize, const NON: usize>(
        &mut self,
        node_builder: &mut NodeBuilder<T, I, NIN, NON>,
        mapping: Vec<usize>, // to which ordered internal node does this go?
    ) {
        // mapping represents which internal nodes to connect the input to
        for internal_node in mapping {
            self.inputs[internal_node].push(node_builder)
        }
    }

    pub fn feed_into<F: Sharable, const NIF: usize, const NOF: usize>(
        &mut self,
        node_builder: &mut NodeBuilder<O, F, NIF, NOF>,
        mapping: Vec<usize>,
    ) {
        for output_position in mapping.iter() {
            self.outputs[*output_position].push(node_builder)
        }
    }
}
