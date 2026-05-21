use crate::engine::construction_layer::unfinished_node_builder::PipelineParameters;
use crate::engine::interface_layer::interface_hl::Interface;
use crate::engine::orchestration_layer::pipeline_graph::PipelineGraph;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::runtime::Runtime;

pub trait PipelineScheduler {
    fn new(
        graph: Arc<PipelineGraph>,
        pipeline_parameters: PipelineParameters,
        io_op_runtime: Arc<Runtime>,
    ) -> Self;
    fn scheduler_start(&mut self);
    fn scheduler_stop(&mut self);
}

pub struct Pipeline<Scheduler: PipelineScheduler> {
    scheduler: Scheduler,
    node_graph: Arc<PipelineGraph>,
    interface: Interface,
}
impl<Scheduler: PipelineScheduler> Pipeline<Scheduler> {
    pub fn new(scheduler: Scheduler, node_graph: Arc<PipelineGraph>, interface: Interface) -> Self {
        Self {
            scheduler,
            node_graph,
            interface,
        }
    }

    pub fn start(&mut self) {
        self.scheduler.scheduler_start();
    }

    pub fn stop(&mut self) {
        self.scheduler.scheduler_stop();
    }
}
