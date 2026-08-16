use std::io;
use std::sync::Arc;
use futures::executor::block_on;
use itertools::Itertools;
use log::Level;
use num::Complex;
use rodio::{OutputStreamBuilder, Sink};
use tokio::runtime::Builder;
use zubr_dsp::dsp::core::converters::{ComplexToReal, RealToComplex};
use zubr_dsp::dsp::modulation::iq::am::am_demod::AMDemodulator;
use zubr_dsp::dsp::modulation::iq::am::am_mod::AMModulator;
use zubr_dsp::dsp::system_response::overlap_save_chunks::generate_overlap_save_steps;
use zubr_dsp::engine::build::build_pipeline;
use zubr_dsp::engine::control_plane::pipeline_hl::Pipeline;
use zubr_dsp::engine::control_plane::scheduler_models::topographical::ThreadPoolTopographicalHandle;
use zubr_dsp::engine::data_plane::construction::unfinished_node_builder::UnfinishedNodeBuilder;
use zubr_dsp::engine::data_plane::structural::generic_pipeline_node::RunModel::{CPU, IO};
use zubr_dsp::engine::zubr_dsp_config::PipelineParameters;
use zubr_dsp::general::endpoints::audio_endpoint::AudioSink;
use zubr_dsp::general::sources::audio_file_source::AudioFileSource;
use zubr_dsp::general::sources::rtl_sdr_source::RtlSdrSource;
use zubr_dsp::general::throttle::Throttle;
use zubr_dsp::initiate_pipeline;

pub fn am_sdr_test() -> Result<(), String> {
    println!("Beginning sdr AM end to end test\n");

    let rt = Arc::new(Builder::new_multi_thread()
        .worker_threads(16)          // increase async worker threads
        .max_blocking_threads(128)  // for spawn_blocking
        .enable_all()
        .build()
        .unwrap());

    let stream = OutputStreamBuilder::open_default_stream().unwrap();
    let aud_sink = Sink::connect_new(stream.mixer());

    initiate_pipeline(Level::Trace);

    let mut pipeline: Pipeline<ThreadPoolTopographicalHandle> = build_pipeline(
        Box::new(|bv, par| {

            let res = block_on(RtlSdrSource::<2048>::new(0, 250000, 200_000_000, Some(1)));

            let mut source: UnfinishedNodeBuilder<_, _, 0, 1> = UnfinishedNodeBuilder::<(), i32, 0, 1>::add_pipeline_source(
                "sdr_source".to_string(),
                res,
                bv.clone(),
                par,
                IO
            );
            let carrier_amplitude = 10.0;
            let modulation_index = 0.5;
            let sample_frequency = 44.1e3;

            source
                .attach_standard::<_, 1, 1>("AM Modulator".to_string(), AMModulator::new(carrier_amplitude, modulation_index), CPU)
                .attach_standard::<_, 1, 1>("AM DeModulator".to_string(), AMDemodulator::new(carrier_amplitude, modulation_index), CPU)
                .attach_standard::<_, 1, 1>("Complex to real".to_string(), ComplexToReal::new(), CPU)
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