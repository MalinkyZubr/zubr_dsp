use std::cell::RefCell;
use std::io;
use std::rc::Rc;
use std::sync::Arc;
use tokio::sync::mpsc::channel;
use rodio::{OutputStream, OutputStreamBuilder, Sink};
use tokio::runtime::Runtime;
use zubr_dsp::engine::construction_layer::build::build_pipeline;
use zubr_dsp::engine::construction_layer::unfinished_node_builder::{PipelineParameters, UnfinishedNodeBuilder};
use zubr_dsp::engine::orchestration_layer::pipeline_graph::PipelineGraph;
use zubr_dsp::engine::orchestration_layer::pipeline_hl::Pipeline;
use zubr_dsp::engine::orchestration_layer::scheduler_models::topographical::ThreadPoolTopographicalHandle;
use zubr_dsp::engine::structural::generic_pipeline_node::RunModel::{CPU, IO};
use zubr_dsp::general::endpoints::audio_endpoint::AudioSink;
use zubr_dsp::general::sources::audio_file_source::AudioFileSource;
use zubr_dsp::general::throttle::Throttle;

pub fn audio_test() -> Result<(), String> {
    println!("Beginning audio test\nEnter absolute path to mp3:");

    let stream = OutputStreamBuilder::open_default_stream().unwrap();
    let sink = Sink::connect_new(stream.mixer());
    let rt = Arc::new(tokio::runtime::Runtime::new().unwrap());
    let mut pipeline: Pipeline<ThreadPoolTopographicalHandle> = build_pipeline(
        Box::new(|bv, par| {
            let mut input = String::new();
            io::stdin()
                .read_line(&mut input)
                .expect("Failed to read line");
            let mut source: UnfinishedNodeBuilder<_, _, 0, 1> = UnfinishedNodeBuilder::<(), i32, 0, 1>::add_pipeline_source(
                "audio_source".to_string(),
                AudioFileSource::<1024>::new(input.as_str().trim(), 10),
                bv.clone(),
                par,
                CPU
            );
            
            let step1: UnfinishedNodeBuilder<_, _, 1, 1> = source
                .attach_standard("test_step".to_string(), Throttle::new(88.2e3), IO)
                .add_pipeline_sink("test_sink".to_string(), AudioSink::new(2, 41100, sink), CPU);
        }),
        PipelineParameters::new(64, 5, None),
        rt,
        false
    ).unwrap();

    pipeline.start();
    println!("Press enter to stop");
    io::stdin().read_line(&mut String::new()).unwrap();
    pipeline.stop();

    Ok(())
}