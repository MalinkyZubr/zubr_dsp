#[cfg(test)]
mod tests {
    use crate::infrastructure::test_models::TestSourceI32;
    use crate::infrastructure::test_models::{verify_input_output, TestLinearI32Mult, TestSinkI32};
    use log::{error, Level};
    use std::sync::Arc;
    use tokio::runtime::Runtime;
    use zubr_dsp::engine::build::build_pipeline;
    use zubr_dsp::engine::control_plane::pipeline_hl::Pipeline;
    use zubr_dsp::engine::control_plane::scheduler_models::topographical::ThreadPoolTopographicalHandle;
    use zubr_dsp::engine::data_plane::communication_layer::data_management::BufferArray;
    use zubr_dsp::engine::data_plane::construction::unfinished_node_builder::PipelineInterfaceConfiguration;
    use zubr_dsp::engine::data_plane::construction::unfinished_node_builder::UnfinishedNodeBuilder;
    use zubr_dsp::engine::data_plane::structural::generic_pipeline_node::RunModel;

    use tokio::sync::mpsc::{channel, Receiver};
    use zubr_dsp::dsp::core::const_arithmetic::ConstAdder;
    use zubr_dsp::dsp::core::pointwise_arithmetic::PointwiseSubtractor;
    use zubr_dsp::engine::data_plane::structural::generic_pipeline_node::RunModel::CPU;
    use zubr_dsp::engine::zubr_dsp_config::PipelineParameters;
    use zubr_dsp::initiate_pipeline;

    fn generate_test_pipeline_cpu(
        async_runtime: Arc<Runtime>,
    ) -> (Pipeline<ThreadPoolTopographicalHandle>, Receiver<i32>) {
        let (out_send, out_recv) = channel(100);

        let pipeline = build_pipeline(
            Box::new(|bv, par| {
                let test_vec: BufferArray<i32, 8> =
                    BufferArray::<i32, 8>::new_with_value([1, 2, 3, 4, 5, 6, 7, 8]);
                let mut source: UnfinishedNodeBuilder<(), i32, 0, 1> =
                    UnfinishedNodeBuilder::<(), i32, 0, 1>::add_pipeline_source(
                        "test source".to_string(),
                        TestSourceI32::new(test_vec),
                        bv.clone(),
                        par.clone(),
                        RunModel::CPU,
                    );

                let mut subtractor: UnfinishedNodeBuilder<BufferArray<i32, 1>, BufferArray<i32, 1>, 2, 2> = UnfinishedNodeBuilder::<BufferArray<i32, 1>, BufferArray<i32, 1>, 2, 2>::create_joint_node(
                    "subtractor".to_string(),
                    PointwiseSubtractor::<1>::new(),
                    bv,
                    par,
                    CPU,
                );

                subtractor.attach_standard::<_, 1, 1>("noop".to_string(), ConstAdder::new([0; 1]), CPU)
                    .add_initial_state(BufferArray::new_with_value([0; 1]))
                    .feed_into(&mut subtractor);
                subtractor.attach_series_deconstructor::<1>("deconstructor".to_string())
                    .add_pipeline_sink("sink".to_string(), TestSinkI32::new(out_send), CPU);
                source.attach_series_reconstructor::<1, 1>("reconstructor".to_string())
                    .feed_into(&mut subtractor);
            }),
            PipelineParameters::standard_no_analytics(),
            async_runtime,
        );

        (pipeline.unwrap(), out_recv)
    }

    #[test]
    fn test_linear_pipeline_cpu() {
        let test_vec: BufferArray<i32, 8> =
            BufferArray::<i32, 8>::new_with_value([1, 2, 3, 4, 5, 6, 7, 8]);
        let rt = Arc::new(tokio::runtime::Runtime::new().unwrap());
        initiate_pipeline(Level::Debug);

        rt.clone().block_on(async {
            let (mut handle, mut receiver) = generate_test_pipeline_cpu(rt.clone());
            handle.start();
            error!("test_linear_pipeline start");

            let mut res_vec = Vec::new();
            for _ in 0..8 {
                let output_value = receiver.recv().await.unwrap();
                res_vec.push(output_value);
                error!("Received: {}", output_value);
            }
            error!("input received");
            handle.stop();
            assert_eq!(res_vec, [-1, -3, -6, -10, -15, -21, -28, -36])
        });
    }
}