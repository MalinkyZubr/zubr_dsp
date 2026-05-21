use serial_test::serial;

#[cfg(test)]
#[serial]
mod tests {
    use zubr_dsp::engine::orchestration_layer::pipeline_hl::Pipeline;
use zubr_dsp::engine::structural::generic_pipeline_node::RunModel;
use zubr_dsp::engine::construction_layer::unfinished_node_builder::PipelineParameters;
use zubr_dsp::engine::construction_layer::node_build_vector::PipelineBuildVector;
    use zubr_dsp::engine::communication_layer::data_management::BufferArray;
    use zubr_dsp::engine::orchestration_layer::scheduler_models::topographical::ThreadPoolTopographicalHandle;
    use zubr_dsp::engine::orchestration_layer::pipeline_graph::PipelineGraph;
    use tokio::runtime::Runtime;
    use crate::infrastructure::test_models::{
        verify_input_output, TestLinearI32Mult, TestSinkI32, TestSourceI32,
    };
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::Arc;
    use zubr_dsp::engine::construction_layer::build::*;
    use zubr_dsp::engine::construction_layer::unfinished_node_builder::UnfinishedNodeBuilder;

    use log::{error, Level};

    use tokio::sync::mpsc::{channel, Receiver};
    use zubr_dsp::initiate_pipeline;

    fn generate_test_pipeline_cpu(async_runtime: Arc<Runtime>) -> (
        Pipeline<ThreadPoolTopographicalHandle>,
        Receiver<i32>,
    ) {
        let (out_send, out_recv) = channel(100);
        
        let pipeline = build_pipeline(
            Box::new(|bv, par| {
                let test_vec: BufferArray<i32, 8> =
                    BufferArray::<i32, 8>::new_with_value([1, 2, 3, 4, 5, 6, 7, 8]);
                let mut source: UnfinishedNodeBuilder<(), i32, 0, 1> = UnfinishedNodeBuilder::<(), i32, 0, 1>::add_pipeline_source(
                    "test source".to_string(),
                    TestSourceI32::new(test_vec),
                    bv.clone(),
                    par.clone(),
                    RunModel::CPU
                );
                let mut step1: UnfinishedNodeBuilder<i32, i32, 1, 1>  = source.attach_standard("test_step".to_string(), TestLinearI32Mult::new(), RunModel::CPU);
                step1.add_pipeline_sink("test_sink".to_string(), TestSinkI32::new(out_send), RunModel::CPU);
            }),
            PipelineParameters::new(64, 5, None),
            async_runtime, 
            false
        );

        (pipeline.unwrap(), out_recv)
    }

    fn generate_test_pipeline_asynchronous(async_runtime: Arc<Runtime>) -> (
        Pipeline<ThreadPoolTopographicalHandle>,
        Receiver<i32>,
    ) {
        let (out_send, out_recv) = channel(100);

        let pipeline = build_pipeline(
            Box::new(|bv, par| {
                let test_vec: BufferArray<i32, 8> =
                    BufferArray::<i32, 8>::new_with_value([1, 2, 3, 4, 5, 6, 7, 8]);
                let mut source: UnfinishedNodeBuilder<(), i32, 0, 1> = UnfinishedNodeBuilder::<(), i32, 0, 1>::add_pipeline_source(
                    "test source".to_string(),
                    TestSourceI32::new(test_vec),
                    bv.clone(),
                    par.clone(),
                    RunModel::IO
                );
                let mut step1: UnfinishedNodeBuilder<i32, i32, 1, 1>  = source.attach_standard("test_step".to_string(), TestLinearI32Mult::new(), RunModel::IO);
                step1.add_pipeline_sink("test_sink".to_string(), TestSinkI32::new(out_send), RunModel::IO);
            }),
            PipelineParameters::new(64, 5, None),
            async_runtime,
            false
        );

        (pipeline.unwrap(), out_recv)
    }
    #[test]
    fn test_linear_pipeline_cpu() {
        let test_vec: BufferArray<i32, 8> =
            BufferArray::<i32, 8>::new_with_value([1, 2, 3, 4, 5, 6, 7, 8]);
        let rt = Arc::new(tokio::runtime::Runtime::new().unwrap());
        initiate_pipeline(Level::Error);

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
        initiate_pipeline(Level::Error);

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
}
