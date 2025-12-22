use crate::pipeline::api::*;
use crate::pipeline::construction_layer::::audio_file_source::AudioFileSource;
use crate::pipeline::endpoints::*;


mod end_to_end_tests {
    use crate::pipeline::endpoints::audio_endpoint::AudioSink;
    use crate::pipeline::orchestration_layer::logging::initialize_logger;
    use super::*;
    use rodio::{OutputStream, OutputStreamBuilder};
    use crate::dsp::filtering::fir::{fir_filter::generate_FIR_impulse_response, windows, windows::*};
    use crate::dsp::filtering::fir::ideal_response::rectangles::{SelectPassRectangular, SidePassRectangular, SidePassType};
    use crate::dsp::filtering::fir::windows::polynomial::WelchWindow;
    use crate::dsp::fft::bit_reversal_optimized::FFTBitReversal;
    use num::Complex;
    use crate::dsp::core::pointwise_arithmetic::PointwiseMultiplier;
    use crate::dsp::system_response::discrete_fd_convolution::FrequencyConvolution;
    use crate::dsp::casting::{ComplexCaster, RealCaster};
    use crate::dsp::filtering::fir::ideal_response::rectangles::SelectPassType::BandPass;
    use crate::dsp::filtering::fir::windows::trig::CosineSumType;
    use crate::dsp::system_response::discrete_td_convolution::DiscreteConvolution;
    use crate::dsp::system_response::system_functions::ImpulseResponse;

    #[test]
    fn test_audio_lpf_playback() {
        initialize_logger();

        log_message("Staring linear pipeline construction".to_string(), Level::Debug);
        
        let lpf_impulse_resp_norm = ImpulseResponse::normalize_to_sum(
            generate_FIR_impulse_response(
            SidePassRectangular::new(300.0, 48000.0, SidePassType::LowPass),
            //trig::CosineSumWindow::new(CosineSumType::Blackman),
            polynomial::WelchWindow{},
            512, 
            48000.0
        ));//, 100.0, 40000.0));
        
        // let lpf_impulse_resp_norm = generate_FIR_impulse_response(
        //     SelectPassRectangular::new(2000.0, 2000.0, 48000.0, BandPass),
        //     polynomial::WelchWindow{},
        //     512, 
        //     48000.0
        // );
        // 

        let mut pipeline = ConstructingPipeline::new(3, 10000, 1, 2, 3, 1000);

        let stream = OutputStreamBuilder::open_default_stream().unwrap();
        let sink = rodio::Sink::connect_new(&stream.mixer());

        NodeBuilder::start_pipeline(
            "test audio source",
            AudioFileSource::new("/home/malinkyzubr/Documents/ZubrDSP/src/pipeline/sources/tests/starstest.wav", 2048, 3),
            &mut pipeline)
            .attach("convolution", DiscreteConvolution::new(2048, lpf_impulse_resp_norm.len(), Some(lpf_impulse_resp_norm)))
        .cap_pipeline(
            "audio sink", AudioSink::new(2, 48000, sink, true)
        );
        let mut pipeline = pipeline.finish_pipeline();
        pipeline.start();

        let time = std::time::Instant::now();
        while pipeline.is_running() {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        assert!(time.elapsed() >= std::time::Duration::from_secs(99));
    }
}