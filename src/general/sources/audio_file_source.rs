use crate::pipeline::communication_layer::data_management::*;
use crate::pipeline::construction_layer::node_types::pipeline_step::PipelineStep;
use crate::pipeline::construction_layer::pipeline_traits::*;
use std::fs::File;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::Decoder;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::{FormatOptions, FormatReader};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

enum AudioReadResult {
    Ok(Vec<f32>),
    EOF,
    Error,
}

pub struct AudioFileSource<const BUFFER_SIZE: usize> {
    filename: String,
    decoder: Box<dyn Decoder>,
    formatter: Box<dyn FormatReader>,
    read_retries: usize,
    overflow_buffer: Vec<f32>,
}

impl<const BUFFER_SIZE: usize> AudioFileSource<BUFFER_SIZE> {
    pub fn new(filename: &str, read_retries: usize) -> Self {
        let (decoder, formatter) = Self::create_audio_context(filename);
        
        Self {
            filename: filename.to_string(),
            decoder,
            formatter,
            read_retries,
            overflow_buffer: Vec::new(),
        }
    }
    
    fn create_audio_context(filename: &str) -> (Box<dyn Decoder>, Box<dyn FormatReader>) {
        let file = Box::new(File::open(filename).unwrap());
        let mss = MediaSourceStream::new(file, Default::default());
        let hint = Hint::new();
        let format_opts: FormatOptions = Default::default();
        let metadata_opts: MetadataOptions = Default::default();
        let decoder_opts: DecoderOptions = Default::default();

        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &format_opts, &metadata_opts)
            .unwrap();

        let format = probed.format;
        let track = format.default_track().unwrap();
        let decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &decoder_opts)
            .unwrap();

        (decoder, format)
    }
    
    fn restart_file(&mut self) {
        let (new_decoder, new_formatter) = Self::create_audio_context(&self.filename);
        self.decoder = new_decoder;
        self.formatter = new_formatter;
    }

    fn extract_packet(&mut self) -> AudioReadResult { // super inefficient due to allocations but ill deal with that later. Silly memory usage
        let mut internal_buffer = None;
        let packet = self.formatter.next_packet();

        let packet = match packet {
            Ok(packet) => packet,
            Err(_) => return AudioReadResult::EOF,
        };

        match self.decoder.decode(&packet) {
            Ok(audio_buf) => {
                if internal_buffer.is_none() {
                    let spec = *audio_buf.spec();
                    let duration = audio_buf.capacity() as u64;
                    internal_buffer = Some(SampleBuffer::<f32>::new(duration, spec))
                }
                if let Some(buf) = &mut internal_buffer {
                    buf.copy_interleaved_ref(audio_buf);
                }

                let vectorized_buffer = internal_buffer.unwrap().samples().to_vec();
                AudioReadResult::Ok(vectorized_buffer)
            }
            Err(_) => AudioReadResult::Error,
        }
    }
}

impl<const BUFFER_SIZE: usize> PipelineStep<(), BufferArray<f32, BUFFER_SIZE>, 0> 
    for AudioFileSource<BUFFER_SIZE> 
{
    fn run_cpu(
        &mut self,
        _input: &mut [DataWrapper<()>; 0],
        output: &mut DataWrapper<BufferArray<f32, BUFFER_SIZE>>,
    ) -> Result<(), ()> {
        let mut error_count = 0;
        let mut start_index = 0;
        
        output.read().read_mut().copy_from_slice(self.overflow_buffer.as_slice());
        start_index = self.overflow_buffer.len();
        
        while start_index < BUFFER_SIZE && error_count < self.read_retries {
            match self.extract_packet() {
                AudioReadResult::Error => error_count += 1,
                AudioReadResult::EOF => {
                    // Restart the file when EOF is reached
                    self.restart_file();
                    error_count = 0; // Reset error count after restart
                }
                AudioReadResult::Ok(packet_data) => {
                    let slice = &mut output.read().read_mut()[start_index..packet_data.len() + start_index];
                    slice.copy_from_slice(&packet_data.as_slice()[..BUFFER_SIZE - start_index]);
                    
                    if BUFFER_SIZE - start_index < packet_data.len() {
                        self.overflow_buffer.copy_from_slice(&packet_data.as_slice()[BUFFER_SIZE - start_index..]);
                        start_index = BUFFER_SIZE;
                    }
                    
                    error_count = 0;
                }
            }
        }

        if error_count >= self.read_retries {
            return Err(());
        }

        Ok(())
    }
}


impl<const BUFFER_SIZE: usize> Source for AudioFileSource<BUFFER_SIZE> {}