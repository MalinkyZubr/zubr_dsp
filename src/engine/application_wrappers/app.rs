use std::sync::Arc;
use iced::{Settings, Theme};
use log::error;
use tokio::runtime::Runtime;
use crate::engine::application_wrappers::gui;
use crate::engine::application_wrappers::gui::app::{app_generator, app_subscription, app_update, app_view};
use crate::engine::build::{build_pipeline, PipelineBuildRoutine};
use crate::engine::control_plane::pipeline_hl::{Pipeline, PipelineScheduler};
use crate::engine::zubr_dsp_config::PipelineParameters;


static ICON: &[u8] = include_bytes!("../../../assets/favicon.ico");
const ICON_HEIGHT: u32 = 256;
const ICON_WIDTH: u32 = 256;


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
                let image = image::load_from_memory(ICON).unwrap();
                let icon = iced::window::icon::from_rgba(image.as_bytes().to_vec(), ICON_HEIGHT, ICON_WIDTH).unwrap();

                let proxy_queue = match pipeline.get_analytics_queue() {
                    Some(queue) => queue,
                    _ => panic!("Welp that sucks")
                };
                pipeline.start();
                let _ = iced::application(app_generator::<2048>(Some(proxy_queue)), app_update, app_view)
                    .theme(Theme::Dark)
                    .subscription(app_subscription)
                    .exit_on_close_request(true)
                    .window(iced::window::Settings {
                        icon: Some(icon),
                        ..Default::default()
                    })
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