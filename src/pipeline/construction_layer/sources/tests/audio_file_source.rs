#[cfg(test)]
mod audio_file_source_tests {
    use std::env;
    use std::fs::File;
    use crate::pipeline::construction_layer::sources::audio_file_source::AudioFileSource;
    use crate::pipeline::api::*;

    #[test]
    pub fn test_audio_file_source_interleaved() {
        let start = std::time::Instant::now();
        let mut file_source = AudioFileSource::new("/home/malinkyzubr/Documents/ZubrDSP/src/pipeline/sources/pipeline/kolotest.mp3", 1025, 3);
        let samples = file_source.run_DISO();
        
        let mut num_samples = 0;
        
        loop {
            match file_source.run_DISO() {
                Ok(val) => {
                    let unwrapped = val.unwrap_standard();
                    num_samples += unwrapped.len();
                    dbg!(&unwrapped.len());
                }
                Err(err) => {
                    dbg!("EOF reached");
                    break;
                }
            }
        }
        
        dbg!("Done, {}", num_samples);
        
        assert!(start.elapsed().as_millis() > 10);
    }
}