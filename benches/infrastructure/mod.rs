#![feature(test)]

extern crate test;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use test::{black_box, Bencher};
use tokio::sync::mpsc::{channel, Receiver, Sender};

use zubr_dsp::pipeline::construction_layer::builders::NodeBuilder;
use zubr_dsp::pipeline::construction_layer::node_builder::{
    IntoWhat, PipelineBuildVector, PipelineParameters,
};
use zubr_dsp::pipeline::construction_layer::node_types::pipeline_step::PipelineStep;
use zubr_dsp::pipeline::construction_layer::pipeline_traits::{Sink, Source};
use zubr_dsp::pipeline::orchestration_layer::pipeline_graph::PipelineGraph;
use zubr_dsp::pipeline::orchestration_layer::thread_pool_models::thread_per_node::{
    build_per_node_thread_pool, ThreadPoolPerNodeHandle,
};
use zubr_dsp::pipeline::orchestration_layer::thread_pool_models::work_stealing_full_buffer::{
    build_topographical_thread_pool, ThreadPoolTopographicalHandle,
};

#[derive(Debug)]
struct BenchSource {
    cur: i32,
}
impl BenchSource {
    fn new() -> Self {
        Self { cur: 0 }
    }
}
#[async_trait::async_trait]
impl PipelineStep<(), i32, 0> for BenchSource {
    fn run_cpu(&mut self, _input: [(); 0]) -> Result<i32, ()> {
        let out = self.cur;
        self.cur = self.cur.wrapping_add(1);
        Ok(out)
    }
}
impl Source for BenchSource {}

#[derive(Debug)]
struct BenchMul2;
#[async_trait::async_trait]
impl PipelineStep<i32, i32, 1> for BenchMul2 {
    fn run_cpu(&mut self, input: [i32; 1]) -> Result<i32, ()> {
        Ok(input[0].wrapping_mul(2))
    }
}

#[derive(Debug)]
struct BenchSink {
    tx: Sender<i32>,
}
impl BenchSink {
    fn new(tx: Sender<i32>) -> Self {
        Self { tx }
    }
}
#[async_trait::async_trait]
impl PipelineStep<i32, (), 1> for BenchSink {
    fn run_cpu(&mut self, input: [i32; 1]) -> Result<(), ()> {
        // best-effort; if the receiver lags behind, don't block the pipeline
        let _ = self.tx.try_send(input[0]);
        Ok(())
    }
}
impl Sink for BenchSink {}

fn build_linear_pipeline() -> (Arc<PipelineGraph>, Receiver<i32>) {
    let build_vector = Rc::new(RefCell::new(PipelineBuildVector::new(
        PipelineParameters::new(1024),
    )));

    let mut source: NodeBuilder<_, _, 0, 1, { IntoWhat::CpuNode }> =
        NodeBuilder::<(), i32, 0, 1, { IntoWhat::CpuNode }>::add_cpu_pipeline_source(
            "bench_source".to_string(),
            BenchSource::new(),
            build_vector.clone(),
        );

    let (tx, rx) = channel(1024);

    let step: NodeBuilder<_, _, 1, 1, { IntoWhat::CpuNode }> = source
        .attach_standard_cpu("bench_mul2".to_string(), BenchMul2)
        .add_cpu_pipeline_sink("bench_sink".to_string(), BenchSink::new(tx));

    source.submit_cpu();
    step.submit_cpu();

    let graph = Arc::new(PipelineGraph::new(build_vector));
    (graph, rx)
}

fn drain_outputs(rx: &mut Receiver<i32>, n: u64) {
    for _ in 0..n {
        rx.blocking_recv().expect("pipeline output channel closed");
    }
}

#[bench]
fn bench_work_stealing_linear(b: &mut Bencher) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let (graph, mut rx) = build_linear_pipeline();
    let mut handle: ThreadPoolTopographicalHandle = build_topographical_thread_pool(3, 1, graph);

    handle.start(&rt);

    b.iter(|| {
        let v = rx.blocking_recv().expect("pipeline output channel closed");
        black_box(v)
    });

    handle.kill();
}

#[bench]
fn bench_thread_per_node_linear(b: &mut Bencher) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let (graph, mut rx) = build_linear_pipeline();
    let mut handle: ThreadPoolPerNodeHandle = build_per_node_thread_pool(graph);

    handle.start(&rt);

    b.iter(|| {
        let v = rx.blocking_recv().expect("pipeline output channel closed");
        black_box(v)
    });

    handle.kill();
}