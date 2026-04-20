use std::cell::RefCell;
use std::io;
use std::rc::Rc;
use std::sync::Arc;
use num::Complex;
use rodio::{OutputStreamBuilder, Sink};
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
use zubr_dsp::pipeline::communication_layer::data_management::BufferArray;
use zubr_dsp::pipeline::construction_layer::builders::NodeBuilder;
use zubr_dsp::pipeline::construction_layer::node_builder::{PipelineBuildVector, PipelineParameters};
use zubr_dsp::pipeline::orchestration_layer::pipeline_graph::PipelineGraph;
use zubr_dsp::pipeline::orchestration_layer::thread_pool_models::work_stealing_full_buffer::build_topographical_thread_pool;
use zubr_dsp::dsp::sampling::simple_downsample::*;


pub fn am_end_to_end_test() -> Result<(), String> {
    println!("Beginning simulated AM end to end test\nEnter absolute path to mp3:");
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");

    let build_vector = Rc::new(RefCell::new(PipelineBuildVector::new(
        PipelineParameters::new(1),
    )));
    let mut source: NodeBuilder<_, _, 0, 1> = NodeBuilder::<(), i32, 0, 1>::add_pipeline_source(
        "audio_source".to_string(),
        AudioFileSource::<4096>::new(input.as_str().trim(), 10),
        build_vector.clone(),
    );

    let stream = OutputStreamBuilder::open_default_stream().unwrap();
    let aud_sink = Sink::connect_new(stream.mixer());
    let mut step1: NodeBuilder<_, _, 1, 1> = source
        .attach_standard("test_step".to_string(), Throttle::new(88.2e3));
    let mut step2: NodeBuilder<_, _, 1, 1> = step1.attach_standard("AmModulator".to_string(), AMModulator::<4096>::new(
        10.0, 1000000.0, 0.5, 2500000.0
    ));
    let (mut breaker, mut combiner) = generate_overlap_save_steps::<
        f32, 4096, 512, 256, 16, 128
    >();
    
    let mut step3: NodeBuilder<_, _, 1, 1> = step2.attach_standard("Chunker".to_string(), breaker);
    let mut step4 = step3.attach_series_deconstructor::<1>("deconstructor".to_string());
    let mut converter: NodeBuilder<_, _, 1, 1> = step4.attach_standard("converter".to_string(), RealToComplex::new());
    let mut step5: NodeBuilder<_, _, 1, 1> = converter.attach_standard("fft".to_string(), FFT::new());
    let mut step6: NodeBuilder<_, _, 1, 1> = step5.attach_standard("analytic filter".to_string(), FIRFilter::<_, 512>::new(tf_analytic::<f32, 128>()));
    let mut step7: NodeBuilder<_, _, 1, 1> = step6.attach_standard("ifft".to_string(), IFFT::new());
    let mut step8 = step7.attach_series_reconstructor::<1, 16>("reconstructor".to_string());
    let mut dechunker: NodeBuilder<_, _, 1, 1>= step8.attach_standard("dechunker".to_string(), combiner);
    let mut step9: NodeBuilder<_, _, 1, 1> = dechunker.attach_standard("Magnitude".to_string(), ComplexMagnitude::new());
    let mut downsampler: NodeBuilder<_, _, 1, 1> = step9.attach_standard("downsampler".to_string(), SimpleDownsampler::<4096, 36>::new());
    let mut step10: NodeBuilder<_, _, 1, 1> = downsampler.attach_standard("demodulator".to_string(), AMDemodulator::new(10.0, 0.5))
        .add_pipeline_sink("audio sink".to_string(), AudioSink::new(2, 44100, aud_sink));

    source.submit_cpu();
    step1.submit_io();
    step2.submit_cpu();
    step3.submit_cpu();
    step4.submit_series_deconstructor();
    step5.submit_cpu();
    step6.submit_cpu();
    step7.submit_cpu();
    step8.submit_series_reconstructor();
    step9.submit_cpu();
    step10.submit_cpu();
    converter.submit_cpu();
    downsampler.submit_cpu();
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