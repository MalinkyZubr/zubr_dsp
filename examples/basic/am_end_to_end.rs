use std::cell::RefCell;
use std::io;
use std::rc::Rc;
use std::sync::Arc;
use num::Complex;
use rodio::{OutputStreamBuilder, Sink};
use zubr_dsp::dsp::core::analytic_signal::IntoAnalytic;
use zubr_dsp::dsp::core::complex_magnitude::ComplexMagnitude;
use zubr_dsp::dsp::core::converters::RealToComplex;
use zubr_dsp::dsp::modulation::am::am_demod::AMDemodulator;
use zubr_dsp::dsp::modulation::am::am_mod::AMModulator;
use zubr_dsp::dsp::system_response::fft::{FFT, IFFT};
use zubr_dsp::dsp::system_response::overlap_save_chunks::{OverlapSaveBreaker, OverlapSaveCombiner};
use zubr_dsp::general::endpoints::audio_endpoint::AudioSink;
use zubr_dsp::general::sources::audio_file_source::AudioFileSource;
use zubr_dsp::general::throttle::Throttle;
use zubr_dsp::pipeline::communication_layer::data_management::BufferArray;
use zubr_dsp::pipeline::construction_layer::builders::NodeBuilder;
use zubr_dsp::pipeline::construction_layer::node_builder::{PipelineBuildVector, PipelineParameters};
use zubr_dsp::pipeline::orchestration_layer::pipeline_graph::PipelineGraph;
use zubr_dsp::pipeline::orchestration_layer::thread_pool_models::work_stealing_full_buffer::build_topographical_thread_pool;

pub fn audio_test() -> Result<(), String> {
    println!("Beginning audio test\nEnter absolute path to mp3:");
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");

    let build_vector = Rc::new(RefCell::new(PipelineBuildVector::new(
        PipelineParameters::new(16),
    )));
    let mut source: NodeBuilder<_, _, 0, 1> = NodeBuilder::<(), i32, 0, 1>::add_pipeline_source(
        "audio_source".to_string(),
        AudioFileSource::<65536>::new(input.as_str().trim(), 10),
        build_vector.clone(),
    );

    let stream = OutputStreamBuilder::open_default_stream().unwrap();
    let aud_sink = Sink::connect_new(stream.mixer());
    let mut step1: NodeBuilder<_, _, 1, 1> = source
        .attach_standard("test_step".to_string(), Throttle::new(88.2e3));
    let mut step2 = step1.attach_standard("AmModulator".to_string(), AMModulator::<65536>::new(
        10.0, 1000000.0, 0.5, 2500000.0
    ));
    let mut step3 = step2.attach_standard("Chunker".to_string(), OverlapSaveBreaker::new());
    let mut step4 = step3.attach_series_deconstructor("deconstructor".to_string());
    let mut converter = step4.attach_standard("converter".to_string(), RealToComplex::new());
    let mut step5 = converter.attach_standard("fft".to_string(), FFT::new());
    let mut step6 = step5.attach_standard("analytic filter".to_string(), IntoAnalytic::new());
    let mut step7 = step6.attach_standard("ifft".to_string(), IFFT::new());
    let mut step8 = step7.attach_series_reconstructor("reconstructor".to_string());
    let mut dechunker= step8.attach_standard("dechunker".to_string(), OverlapSaveCombiner::new());
    let mut step9 = dechunker.attach_standard("Magnitude".to_string(), ComplexMagnitude::new());
    let mut step10 = step9.attach_standard("demodulator".to_string(), AMDemodulator::new(10.0, 0.5))
        .add_pipeline_sink("audio sink".to_string(), AudioSink::new(2, 44100, aud_sink));

    source.submit_cpu();
    step1.submit_io();
    step2.submit_cpu();
    step3.submit_cpu();
    step4.submit_cpu();
    step5.submit_cpu();
    step6.submit_cpu();
    step7.submit_cpu();
    step8.submit_cpu();
    step9.submit_cpu();
    step10.submit_cpu();
    converter.submit_cpu();
    dechunker.submit_cpu();

    let graph = Arc::new(PipelineGraph::new(build_vector));
    let mut handle = build_topographical_thread_pool(4, 1, graph.clone());

    let rt = tokio::runtime::Runtime::new().unwrap();
    handle.start(&rt);

    println!("Press enter to stop");
    io::stdin().read_line(&mut String::new()).unwrap();
    handle.kill();

    Ok(())
}