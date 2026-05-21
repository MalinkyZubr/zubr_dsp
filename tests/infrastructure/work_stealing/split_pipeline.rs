use serial_test::serial;

#[cfg(test)]
#[serial]
mod tests {
    use zubr_dsp::engine::construction_layer::unfinished_node_builder::PipelineParameters;
use zubr_dsp::engine::structural::generic_pipeline_node::RunModel;
use zubr_dsp::engine::construction_layer::unfinished_node_builder::UnfinishedNodeBuilder;
use zubr_dsp::engine::communication_layer::data_management::BufferArray;
use zubr_dsp::engine::construction_layer::build::build_pipeline;
use zubr_dsp::engine::orchestration_layer::scheduler_models::topographical::ThreadPoolTopographicalHandle;
use zubr_dsp::engine::orchestration_layer::pipeline_hl::Pipeline;
use tokio::runtime::Runtime;
use crate::infrastructure::test_models::{
        verify_input_output, TestAdder, TestLinearI32Mult, TestSinkI32, TestSourceI32,
    };
    use log::{error, Level};
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::Arc;

    use tokio::sync::mpsc::{channel, Receiver};
    use zubr_dsp::initiate_pipeline;

    fn generate_test_pipeline(async_runtime: Arc<Runtime>) -> (
        Pipeline<ThreadPoolTopographicalHandle>,
        Receiver<i32>,
        Receiver<i32>,
    ) {
        initiate_pipeline(Level::Debug);

        let (out_send_1, out_recv_1) = channel(100);
        let (out_send_2, out_recv_2) = channel(100);
        
        let out_send_1_clone = out_send_1.clone();
        let out_send_2_clone = out_send_2.clone();
        
        let pipeline = build_pipeline(
            Box::new(
                |bv, par| {
                    let test_vec: BufferArray<i32, 8> =
                        BufferArray::<i32, 8>::new_with_value([1, 2, 3, 4, 5, 6, 7, 8]);

                    let mut source: UnfinishedNodeBuilder<_, _, 0, 1> = UnfinishedNodeBuilder::<(), i32, 0, 1>::add_pipeline_source(
                        "test_source".to_string(),
                        TestSourceI32::new(test_vec),
                        bv.clone(),
                        par.clone(),
                        RunModel::CPU
                    );
                    let mut step1: UnfinishedNodeBuilder<_, _, 1, 3> =
                        source.attach_standard("test_step1".to_string(), TestLinearI32Mult::new(), RunModel::CPU);
                    let step20: UnfinishedNodeBuilder<_, _, 1, 1> =
                        step1.attach_standard("test_step20".to_string(), TestLinearI32Mult::new(), RunModel::CPU);
                    let step21: UnfinishedNodeBuilder<_, _, 1, 1> =
                        step1.attach_standard("test_step21".to_string(), TestLinearI32Mult::new(), RunModel::CPU);
                    let step22: UnfinishedNodeBuilder<_, _, 1, 1> =
                        step1.attach_standard("test_step22".to_string(), TestLinearI32Mult::new(), RunModel::CPU);

                    let mut joint_node: UnfinishedNodeBuilder<i32, i32, 3, 2> =
                        UnfinishedNodeBuilder::<i32, i32, 3, 1>::create_joint_node("test_step3".to_string(), TestAdder::new(), bv.clone(), par, RunModel::CPU);
                    step20.feed_into(&mut joint_node);
                    step21.feed_into(&mut joint_node);
                    step22.feed_into(&mut joint_node);

                    joint_node
                        .add_pipeline_sink(
                            "test_sink1".to_string(),
                            TestSinkI32::new(out_send_1_clone),
                            RunModel::CPU,
                        )
                        .add_pipeline_sink(
                            "test_sink2".to_string(),
                            TestSinkI32::new(out_send_2_clone),
                            RunModel::CPU,
                        );
                }   
            ),
            PipelineParameters::new(64, 5, None),
            async_runtime,
            false
        );

        (pipeline.unwrap(), out_recv_1, out_recv_2)
    }

    #[test]
    fn test_split_pipeline() {
        let rt = Arc::new(tokio::runtime::Runtime::new().unwrap());
        let test_vec: BufferArray<i32, 8> =
            BufferArray::<i32, 8>::new_with_value([1, 2, 3, 4, 5, 6, 7, 8]);

        rt.clone().block_on(async {
            let (mut pipeline, mut receiver1, mut receiver2) = generate_test_pipeline(rt);
            pipeline.start();
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
            pipeline.stop();
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
