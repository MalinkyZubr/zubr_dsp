use tokio::runtime::Runtime;
use crate::engine::orchestration_layer::pipeline_graph::PipelineGraph;

pub trait PipelineScheduler {
    fn scheduler_start(&mut self, async_runtime: &Runtime);
    fn scheduler_stop(&mut self);
}


pub struct Pipeline {
    scheduler: Box<dyn PipelineScheduler>,
    node_graph: PipelineGraph,
    
}