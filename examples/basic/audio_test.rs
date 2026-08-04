use std::cell::RefCell;
use std::io;
use std::rc::Rc;
use std::sync::Arc;
use log::{info, Level};
use tokio::sync::mpsc::channel;
use rodio::{OutputStream, OutputStreamBuilder, Sink};
use tokio::runtime::{Builder, Runtime};
use zubr_dsp::dsp::core::complex_magnitude::ComplexMagnitude;
use zubr_dsp::dsp::core::converters::RealToComplex;
use zubr_dsp::dsp::filtering::fir::fir::FIRFilter;
use zubr_dsp::dsp::modulation::am::am_demod::AMDemodulator;
use zubr_dsp::dsp::modulation::am::am_mod::AMModulator;
use zubr_dsp::dsp::system_response::fft::{FFT, IFFT};
use zubr_dsp::dsp::system_response::overlap_save_chunks::generate_overlap_save_steps;
use zubr_dsp::dsp::system_response::special_transfer_functions::tf_analytic;
use zubr_dsp::engine::build::build_pipeline;
use zubr_dsp::engine::control_plane::logging::{init_logger, init_stdout_logger, StdoutOutput};
use zubr_dsp::engine::control_plane::pipeline_graph::PipelineGraph;
use zubr_dsp::engine::control_plane::pipeline_hl::Pipeline;
use zubr_dsp::engine::control_plane::scheduler_models::topographical::ThreadPoolTopographicalHandle;
use zubr_dsp::engine::data_plane::construction::unfinished_node_builder::{PipelineInterfaceConfiguration, UnfinishedNodeBuilder};
use zubr_dsp::engine::data_plane::structural::generic_pipeline_node::RunModel::{CPU, IO};
use zubr_dsp::engine::zubr_dsp_config::PipelineParameters;
use zubr_dsp::general::endpoints::audio_endpoint::AudioSink;
use zubr_dsp::general::sources::audio_file_source::AudioFileSource;
use zubr_dsp::general::throttle::Throttle;
use zubr_dsp::initiate_pipeline;

pub fn audio_test() -> Result<(), String> {
    println!("Beginning audio test\nEnter absolute path to mp3:");

    info!("Starting audio test");

    let rt = Arc::new(Builder::new_multi_thread()
        .worker_threads(16)          // increase async worker threads
        .max_blocking_threads(128)  // for spawn_blocking
        .enable_all()
        .build()
        .unwrap());

    let stream = OutputStreamBuilder::open_default_stream().unwrap();
    let aud_sink = Sink::connect_new(stream.mixer());

    let mut pipeline: Pipeline<ThreadPoolTopographicalHandle> = build_pipeline(
        Box::new(|bv, par| {
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
        }),
        PipelineParameters::standard_no_analytics(),
        rt,
    ).unwrap();

    pipeline.start();

    println!("Press enter to stop");
    io::stdin().read_line(&mut String::new()).unwrap();
    pipeline.stop();

    Ok(())
}