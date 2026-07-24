use std::collections::HashMap;
use std::pin::Pin;
use crate::engine::data_plane::construction::unfinished_node_builder::PipelineParameters;
use crate::engine::interface_layer::interface_hl::Interface;
use crate::engine::control_plane::pipeline_graph::PipelineGraph;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use log::info;
use tokio::runtime::Runtime;
use crate::engine::control_plane::background_proc_manager::BackgroundTaskManager;
use crate::engine::control_plane::node_state_manager::ExternalStopSource;
use crate::engine::control_plane::pipeline_analytics::{NodeAnalytics, PipelineAnalyticsSink};

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
    external_stop_source: ExternalStopSource,
    analytics_sink: Option<Arc<PipelineAnalyticsSink>>,
    background_task_manager: BackgroundTaskManager,
}
impl<Scheduler: PipelineScheduler> Pipeline<Scheduler> {
    pub fn new(scheduler: Scheduler, node_graph: Arc<PipelineGraph>, external_stop_source: ExternalStopSource, analytics_sink: Option<Arc<PipelineAnalyticsSink>>) -> Self {
        let background_task_manager = BackgroundTaskManager::new();
        
        Self {
            scheduler,
            node_graph,
            external_stop_source,
            analytics_sink,
            background_task_manager,
        }
    }

    pub fn start(&mut self) {
        info!("Starting Pipeline");
        self.scheduler.scheduler_start();
    }

    pub fn stop(&mut self) {
        self.scheduler.scheduler_stop();
    }
    
    pub fn get_analytics(&self) -> Option<HashMap<usize, NodeAnalytics>> {
        match self.analytics_sink.as_ref() {
            Some(sink) => {
                let mut result_map = HashMap::new();
                for id in 0..self.node_graph.get_num_nodes() {
                    match sink.get_analytics(id) { 
                        Some(anltc) => {result_map.insert(id, anltc);},
                        None => (),
                    }
                }
                Some(result_map)
            }
            None => None,
        }
    }

    pub fn get_analytics_arc(&self) -> Option<Arc<PipelineAnalyticsSink>> {
        self.analytics_sink.clone()
    }
}
