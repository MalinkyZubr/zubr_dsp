use crate::pipeline::api::*;
use crate::pipeline::construction_layer::::audio_file_source::AudioFileSource;
use crate::pipeline::endpoints::*;


mod end_to_end_tests {
    use crate::pipeline::endpoints::audio_endpoint::AudioSink;
    use crate::pipeline::orchestration_layer::logging::initialize_logger;
    use super::*;
    use rodio::{OutputStream, OutputStreamBuilder};

    #[test]
    fn test_audio_file_playback() {
        initialize_logger();

        log_message("Staring linear pipeline construction".to_string(), Level::Debug);

        let mut pipeline = ConstructingPipeline::new(3, 1000, 1, 2, 3, 1000);

        let stream = OutputStreamBuilder::open_default_stream().unwrap();
        let sink = rodio::Sink::connect_new(&stream.mixer());

        NodeBuilder::start_pipeline(
            "test audio source",
            AudioFileSource::new("/home/malinkyzubr/Documents/ZubrDSP/src/pipeline/sources/tests/starstest.wav", 2048, 3),
            &mut pipeline)
            .cap_pipeline(
            "audio sink", AudioSink::new(1, 40000, sink, true)
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