#[cfg(test)]
mod tests {
    use crate::infrastructure::test_models::{verify_input_output, TestLinearI32Mult, TestSinkI32, TestSourceI32};
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::{Arc, OnceLock};
    use std::time::Duration;
    use log::{error, Level};
    use tokio::io::AsyncWriteExt;
    use tokio::sync::mpsc::{channel, Receiver};
    use ZubrDSP::initiate_pipeline;
    use ZubrDSP::pipeline::construction_layer::builders::NodeBuilder;
    use ZubrDSP::pipeline::construction_layer::node_builder::{reset_node_id_counter, IntoWhat, PipelineBuildVector, PipelineParameters};
    use ZubrDSP::pipeline::orchestration_layer::pipeline_graph::PipelineGraph;
    use ZubrDSP::pipeline::orchestration_layer::thread_pool_models::work_stealing_full_buffer::{build_topographical_thread_pool, ThreadPoolTopographicalHandle};

    const TEST_VEC: [i32; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
    fn generate_test_pipeline_cpu() -> (Arc<PipelineGraph>, ThreadPoolTopographicalHandle, Receiver<i32>){
        reset_node_id_counter();
        initiate_pipeline(Level::Warn);
        let build_vector = Rc::new(RefCell::new(PipelineBuildVector::new(
            PipelineParameters::new(16),
        )));
        let mut source: NodeBuilder<_, _, 0, 1, { IntoWhat::CpuNode }> =
            NodeBuilder::<(), i32, 0, 1, { IntoWhat::CpuNode }>::add_cpu_pipeline_source(
                "test_source".to_string(),
                TestSourceI32::new(TEST_VEC.to_vec()),
                build_vector.clone(),
            );

        let (out_send, out_recv) = channel(100);
        let step1: NodeBuilder<_, _, 1, 1, { IntoWhat::CpuNode }> = source
            .attach_standard_cpu("test_step".to_string(), TestLinearI32Mult::new())
            .add_cpu_pipeline_sink(
                "test_sink".to_string(),
                TestSinkI32::new(out_send),
            );
        
        source.submit_cpu();
        step1.submit_cpu();

        let graph = Arc::new(PipelineGraph::new(build_vector));
        let handle = build_topographical_thread_pool(4, 1, graph.clone());

        (graph, handle, out_recv)
    }
    
    fn generate_test_pipeline_asynchronous() -> (Arc<PipelineGraph>, ThreadPoolTopographicalHandle, Receiver<i32>) {
        reset_node_id_counter();
        initiate_pipeline(Level::Warn);
        let build_vector = Rc::new(RefCell::new(PipelineBuildVector::new(
            PipelineParameters::new(16),
        )));
        let mut source: NodeBuilder<_, _, 0, 1, { IntoWhat::IoNode }> =
            NodeBuilder::<(), i32, 0, 1, { IntoWhat::IoNode }>::add_io_pipeline_source(
                "test_source".to_string(),
                TestSourceI32::new(TEST_VEC.to_vec()),
                build_vector.clone(),
            );

        let (out_send, out_recv) = channel(100);
        let step1: NodeBuilder<_, _, 1, 1, { IntoWhat::IoNode }> = source
            .attach_standard_io("test_step".to_string(), TestLinearI32Mult::new())
            .add_io_pipeline_sink(
                "test_sink".to_string(),
                TestSinkI32::new(out_send),
            );

        source.submit_io();
        step1.submit_io();

        let graph = Arc::new(PipelineGraph::new(build_vector));
        let handle = build_topographical_thread_pool(4, 1, graph.clone());

        (graph, handle, out_recv)
    }
    #[test]
    fn test_linear_pipeline_cpu() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        
        rt.block_on(async {
            let (graph, mut handle, mut receiver) = generate_test_pipeline_cpu();
            handle.start(&rt);
            error!("test_linear_pipeline start");
            
            let mut res_vec = Vec::new();
            for _ in 0..8 {
                let output_value = receiver.recv().await.unwrap();
                res_vec.push(output_value);
                error!("Received: {}", output_value);
            }
            error!("input received");
            handle.kill();
            verify_input_output(TEST_VEC.to_vec(), res_vec, |x| x * 2);
        });
    }

    #[test]
    fn test_linear_pipeline_async() {
        let rt = tokio::runtime::Runtime::new().unwrap();

        rt.block_on(async {
            let (graph, mut handle, mut receiver) = generate_test_pipeline_asynchronous();
            handle.start(&rt);
            error!("test_linear_pipeline start");

            let mut res_vec = Vec::new();
            for _ in 0..8 {
                let output_value = receiver.recv().await.unwrap();
                res_vec.push(output_value);
                error!("Received: {}", output_value);
            }
            error!("input received");
            handle.kill();
            verify_input_output(TEST_VEC.to_vec(), res_vec, |x| x * 2);
        });
    }
}