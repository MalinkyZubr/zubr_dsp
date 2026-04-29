use std::cell::RefCell;
use std::io;
use std::rc::Rc;
use std::sync::Arc;
use log::Level;
use num::Complex;
use rodio::{OutputStreamBuilder, Sink};
use tokio::runtime::Builder;
use zubr_dsp::dsp::core::complex_magnitude::ComplexMagnitude;
use zubr_dsp::dsp::core::converters::RealToComplex;
use zubr_dsp::dsp::filtering::fir::fir::FIRFilter;
use zubr_dsp::dsp::modulation::am::am_demod::AMDemodulator;
use zubr_dsp::dsp::modulation::am::am_mod::AMModulator;
use zubr_dsp::dsp::sampling::resampling::{Resampler, UpsamplingMethod};
use zubr_dsp::dsp::sampling::simple_downsample::SimpleDownsampler;
use zubr_dsp::dsp::system_response::fft::{FFT, IFFT};
use zubr_dsp::dsp::system_response::overlap_save_chunks::{generate_overlap_save_steps, OverlapSaveBreaker, OverlapSaveCombiner};
use zubr_dsp::dsp::system_response::special_transfer_functions::tf_analytic;
use zubr_dsp::general::endpoints::audio_endpoint::AudioSink;
use zubr_dsp::general::sources::audio_file_source::AudioFileSource;
use zubr_dsp::general::throttle::Throttle;
use zubr_dsp::engine::communication_layer::data_management::BufferArray;
use zubr_dsp::engine::construction_layer::unfinished_node_builder::UnfinishedNodeBuilder;
use zubr_dsp::engine::construction_layer::unfinished_node::{PipelineBuildVector, PipelineParameters};
use zubr_dsp::engine::orchestration_layer::pipeline_graph::PipelineGraph;
use zubr_dsp::engine::orchestration_layer::scheduler_models::topographical::build_topographical_thread_pool;
use zubr_dsp::dsp::sampling::simple_downsample::*;
use zubr_dsp::initiate_pipeline;

pub fn io_bound_breaker_reassemble_test() -> Result<(), String> {
    println!("Beginning simulated AM end to end test\nEnter absolute path to mp3:");
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");

    let build_vector = Rc::new(RefCell::new(PipelineBuildVector::new(
        PipelineParameters::new(15),
    )));

    let stream = OutputStreamBuilder::open_default_stream().unwrap();
    let aud_sink = Sink::connect_new(stream.mixer());
    let (mut breaker, mut combiner) = generate_overlap_save_steps::<
        f32, 2048, 512, 256, 8, 128
    >();

    let mut source: UnfinishedNodeBuilder<_, _, 0, 1> = UnfinishedNodeBuilder::<(), i32, 0, 1>::add_pipeline_source(
        "audio_source".to_string(),
        AudioFileSource::<2048>::new(input.as_str().trim(), 10),
        build_vector.clone(),
    );
    let mut step1: UnfinishedNodeBuilder<_, _, 1, 1> = source
        .attach_standard("throttle".to_string(), Throttle::new(88.2e3));
    let mut step2: UnfinishedNodeBuilder<_, _, 1, 1> = step1.attach_standard("Chunker".to_string(), breaker);
    let mut step3 = step2.attach_series_deconstructor::<1>("deconstructor".to_string());
    let mut step4 = step3.attach_series_reconstructor::<1, 8>("reconstructor".to_string());
    let mut step5: UnfinishedNodeBuilder<_, _, 1, 1>= step4.attach_standard("dechunker".to_string(), combiner);
    let mut step6: UnfinishedNodeBuilder<_, _, 1, 1> = step5.attach_standard("demodulator".to_string(), AMDemodulator::new(10.0, 0.5))
        .add_pipeline_sink("audio sink".to_string(), AudioSink::new(2, 44100, aud_sink));

    source.submit_cpu();
    step1.submit_io();
    step2.submit_cpu();
    step3.submit_series_deconstructor();
    step4.submit_series_reconstructor();
    step5.submit_cpu();
    step6.submit_cpu();


    let rt = Builder::new_multi_thread()
        .worker_threads(2)          // increase async worker threads
        .max_blocking_threads(128)  // for spawn_blocking
        .enable_all()
        .build()
        .unwrap();
    let graph = Arc::new(PipelineGraph::new(build_vector));
    let mut handle = build_topographical_thread_pool(4, 4, graph.clone(), &rt, None);

    handle.start(&rt);

    println!("Press enter to stop");
    io::stdin().read_line(&mut String::new()).unwrap();
    handle.kill();

    Ok(())
}