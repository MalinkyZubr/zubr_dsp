use crate::engine::communication_layer::data_management::DataWrapper;
use crate::engine::structural::pipeline_type_traits::Sharable;
use async_trait::async_trait;

// how can I make multiple input and output types more convenient?
/*
1. every engine step has a separate trait method for each input type, with separate signature. By defualt it will return an error saying its unimplemented
2. user can return whatever data scheme they want from each separate handler for the node to do with what it pleases
3. at the beginning of runtime, depending on the receiver type assigned to the node, a different handler (node method) is chosen to receive, so no additional match is needed
 */
#[async_trait]
pub trait PipelineNodeOp<I: Sharable, O: Sharable, const NI: usize>: Send + Sync + 'static {
    fn run_cpu(
        &mut self,
        _input: &mut [DataWrapper<I>; NI],
        _output: &mut DataWrapper<O>,
    ) -> Result<(), ()> {
        panic!("run not implemented!")
    }
    async fn run_io(
        &mut self,
        _input: &mut [DataWrapper<I>; NI],
        _output: &mut DataWrapper<O>,
    ) -> Result<(), ()> {
        panic!("run not implemented!")
    }
}


pub trait PipelineSource<T: Sharable>: PipelineNodeOp<(), T, 0> {}

impl<T, U> PipelineSource<U> for T
where
    U: Sharable,
    T: PipelineNodeOp<(), U, 0>,
{}

pub trait PipelineSink<T: Sharable, const NI: usize>: PipelineNodeOp<T, (), NI> {}

impl<T, U, const NI: usize> PipelineSink<U, NI> for T
where
    U: Sharable,
    T: PipelineNodeOp<U, (), NI>,
{}