#[cfg(test)]
mod tests {
    use crate::infrastructure::test_models::{verify_input_output, TestLinearI32Mult, TestSinkI32, TestSourceI32};
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::Arc;
    use std::time::Duration;
    use log::Level;
    use tokio::io::AsyncWriteExt;
    use tokio::sync::mpsc::{channel, Receiver};
    use ZubrDSP::initiate_pipeline;
    use ZubrDSP::pipeline::construction_layer::builders::NodeBuilder;
    use ZubrDSP::pipeline::construction_layer::node_builder::{
        IntoWhat, PipelineBuildVector, PipelineParameters,
    };
    use ZubrDSP::pipeline::orchestration_layer::pipeline_graph::PipelineGraph;
    use ZubrDSP::pipeline::orchestration_layer::thread_pool_models::work_stealing_full_buffer::{build_topographical_thread_pool, ThreadPoolTopographicalHandle};

    const TEST_VEC: [i32; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
    fn generate_test_pipeline() -> (Arc<PipelineGraph>, ThreadPoolTopographicalHandle, Receiver<Vec<i32>>){
        initiate_pipeline(Level::Debug);
        let build_vector = Rc::new(RefCell::new(PipelineBuildVector::new(
            PipelineParameters::new(16),
        )));
        let mut source: NodeBuilder<_, _, 0, 1, { IntoWhat::CPU_NODE }> =
            NodeBuilder::<(), i32, 0, 1, { IntoWhat::CPU_NODE }>::add_cpu_pipeline_source(
                "test_source".to_string(),
                TestSourceI32::new(TEST_VEC.to_vec()),
                build_vector.clone(),
            );

        let (out_send, out_recv) = channel(100);
        let step1: NodeBuilder<_, _, 1, 1, { IntoWhat::CPU_NODE }> = source
            .attach_standard_cpu("test_step".to_string(), TestLinearI32Mult::new())
            .add_cpu_pipeline_sink(
                "test_sink".to_string(),
                TestSinkI32::new(out_send, TEST_VEC.len()),
            );

        source.submit_cpu();
        step1.submit_cpu();

        let graph = Arc::new(PipelineGraph::new(build_vector));
        let handle = build_topographical_thread_pool(4, 1, graph.clone());

        (graph, handle, out_recv)
    }
    #[tokio::test]
    async fn test_linear_pipeline() {
        let (graph, mut handle, mut receiver) = generate_test_pipeline();
        handle.start();
        
        let output_value = receiver.recv().await.unwrap();
        handle.kill();
        verify_input_output(TEST_VEC.to_vec(), output_value, |x| x * 2);
    }

    // #[tokio::test]
    // async fn test_example_async() {
    //     // Asynchronous test example
    //     tokio::time::sleep(Duration::from_millis(1)).await;
    //     assert_eq!(2 + 2, 4);
    // }
    // 
    // #[test]
    // fn test_example_with_runtime() {
    //     // Test that needs its own runtime
    //     let rt = tokio::runtime::Runtime::new().unwrap();
    // 
    //     rt.block_on(async {
    //         tokio::time::sleep(Duration::from_millis(1)).await;
    //         assert_eq!(2 + 2, 4);
    //     });
    // }
    // 
    // #[test]
    // fn test_example_isolated_runtime() {
    //     // Test that isolates runtime creation in a separate thread
    //     let handle = std::thread::spawn(|| {
    //         let rt = tokio::runtime::Runtime::new().unwrap();
    // 
    //         rt.block_on(async {
    //             tokio::time::sleep(Duration::from_millis(1)).await;
    //             2 + 2
    //         })
    //     });
    // 
    //     let result = handle.join().unwrap();
    //     assert_eq!(result, 4);
    // }
    // 
    // #[test]
    // #[should_panic(expected = "expected panic message")]
    // fn test_example_panic() {
    //     panic!("expected panic message");
    // }
    // 
    // #[test]
    // #[ignore]
    // fn test_example_ignored() {
    //     // This test will be ignored by default
    //     // Run with: cargo test -- --ignored
    //     assert_eq!(2 + 2, 4);
    // }
}
