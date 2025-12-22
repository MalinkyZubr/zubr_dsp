use crate::pipeline::construction_layer::pipeline_node::PipelineNode;
use crate::pipeline::orchestration_layer::pipeline::ConstructionQueue;

pub trait PipelineRecipe<
    I: crate::pipeline::pipeline_traits::Sharable,
    O: crate::pipeline::pipeline_traits::Sharable,
>
{
    // allow to save and standardize macro-scale components that you dont wan tto repeatedly redefine
    fn construct<
        FI: crate::pipeline::pipeline_traits::Sharable,
        FO: crate::pipeline::pipeline_traits::Sharable,
    >(
        origin_node: PipelineNode<FI, I>,
        construction_queue: ConstructionQueue,
    ) -> PipelineNode<O, FO>;
}
