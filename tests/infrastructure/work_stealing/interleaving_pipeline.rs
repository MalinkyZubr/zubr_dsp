#[cfg(test)]
mod tests {
    use crate::infrastructure::test_models::{TestSinkI32Vec, TestSourceI32Vec};
    use log::{error, Level};
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::Arc;

    use tokio::sync::mpsc::{channel, Receiver};
    use zubr_dsp::initiate_pipeline;
    use zubr_dsp::pipeline::communication_layer::data_management::BufferArray;
    use zubr_dsp::pipeline::construction_layer::builders::NodeBuilder;
    use zubr_dsp::pipeline::construction_layer::node_builder::{
        PipelineBuildVector, PipelineParameters,
    };
    use zubr_dsp::pipeline::orchestration_layer::pipeline_graph::PipelineGraph;
    use zubr_dsp::pipeline::orchestration_layer::thread_pool_models::work_stealing_full_buffer::{
        build_topographical_thread_pool, ThreadPoolTopographicalHandle,
    };

    fn generate_test_pipeline() -> (
        Arc<PipelineGraph>,
        ThreadPoolTopographicalHandle,
        Receiver<BufferArray<i32, 4>>,
        Receiver<BufferArray<i32, 4>>,
    ) {
        let test_vec: BufferArray<i32, 8> =
            BufferArray::<i32, 8>::new_with_value([1, 2, 3, 4, 5, 6, 7, 8]);
        initiate_pipeline(Level::Debug);
        let build_vector = Rc::new(RefCell::new(PipelineBuildVector::new(
            PipelineParameters::new(16),
        )));
        let mut source: NodeBuilder<_, _, 0, 1> =
            NodeBuilder::<(), i32, 0, 1>::add_cpu_pipeline_source(
                "test_source".to_string(),
                TestSourceI32Vec::new(test_vec),
                build_vector.clone(),
            );

        let (out_send_1, out_recv_1) = channel(100);
        let (out_send_2, out_recv_2) = channel(100);

        let interleaved_separator: NodeBuilder<BufferArray<i32, 8>, BufferArray<i32, 4>, 1, 2> =
            source
                .attach_interleaved_separator::<2>("separator 1".to_string())
                .add_cpu_pipeline_sink(
                    "channel 1 sink".to_string(),
                    TestSinkI32Vec::new(out_send_1),
                )
                .add_cpu_pipeline_sink(
                    "channel 2 sink".to_string(),
                    TestSinkI32Vec::new(out_send_2),
                );

        source.submit_cpu();
        interleaved_separator.submit_interleaved_separator();

        let graph = Arc::new(PipelineGraph::new(build_vector));
        let handle = build_topographical_thread_pool(4, 1, graph.clone());

        (graph, handle, out_recv_1, out_recv_2)
    }

    #[test]
    fn test_interleaving_pipeline() {
        let rt = tokio::runtime::Runtime::new().unwrap();

        rt.block_on(async {
            let (_graph, mut handle, mut receiver1, mut receiver2) = generate_test_pipeline();
            handle.start(&rt);
            error!("test_linear_pipeline start");

            let vv1: [i32; 4] = *receiver1.recv().await.unwrap().read();
            let vv11: [i32; 4] = [1, 3, 5, 7];

            let vv2: [i32; 4] = *receiver2.recv().await.unwrap().read();
            let vv22: [i32; 4] = [2, 4, 6, 8];
            assert_eq!(vv1, vv11);
            assert_eq!(vv2, vv22);
        });
    }
}
