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


enum AnalyticsSinkEnum {
    Proxied(Arc<Queue<NodeAnalytics>>),
    Direct(PipelineAnalyticsSink)
}

pub struct Pipeline<Scheduler: PipelineScheduler> {
    scheduler: Scheduler,
    node_graph: Arc<PipelineGraph>,
    external_stop_source: ExternalStopSource,
    analytics_sink: Option<AnalyticsSinkEnum>,
    background_proc_manager: Option<BackgroundTaskManager>
}
impl<Scheduler: PipelineScheduler> Pipeline<Scheduler> {
    pub fn new(scheduler: Scheduler, node_graph: Arc<PipelineGraph>, external_stop_source: ExternalStopSource, analytics_sink: Option<PipelineAnalyticsSink>, background_manager: Option<BackgroundTaskManager>) -> Self {
        let analytics_sink = match analytics_sink {
            Some(sink) => Some(AnalyticsSinkEnum::Direct(sink)),
            None => None
        };

        let mut pipeline = Self {
            scheduler,
            node_graph,
            external_stop_source,
            analytics_sink,
            background_proc_manager: background_manager,
        };
        match &mut pipeline.background_proc_manager {
            Some(proc_manager) => {
                match pipeline.analytics_sink {
                    Some()
                }
                if pipeline.analytics_sink.is_some() {
                    let mut sink = pipeline.analytics_sink.take().unwrap();
                    proc_manager.add_task(
                        String::from("Analytics Sink"),
                        Box::pin(async move {
                            sink.get_analytics_task().await
                        })
                    )
                }
            }
            None => {}
        }

        pipeline
    }

    pub fn start(&mut self) {
        info!("Starting Pipeline");
        self.scheduler.scheduler_start();
    }

    pub fn stop(&mut self) {
        self.scheduler.scheduler_stop();
    }
    pub fn get_analytics_sink(&self) -> &Option<PipelineAnalyticsSink> {
        &self.analytics_sink
    }
}
