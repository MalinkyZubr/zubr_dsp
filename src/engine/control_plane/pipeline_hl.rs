use std::collections::HashMap;
use std::pin::{pin, Pin};
use crate::engine::zubr_dsp_config::PipelineParameters;
use crate::engine::control_plane::pipeline_graph::PipelineGraph;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use log::info;
use scc::Queue;
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
    analytics_queue: Option<Arc<Queue<NodeAnalytics>>>,
    background_proc_manager: Option<BackgroundTaskManager>
}
impl<Scheduler: PipelineScheduler> Pipeline<Scheduler> {
    pub fn new_headless(scheduler: Scheduler, node_graph: Arc<PipelineGraph>, external_stop_source: ExternalStopSource) -> Self {
        let mut pipeline = Self {
            scheduler,
            node_graph,
            external_stop_source,
            analytics_queue: None,
            background_proc_manager: None,
        };

        pipeline
    }

    pub fn new_head(scheduler: Scheduler, node_graph: Arc<PipelineGraph>, external_stop_source: ExternalStopSource, mut analytics_sink: PipelineAnalyticsSink, mut background_manager: BackgroundTaskManager) -> Self{
        let analytics_queue = analytics_sink.get_proxy_queue();
        background_manager.add_task("analytics sink".to_string(), Box::new(analytics_sink));

        let mut pipeline = Self {
            scheduler,
            node_graph,
            external_stop_source,
            analytics_queue: Some(analytics_queue),
            background_proc_manager: Some(background_manager)
        };

        pipeline
    }

    pub fn start(&mut self) {
        info!("Starting Pipeline");
        self.scheduler.scheduler_start();
    }

    pub fn stop(&mut self) {
        self.scheduler.scheduler_stop();
    }
    pub fn get_analytics_queue(&self) -> Option<Arc<Queue<NodeAnalytics>>> {
        self.analytics_queue.clone()
    }
}
