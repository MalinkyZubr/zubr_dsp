#[cfg(test)]
mod tests {
    use crate::infrastructure::test_models::{verify_input_output, TestAdder, TestLinearI32Mult, TestSinkI32, TestSinkI32Vec, TestSourceI32, TestSourceI32Vec};
    use log::{error, Level};
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::{Arc, OnceLock};
    use std::time::Duration;
    use tokio::io::AsyncWriteExt;
    use tokio::sync::mpsc::{channel, Receiver};
    use ZubrDSP::initiate_pipeline;
    use ZubrDSP::pipeline::construction_layer::builders::NodeBuilder;
    use ZubrDSP::pipeline::construction_layer::node_builder::{
        reset_node_id_counter, IntoWhat, PipelineBuildVector, PipelineParameters,
    };
    use ZubrDSP::pipeline::orchestration_layer::pipeline_graph::PipelineGraph;
    use ZubrDSP::pipeline::orchestration_layer::thread_pool_models::work_stealing_full_buffer::{
        build_topographical_thread_pool, ThreadPoolTopographicalHandle,
    };

    const TEST_VEC: [i32; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
    fn generate_test_pipeline() -> (
        Arc<PipelineGraph>,
        ThreadPoolTopographicalHandle,
        Receiver<Vec<i32>>, Receiver<Vec<i32>>
    ) {
        reset_node_id_counter();
        initiate_pipeline(Level::Debug);
        let build_vector = Rc::new(RefCell::new(PipelineBuildVector::new(
            PipelineParameters::new(16),
        )));
        let mut source: NodeBuilder<_, _, 0, 1, { IntoWhat::CpuNode }> =
            NodeBuilder::<(), i32, 0, 1, { IntoWhat::CpuNode }>::add_cpu_pipeline_source(
                "test_source".to_string(),
                TestSourceI32Vec::new(TEST_VEC.to_vec()),
                build_vector.clone(),
            );

        let (out_send_1, out_recv_1) = channel(100);
        let (out_send_2, out_recv_2) = channel(100);

        let interleaved_separator = source.attach_interleaved_separator::<2>("separator 1".to_string())
            .add_cpu_pipeline_sink("channel 1 sink".to_string(), TestSinkI32Vec::new(out_send_1))
            .add_cpu_pipeline_sink("channel 2 sink".to_string(), TestSinkI32Vec::new(out_send_2));
        
        source.submit_cpu();
        interleaved_separator.submit_interleaved_separator();

        let graph = Arc::new(PipelineGraph::new(build_vector));
        let handle = build_topographical_thread_pool(4, 1, graph.clone());

        (graph, handle, out_recv_1, out_recv_2)
    }

    #[test]
    fn test_linear_pipeline_cpu() {
        let rt = tokio::runtime::Runtime::new().unwrap();

        rt.block_on(async {
            let (_graph, mut handle, mut receiver1, mut receiver2) = generate_test_pipeline();
            handle.start(&rt);
            error!("test_linear_pipeline start");

            assert_eq!(receiver1.recv().await.unwrap(), vec![1, 3, 5, 7]);
            assert_eq!(receiver2.recv().await.unwrap(), vec![2, 4, 6, 8]);
        });
    }
}
