use std::cell::RefCell;
use std::io;
use std::rc::Rc;
use std::sync::Arc;
use tokio::sync::mpsc::channel;
use zubr_dsp::general::endpoints::audio_endpoint::AudioSink;
use zubr_dsp::general::sources::audio_file_source::AudioFileSource;
use zubr_dsp::general::throttle::Throttle;
use zubr_dsp::pipeline::construction_layer::builders::NodeBuilder;
use zubr_dsp::pipeline::construction_layer::node_builder::{PipelineBuildVector, PipelineParameters};
use zubr_dsp::pipeline::orchestration_layer::pipeline_graph::PipelineGraph;
use zubr_dsp::pipeline::orchestration_layer::thread_pool_models::work_stealing_full_buffer::build_topographical_thread_pool;
use rodio::{OutputStream, OutputStreamBuilder, Sink};


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
        AudioFileSource::<1024>::new(input.as_str().trim(), 10),
        build_vector.clone(),
    );

    let stream = OutputStreamBuilder::open_default_stream().unwrap();
    let sink = Sink::connect_new(stream.mixer());
    let step1: NodeBuilder<_, _, 1, 1> = source
        .attach_standard("test_step".to_string(), Throttle::new(88.2e3))
        .add_pipeline_sink("test_sink".to_string(), AudioSink::new(2, 40100, sink));

    source.submit_cpu();
    step1.submit_io();

    let graph = Arc::new(PipelineGraph::new(build_vector));
    let mut handle = build_topographical_thread_pool(4, 1, graph.clone());

    let rt = tokio::runtime::Runtime::new().unwrap();
    handle.start(&rt);

    println!("Press enter to stop");
    io::stdin().read_line(&mut String::new()).unwrap();
    handle.kill();

    Ok(())
}