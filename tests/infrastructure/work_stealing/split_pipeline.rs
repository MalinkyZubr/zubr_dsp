use serial_test::serial;

#[cfg(test)]
#[serial]
mod tests {
    use crate::infrastructure::test_models::{
        verify_input_output, TestAdder, TestLinearI32Mult, TestSinkI32, TestSourceI32,
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
        Receiver<i32>,
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
                TestSourceI32::new(test_vec),
                build_vector.clone(),
            );

        let (out_send_1, out_recv_1) = channel(100);
        let (out_send_2, out_recv_2) = channel(100);
        let mut step1: NodeBuilder<_, _, 1, 3> =
            source.attach_standard_cpu("test_step1".to_string(), TestLinearI32Mult::new());
        let step20: NodeBuilder<_, _, 1, 1> =
            step1.attach_standard_cpu("test_step20".to_string(), TestLinearI32Mult::new());
        let step21: NodeBuilder<_, _, 1, 1> =
            step1.attach_standard_cpu("test_step21".to_string(), TestLinearI32Mult::new());
        let step22: NodeBuilder<_, _, 1, 1> =
            step1.attach_standard_cpu("test_step22".to_string(), TestLinearI32Mult::new());

        let mut joint_node: NodeBuilder<i32, i32, 3, 2> =
            NodeBuilder::<i32, i32, 3, 1>::create_cpu_joint_node(
                "test_step3".to_string(),
                TestAdder::new(),
                build_vector.clone(),
            );
        step20.feed_into_cpu(&mut joint_node).submit_cpu();
        step21.feed_into_cpu(&mut joint_node).submit_cpu();
        step22.feed_into_cpu(&mut joint_node).submit_cpu();

        joint_node
            .add_cpu_pipeline_sink(
                "test_sink1".to_string(),
                TestSinkI32::new(out_send_1.clone()),
            )
            .add_cpu_pipeline_sink(
                "test_sink2".to_string(),
                TestSinkI32::new(out_send_2.clone()),
            )
            .submit_cpu();

        source.submit_cpu();
        step1.submit_cpu();

        let graph = Arc::new(PipelineGraph::new(build_vector));
        let handle = build_topographical_thread_pool(4, 1, graph.clone());

        (graph, handle, out_recv_1, out_recv_2)
    }

    #[test]
    fn test_split_pipeline() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let test_vec: BufferArray<i32, 8> =
            BufferArray::<i32, 8>::new_with_value([1, 2, 3, 4, 5, 6, 7, 8]);

        rt.block_on(async {
            let (_graph, mut handle, mut receiver1, mut receiver2) = generate_test_pipeline();
            handle.start(&rt);
            error!("test_linear_pipeline start");

            let mut res_vec_1 = Vec::new();
            let mut res_vec_2 = Vec::new();

            for _ in 0..8 {
                let output_value = receiver1.recv().await.unwrap();
                res_vec_1.push(output_value);
                error!("SINK 1 Received: {}", output_value);
                let output_value = receiver2.recv().await.unwrap();
                res_vec_2.push(output_value);
                error!("SINK 2 Received: {}", output_value);
            }
            handle.kill();
            verify_input_output(
                test_vec,
                BufferArray::new_with_value(res_vec_1.try_into().unwrap()),
                |x| x * 12,
            );
            verify_input_output(
                test_vec,
                BufferArray::new_with_value(res_vec_2.try_into().unwrap()),
                |x| x * 12,
            );
        });
    }
}
