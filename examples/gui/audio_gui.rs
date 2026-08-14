use std::io;
use std::sync::Arc;
use iced::Theme;
use rodio::{OutputStreamBuilder, Sink};
use tokio::runtime::Builder;
use zubr_dsp::engine::application_wrappers::app::ZubrDSPApplication;
use zubr_dsp::engine::application_wrappers::gui::app::{App, app_update, app_view, app_generator};
use zubr_dsp::engine::build::{build_pipeline, PipelineBuildRoutine};
use zubr_dsp::engine::control_plane::pipeline_hl::Pipeline;
use zubr_dsp::engine::control_plane::scheduler_models::topographical::ThreadPoolTopographicalHandle;
use zubr_dsp::engine::data_plane::construction::unfinished_node_builder::UnfinishedNodeBuilder;
use zubr_dsp::engine::data_plane::structural::generic_pipeline_node::RunModel::{CPU, IO};
use zubr_dsp::engine::zubr_dsp_config::PipelineParameters;
use zubr_dsp::general::endpoints::audio_endpoint::AudioSink;
use zubr_dsp::general::sources::audio_file_source::AudioFileSource;
use zubr_dsp::general::throttle::Throttle;

pub fn audio_gui() -> Result<(), String> {
    let rt = Arc::new(Builder::new_multi_thread()
        .worker_threads(16)          // increase async worker threads
        .max_blocking_threads(128)  // for spawn_blocking
        .enable_all()
        .build()
        .unwrap());

    let stream = OutputStreamBuilder::open_default_stream().unwrap();
    let aud_sink = Sink::connect_new(stream.mixer());

    let build_function: PipelineBuildRoutine = Box::new(|bv, par| {
        let mut input = String::new();
        // io::stdin()
        //     .read_line(&mut input)
        //     .expect("Failed to read line");
        input = String::from("/home/malinkyzubr/Documents/test.mp3");

        let mut source: UnfinishedNodeBuilder<_, _, 0, 1> = UnfinishedNodeBuilder::<(), i32, 0, 1>::add_pipeline_source(
            "audio_source".to_string(),
            AudioFileSource::<2048>::new(input.as_str().trim(), 10),
            bv.clone(),
            par,
            CPU
        );
        source
            .attach_standard::<_, 1, 1>("throttle".to_string(), Throttle::new(88.2e3), IO)
            .add_pipeline_sink("audio sink".to_string(), AudioSink::new(2, 44100, aud_sink), CPU);
    });

    let mut app = ZubrDSPApplication::<ThreadPoolTopographicalHandle>::new_gui(build_function, PipelineParameters::standard(), rt);
    app.start();
    app.stop();
    // println!("Press enter to stop");
    // io::stdin().read_line(&mut String::new()).unwrap();
    Ok(())
}