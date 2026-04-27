use serial_test::serial;

#[cfg(test)]
#[serial]
mod tests {
    use tokio::runtime::Runtime;
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
    use zubr_dsp::pipeline::orchestration_layer::thread_pool_models::work_stealing_full_buffer::{
        build_topographical_thread_pool, ThreadPoolTopographicalHandle,
    };

    fn generate_test_pipeline(async_runtime: &Runtime) -> (
        Arc<PipelineGraph>,
        ThreadPoolTopographicalHandle,
        Receiver<i32>,
    ) {
        let test_vec: BufferArray<i32, 8> =
            BufferArray::<i32, 8>::new_with_value([1, 2, 3, 4, 5, 6, 7, 8]);
        initiate_pipeline(Level::Warn);
        let build_vector = Rc::new(RefCell::new(PipelineBuildVector::new(
            PipelineParameters::new(16),
        )));
        let mut source: NodeBuilder<_, _, 0, 1> = NodeBuilder::<(), i32, 0, 1>::add_pipeline_source(
            "test_source".to_string(),
            TestSourceI32::new(test_vec),
            build_vector.clone(),
        );

        let (out_send, out_recv) = channel(100);
        let mut step1: NodeBuilder<_, _, 1, 1> = source
            .attach_standard("test_step1".to_string(), TestLinearI32Mult::new());
        let mut step2: NodeBuilder<_, _, 1, 1> = step1.attach_standard("test_step2".to_string(), TestLinearI32Mult::new());
        let mut step3: NodeBuilder<_, _, 1, 1> = step2.attach_standard("test_step3".to_string(), TestLinearI32Mult::new());
        let mut step4: NodeBuilder<_, _, 1, 1> = step3.attach_standard("test_step4".to_string(), TestLinearI32Mult::new());
        let mut step5: NodeBuilder<_, _, 1, 1> = step4.attach_standard("test_step5".to_string(), TestLinearI32Mult::new());
        let mut step6: NodeBuilder<_, _, 1, 1> = step5.attach_standard("test_step6".to_string(), TestLinearI32Mult::new());
        let mut step7: NodeBuilder<_, _, 1, 1> = step6.attach_standard("test_step7".to_string(), TestLinearI32Mult::new());
        let mut step8: NodeBuilder<_, _, 1, 1> = step7.attach_standard("test_step8".to_string(), TestLinearI32Mult::new());
        let mut step9: NodeBuilder<_, _, 1, 1> = step8.attach_standard("test_step9".to_string(), TestLinearI32Mult::new());
        let mut step10: NodeBuilder<_, _, 1, 1> = step9.attach_standard("test_step10".to_string(), TestLinearI32Mult::new());
        let mut step11: NodeBuilder<_, _, 1, 1> = step10.attach_standard("test_step11".to_string(), TestLinearI32Mult::new());
        let mut step12: NodeBuilder<_, _, 1, 1> = step11.attach_standard("test_step12".to_string(), TestLinearI32Mult::new());
        let mut step13: NodeBuilder<_, _, 1, 1> = step12.attach_standard("test_step13".to_string(), TestLinearI32Mult::new());
        let mut step14: NodeBuilder<_, _, 1, 1> = step13.attach_standard("test_step14".to_string(), TestLinearI32Mult::new());
        let mut step15: NodeBuilder<_, _, 1, 1> = step14.attach_standard("test_step15".to_string(), TestLinearI32Mult::new());
        let mut step16: NodeBuilder<_, _, 1, 1> = step15.attach_standard("test_step16".to_string(), TestLinearI32Mult::new());
        let mut step17: NodeBuilder<_, _, 1, 1> = step16.attach_standard("test_step17".to_string(), TestLinearI32Mult::new())
            .add_pipeline_sink("test_sink".to_string(), TestSinkI32::new(out_send));

        source.submit_cpu();
        step1.submit_cpu();
        step2.submit_cpu();
        step3.submit_cpu();
        step4.submit_cpu();
        step5.submit_cpu();
        step6.submit_cpu();
        step7.submit_cpu();
        step8.submit_cpu();
        step9.submit_cpu();
        step10.submit_cpu();
        step11.submit_cpu();
        step12.submit_io();
        step13.submit_cpu();
        step14.submit_cpu();
        step15.submit_cpu();
        step16.submit_cpu();
        step17.submit_cpu();

        let graph = Arc::new(PipelineGraph::new(build_vector));
        let handle = build_topographical_thread_pool(4, 1, graph.clone(), async_runtime, None);

        (graph, handle, out_recv)
    }
    
    #[test]
    fn test_linear_pipeline_cpu() {
        let test_vec: BufferArray<i32, 8> =
            BufferArray::<i32, 8>::new_with_value([1, 2, 3, 4, 5, 6, 7, 8]);
        let rt = tokio::runtime::Runtime::new().unwrap();

        rt.block_on(async {
            let (_graph, mut handle, mut receiver) = generate_test_pipeline(&rt);
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
                |x| x * 131072,
            );
        });
    }
}
