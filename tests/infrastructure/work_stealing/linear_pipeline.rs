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
    use zubr_dsp::engine::data_plane::construction::unfinished_node_builder::UnfinishedNodeBuilder;
    use zubr_dsp::engine::data_plane::construction::unfinished_node_builder::{
        PipelineInterfaceConfiguration, PipelineParameters,
    };
    use zubr_dsp::engine::data_plane::structural::generic_pipeline_node::RunModel;

    use tokio::sync::mpsc::{channel, Receiver};
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
                let mut step1: UnfinishedNodeBuilder<i32, i32, 1, 1> = source.attach_standard(
                    "test_step".to_string(),
                    TestLinearI32Mult::new(),
                    RunModel::CPU,
                );
                step1.add_pipeline_sink(
                    "test_sink".to_string(),
                    TestSinkI32::new(out_send),
                    RunModel::CPU,
                );
            }),
            PipelineParameters::new(
                64,
                4,
                None,
                false,
                PipelineInterfaceConfiguration::Headless,
                16,
                16,
            ),
            async_runtime,
        );

        (pipeline.unwrap(), out_recv)
    }

    fn generate_test_pipeline_asynchronous(
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
                        RunModel::IO,
                    );
                let mut step1: UnfinishedNodeBuilder<i32, i32, 1, 1> = source.attach_standard(
                    "test_step".to_string(),
                    TestLinearI32Mult::new(),
                    RunModel::IO,
                );
                step1.add_pipeline_sink(
                    "test_sink".to_string(),
                    TestSinkI32::new(out_send),
                    RunModel::IO,
                );
            }),
            PipelineParameters::new(
                64,
                4,
                None,
                false,
                PipelineInterfaceConfiguration::Headless,
                16,
                16,
            ),
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
        let rt = Arc::new(tokio::runtime::Runtime::new().unwrap());
        initiate_pipeline(Level::Warn);

        rt.clone().block_on(async {
            let (mut handle, mut receiver) = generate_test_pipeline_asynchronous(rt.clone());
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
            verify_input_output(
                test_vec,
                BufferArray::new_with_value(res_vec.try_into().unwrap()),
                |x| x * 2,
            );
        });
    }
}
