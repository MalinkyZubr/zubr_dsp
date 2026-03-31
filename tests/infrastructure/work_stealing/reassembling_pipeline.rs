#[cfg(test)]
#[serial_test::serial]
mod tests {
    use crate::infrastructure::test_models::{
        verify_input_output, TestLinearI32Mult, TestSinkI32, TestSinkI32Vec, TestSourceI32Vec,
    };
    use log::{error, Level};
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::Arc;
    use zubr_dsp::pipeline::communication_layer::data_management::BufferArray;

    use tokio::sync::mpsc::{channel, Receiver};
    use zubr_dsp::initiate_pipeline;
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
        Receiver<i32>,
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

        let deconstructor =
            source.attach_series_deconstructor::<2>("test deconstructor".to_string());
        let mut deconstructor = deconstructor
            .add_cpu_pipeline_sink("test_sink_1".to_string(), TestSinkI32::new(out_send_1));
        let mut step1 = deconstructor
            .attach_standard_cpu::<_, 1, 1>("test node 1".to_string(), TestLinearI32Mult::new());
        let reconstructor = step1
            .attach_series_reconstructor::<1, 4>("test reconstructor".to_string())
            .add_cpu_pipeline_sink("vec sink".to_string(), TestSinkI32Vec::new(out_send_2));

        deconstructor.submit_series_deconstructor();
        step1.submit_cpu();
        source.submit_cpu();
        reconstructor.submit_series_reconstructor();

        let graph = Arc::new(PipelineGraph::new(build_vector));
        let handle = build_topographical_thread_pool(4, 1, graph.clone());

        (graph, handle, out_recv_1, out_recv_2)
    }

    #[test]
    fn test_reassembling_pipeline_cpu() {
        let test_vec: BufferArray<i32, 8> =
            BufferArray::<i32, 8>::new_with_value([1, 2, 3, 4, 5, 6, 7, 8]);
        let rt = tokio::runtime::Runtime::new().unwrap();

        rt.block_on(async {
            let (_graph, mut handle, mut receiver1, mut receiver2) = generate_test_pipeline();
            handle.start(&rt);
            error!("test_linear_pipeline start");

            let mut res_vec_1 = Vec::new();
            let mut res_vec_2 = Vec::new();

            for _ in 0..2 {
                for _ in 0..4 {
                    let output_value = receiver1.recv().await.unwrap();
                    res_vec_1.push(output_value);
                    error!("SINK 1 Received: {}", output_value);
                }
                let output_value = receiver2.recv().await.unwrap();
                res_vec_2.push(output_value.clone());
            }
            handle.kill();
            verify_input_output(
                test_vec,
                BufferArray::new_with_value(res_vec_1.try_into().unwrap()),
                |x| x,
            );

            let res_vec_t: Vec<[i32; 4]> = res_vec_2.iter().map(|x| *x.read()).collect();
            let res_vec_t: [[i32; 4]; 2] = res_vec_t.try_into().unwrap();
            assert_eq!(res_vec_t, [[2, 4, 6, 8], [10, 12, 14, 16]]);
        });
    }
}
