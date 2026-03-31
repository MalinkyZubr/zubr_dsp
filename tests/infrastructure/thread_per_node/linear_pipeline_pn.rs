use serial_test::serial;

#[cfg(test)]
#[serial]
mod tests {
    use crate::infrastructure::test_models::{
        verify_input_output, TestLinearI32Mult, TestSinkI32, TestSourceI32,
    };
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::Arc;
    use zubr_dsp::pipeline::communication_layer::data_management::BufferArray;

    use log::{error, Level};

    use tokio::sync::mpsc::{channel, Receiver};
    use zubr_dsp::initiate_pipeline;
    use zubr_dsp::pipeline::construction_layer::builders::NodeBuilder;
    use zubr_dsp::pipeline::construction_layer::node_builder::{
        PipelineBuildVector, PipelineParameters,
    };
    use zubr_dsp::pipeline::orchestration_layer::pipeline_graph::PipelineGraph;
    use zubr_dsp::pipeline::orchestration_layer::thread_pool_models::thread_per_node::{
        build_per_node_thread_pool, ThreadPoolPerNodeHandle,
    };

    fn generate_test_pipeline_cpu() -> (Arc<PipelineGraph>, ThreadPoolPerNodeHandle, Receiver<i32>)
    {
        let test_vec: BufferArray<i32, 8> =
            BufferArray::<i32, 8>::new_with_value([1, 2, 3, 4, 5, 6, 7, 8]);
        initiate_pipeline(Level::Warn);
        let build_vector = Rc::new(RefCell::new(PipelineBuildVector::new(
            PipelineParameters::new(16),
        )));
        let mut source: NodeBuilder<_, _, 0, 1> =
            NodeBuilder::<(), i32, 0, 1>::add_cpu_pipeline_source(
                "test_source".to_string(),
                TestSourceI32::new(test_vec),
                build_vector.clone(),
            );

        let (out_send, out_recv) = channel(100);
        let step1: NodeBuilder<_, _, 1, 1> = source
            .attach_standard_cpu("test_step".to_string(), TestLinearI32Mult::new())
            .add_cpu_pipeline_sink("test_sink".to_string(), TestSinkI32::new(out_send));

        source.submit_cpu();
        step1.submit_cpu();

        let graph = Arc::new(PipelineGraph::new(build_vector));
        let handle = build_per_node_thread_pool(graph.clone());

        (graph, handle, out_recv)
    }

    fn generate_test_pipeline_asynchronous(
    ) -> (Arc<PipelineGraph>, ThreadPoolPerNodeHandle, Receiver<i32>) {
        let test_vec: BufferArray<i32, 8> =
            BufferArray::<i32, 8>::new_with_value([1, 2, 3, 4, 5, 6, 7, 8]);
        initiate_pipeline(Level::Warn);
        let build_vector = Rc::new(RefCell::new(PipelineBuildVector::new(
            PipelineParameters::new(16),
        )));
        let mut source: NodeBuilder<_, _, 0, 1> =
            NodeBuilder::<(), i32, 0, 1>::add_io_pipeline_source(
                "test_source".to_string(),
                TestSourceI32::new(test_vec),
                build_vector.clone(),
            );

        let (out_send, out_recv) = channel(100);
        let step1: NodeBuilder<_, _, 1, 1> = source
            .attach_standard_io("test_step".to_string(), TestLinearI32Mult::new())
            .add_io_pipeline_sink("test_sink".to_string(), TestSinkI32::new(out_send));

        source.submit_io();
        step1.submit_io();

        let graph = Arc::new(PipelineGraph::new(build_vector));
        let handle = build_per_node_thread_pool(graph.clone());

        (graph, handle, out_recv)
    }
    #[test]
    fn test_linear_pipeline_cpu() {
        let test_vec: BufferArray<i32, 8> =
            BufferArray::<i32, 8>::new_with_value([1, 2, 3, 4, 5, 6, 7, 8]);
        let rt = tokio::runtime::Runtime::new().unwrap();

        rt.block_on(async {
            let (_graph, mut handle, mut receiver) = generate_test_pipeline_cpu();
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
            verify_input_output(
                test_vec,
                BufferArray::new_with_value(res_vec.try_into().unwrap()),
                |x| x * 2,
            );
        });
    }

    #[test]
    fn test_linear_pipeline_async() {
        let test_vec: BufferArray<i32, 8> =
            BufferArray::<i32, 8>::new_with_value([1, 2, 3, 4, 5, 6, 7, 8]);
        let rt = tokio::runtime::Runtime::new().unwrap();

        rt.block_on(async {
            let (_graph, mut handle, mut receiver) = generate_test_pipeline_asynchronous();
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
            verify_input_output(
                test_vec,
                BufferArray::new_with_value(res_vec.try_into().unwrap()),
                |x| x * 2,
            );
        });
    }
}
