pub mod audio_sink_tests {
    use std::f32::consts::PI;
    use std::time;
    use crate::pipeline::endpoints::audio_endpoint::*;
    use rodio::{Decoder, OutputStream, Sink, Source, OutputStreamBuilder, Sample, ChannelCount, SampleRate};
    use thread_priority::*;
    use crate::pipeline::endpoints::rodio_source::*;

    #[test]
    pub fn test_audio_endpoint() {
        set_current_thread_priority(ThreadPriority::Max);
        
        let start = time::Instant::now();
        let stream_handle = OutputStreamBuilder::open_default_stream()
            .expect("open default audio stream");

        let sink = rodio::Sink::connect_new(&stream_handle.mixer());
        
        let freq = 2000.0;
        let sample_rate = 4800.0;

        let mut samples = vec![Sample::from(0.0); 65536000];
        for time in 0..samples.len() {
            samples[time] = Sample::from((time as f32 * 2.0 * PI * freq / sample_rate).sin())
        }

        let source: SourceObject = SourceObject::new(
            samples, ChannelCount::from(1_u16), SampleRate::from(sample_rate as u32)
        );

        //sink.append(source);
        sink.pause();

        std::thread::sleep(std::time::Duration::from_millis(200));
        sink.play();
        sink.sleep_until_end();

        assert!(start.elapsed().as_millis() > 1);
    }
}
