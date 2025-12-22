use crate::pipeline::construction_layer::sources::audio_file_source::AudioReadResult::EOF;
use crate::pipeline::interfaces::ODFormat;
use crate::pipeline::interfaces::PipelineStep;
use crate::pipeline::pipeline_traits::Source;
use std::fs::File;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::Decoder;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::{FormatOptions, FormatReader};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
/*
1. empty the output vector in buffer_size increments
2. every time falls below the threshold, read anotehr packet and append to the buffer as a vector

 */

enum AudioReadResult {
    Ok(Vec<f32>),
    EOF,
    Error,
}

pub struct AudioFileSource {
    buffer_size: usize,
    decoder: Box<dyn Decoder>,
    formatter: Box<dyn FormatReader>,
    read_retries: usize,
    pub buffer: Vec<f32>,
    eof_flag: bool,
}

impl AudioFileSource {
    pub fn new(filename: &str, buffer_size: usize, read_retries: usize) -> Self {
        let file = Box::new(File::open(filename.to_string()).unwrap());

        // Create the media source stream using the boxed media source from above.
        let mss = MediaSourceStream::new(file, Default::default());

        // Create a hint to help the format registry guess what format reader is appropriate. In this
        // example we'll leave it empty.
        let hint = Hint::new();

        // Use the default options when reading and decoding.
        let format_opts: FormatOptions = Default::default();
        let metadata_opts: MetadataOptions = Default::default();
        let decoder_opts: DecoderOptions = Default::default();

        // Probe the media source stream for a format.
        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &format_opts, &metadata_opts)
            .unwrap();

        // Get the format reader yielded by the probe operation.
        let mut format = probed.format;

        // Get the default track.
        let track = format.default_track().unwrap();

        // Create a decoder for the track.
        let mut decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &decoder_opts)
            .unwrap();

        Self {
            buffer_size,
            decoder,
            formatter: format,
            buffer: Vec::new(),
            read_retries,
            eof_flag: false,
        }
    }
    fn extract_packet(&mut self) -> AudioReadResult {
        let mut internal_buffer = None;
        let packet = self.formatter.next_packet();

        let packet = match packet {
            Ok(packet) => packet,
            Err(err) => return EOF,
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
impl PipelineStep<(), Vec<f32>> for AudioFileSource {
    fn run_DISO(&mut self) -> Result<ODFormat<Vec<f32>>, String> {
        let mut error_count = 0;

        if self.eof_flag {
            return Err("EOF reached".to_string());
        }
        while self.buffer.len() < self.buffer_size && error_count < self.read_retries {
            match self.extract_packet() {
                AudioReadResult::Error => error_count += 1,
                EOF => break,
                AudioReadResult::Ok(packet_data) => {
                    self.buffer.extend(packet_data);
                    error_count = 0;
                }
            }
        }
        if error_count > self.read_retries {
            Err("Error in reading from audio file".to_string())
        } else if self.buffer.len() >= self.buffer_size {
            let to_return = self.buffer.drain(0..self.buffer_size).collect();
            Ok(ODFormat::Standard(to_return))
        } else {
            let mut to_return: Vec<f32> = self.buffer.drain(0..self.buffer.len()).collect();
            to_return.extend(vec![0.0; self.buffer_size - to_return.len()]);

            self.eof_flag = true;

            Ok(ODFormat::Standard(to_return))
        }
    }
}
impl Source for AudioFileSource {}
