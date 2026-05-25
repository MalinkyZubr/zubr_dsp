use std::cell::RefCell;
use std::io;
use std::rc::Rc;
use std::sync::Arc;
use log::Level;
use num::Complex;
use rodio::{OutputStreamBuilder, Sink};
use tokio::runtime::Builder;
use zubr_dsp::dsp::core::complex_magnitude::ComplexMagnitude;
use zubr_dsp::dsp::core::converters::{ComplexToReal, RealToComplex};
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
use zubr_dsp::engine::data_plane::::data_management::BufferArray;
use zubr_dsp::engine::data_plane::::unfinished_node_builder::{PipelineParameters, UnfinishedNodeBuilder};
use zubr_dsp::dsp::sampling::simple_downsample::*;
use zubr_dsp::engine::build::build_pipeline;
use zubr_dsp::engine::control_plane::pipeline_hl::Pipeline;
use zubr_dsp::engine::control_plane::scheduler_models::topographical::ThreadPoolTopographicalHandle;
use zubr_dsp::engine::data_plane::::generic_pipeline_node::RunModel::{CPU, IO};
use zubr_dsp::initiate_pipeline;

pub fn am_end_to_end_test() -> Result<(), String> {
    println!("Beginning simulated AM end to end test\nEnter absolute path to mp3:");

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
            let (mut breaker, mut combiner) = generate_overlap_save_steps::<
                f32, 2048, 512, 256, 8, 128
            >();

            let mut input = String::new();
            io::stdin()
                .read_line(&mut input)
                .expect("Failed to read line");

            let mut source: UnfinishedNodeBuilder<_, _, 0, 1> = UnfinishedNodeBuilder::<(), i32, 0, 1>::add_pipeline_source(
                "audio_source".to_string(),
                AudioFileSource::<2048>::new(input.as_str().trim(), 10),
                bv.clone(),
                par,
                CPU
            );
            let carrier_amplitude = 10.0;
            let carrier_frequency = 1e8;
            let modulation_index = 0.5;
            let sample_frequency = 2.5e8;
            
            source
                .attach_standard::<_, 1, 1>("throttle".to_string(), Throttle::new(88.2e3), IO)
                .attach_standard::<_, 1, 1>("AM Modulator".to_string(), AMModulator::new(carrier_amplitude, carrier_frequency, modulation_index, sample_frequency), CPU)
                .attach_standard::<_, 1, 1>("Chunker".to_string(), breaker, CPU)
                .attach_series_deconstructor::<1>("deconstructor".to_string())
                .attach_standard::<_, 1, 1>("to complex".to_string(), RealToComplex::new(), CPU)
                .attach_standard::<_, 1, 1>("fft".to_string(), FFT::new(), CPU)
                .attach_standard::<_, 1, 1>("analytic filter".to_string(), FIRFilter::new(tf_analytic::<_, 512>()), CPU)
                .attach_standard::<_, 1, 1>("ifft".to_string(), IFFT::new(), CPU)
                .attach_standard::<_, 1, 1>("to real".to_string(), ComplexMagnitude::new(), CPU)
                .attach_series_reconstructor::<1, 8>("reconstructor".to_string())
                .attach_standard::<_, 1, 1>("dechunker".to_string(), combiner, CPU)
                .attach_standard::<_, 1, 1>("demodulator".to_string(), AMDemodulator::new(carrier_amplitude, modulation_index), CPU)
                .add_pipeline_sink("audio sink".to_string(), AudioSink::new(2, 44100, aud_sink), CPU);
        }),
        PipelineParameters::new(16, 5, None),
        rt,
        false
    ).unwrap();
    
    pipeline.start();

    println!("Press enter to stop");
    io::stdin().read_line(&mut String::new()).unwrap();
    pipeline.stop();

    Ok(())
}