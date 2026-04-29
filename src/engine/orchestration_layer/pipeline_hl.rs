use std::sync::Arc;
use crate::engine::interface_layer::interface_hl::Interface;
use tokio::runtime::Runtime;
use crate::engine::construction_layer::unfinished_node_builder::PipelineParameters;
use crate::engine::orchestration_layer::pipeline_graph::PipelineGraph;

pub trait PipelineScheduler {
    fn new(graph: Arc<PipelineGraph>, pipeline_parameters: PipelineParameters, io_op_runtime: Runtime) -> Self;
    fn scheduler_start(&mut self, async_runtime: &Runtime);
    fn scheduler_stop(&mut self);
}


pub struct Pipeline<Scheduler: PipelineScheduler> {
    scheduler: Scheduler,
    node_graph: Arc<PipelineGraph>,
    interface: Interface,
}
impl<Scheduler: PipelineScheduler> Pipeline<Scheduler> {
    pub fn new(scheduler: Scheduler, node_graph: Arc<PipelineGraph>, interface: Interface) -> Self {
        Self { scheduler, node_graph, interface }
    }
}