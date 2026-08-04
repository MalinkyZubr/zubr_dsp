use std::sync::Arc;
use iced::Theme;
use log::error;
use tokio::runtime::Runtime;
use crate::engine::application_wrappers::gui;
use crate::engine::application_wrappers::gui::app::{app_generator, app_update, app_view};
use crate::engine::build::{build_pipeline, PipelineBuildRoutine};
use crate::engine::control_plane::pipeline_hl::{Pipeline, PipelineScheduler};
use crate::engine::zubr_dsp_config::PipelineParameters;

pub enum ZubrDSPApplication<Scheduler: PipelineScheduler> {
    Headless(Pipeline<Scheduler>),
    Gui(Pipeline<Scheduler>),
    Cli,
    Socket,
    Websocket
}
impl <Scheduler: PipelineScheduler> ZubrDSPApplication<Scheduler> {
    pub fn new_gui(pipeline_build_routine: PipelineBuildRoutine, mut pipeline_parameters: PipelineParameters, io_op_runtime: Arc<Runtime>) -> Self {
        pipeline_parameters.proxied = true;
        
        if pipeline_parameters.analytics_parameters.is_none() {
            panic!("Failed to build gui pipeline. No analytics configured")
        }
        let pipeline = match build_pipeline(
            pipeline_build_routine,
            pipeline_parameters,
            io_op_runtime
        ) {
            Ok(pipeline) => pipeline,
            Err(msg) => {
                error!("Failed to build pipeline app: {}", msg);
                panic!("Pipeline build failure");
            }
        };
        ZubrDSPApplication::Gui(pipeline)
    }
    pub fn start(&mut self) {
        match self {
            ZubrDSPApplication::Headless(pipeline) => pipeline.start(),
            ZubrDSPApplication::Gui(pipeline) => {
                let proxy_queue = match pipeline.get_analytics_sink() {
                    Some(sink) => sink.get_proxy_queue(),
                    None => panic!("Failed to get analytics queue. Exiting")
                };
                pipeline.start();
                let _ = iced::application(app_generator::<1024>(Some(proxy_queue)), app_update, app_view)
                    .theme(Theme::Dark)
                    .exit_on_close_request(true)
                    .run();
            },
            _ => error!("ZubrDSPApplication::start failed, variant not implemented")
        }
    }
    
    pub fn stop(&mut self) -> Result<(), ()> {
        match self {
            ZubrDSPApplication::Headless(pipeline) => {
                pipeline.stop();
                Ok(())
            }
            _ => Err(())
        }
    }
    
    pub fn try_into_scheduler(mut self) -> Result<Pipeline<Scheduler>, Self> {
        match self {
            ZubrDSPApplication::Headless(pipeline) => {
                Ok(pipeline)
            }
            _ => Err(self)
        }
    }
}