#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::Arc;
    use log::{error, Level};
    use tokio::runtime::Runtime;
    use tokio::sync::mpsc::{channel, Receiver};
    use zubr_dsp::engine::build::build_pipeline;
    use zubr_dsp::engine::control_plane::pipeline_graph::PipelineGraph;
    use zubr_dsp::engine::control_plane::pipeline_hl::Pipeline;
    use zubr_dsp::engine::control_plane::scheduler_models::topographical::{ThreadPoolTopographical, ThreadPoolTopographicalHandle};
    use zubr_dsp::engine::data_plane::communication_layer::data_management::BufferArray;
    use zubr_dsp::engine::data_plane::construction::node_build_vector::PipelineBuildVector;
    use zubr_dsp::engine::data_plane::construction::unfinished_node_builder::UnfinishedNodeBuilder;
    use zubr_dsp::engine::data_plane::structural::generic_pipeline_node::RunModel::{CPU, IO};
    use zubr_dsp::engine::zubr_dsp_config::PipelineParameters;
    use zubr_dsp::initiate_pipeline;
    use crate::infrastructure::test_models::{TestSinkI32Vec, TestSourceI32Vec};

    fn generate_test_pipeline(async_runtime: Arc<Runtime>) -> (
        Pipeline<ThreadPoolTopographicalHandle>,
        Receiver<BufferArray<i32, 4>>,
        Receiver<BufferArray<i32, 4>>,
    ) {
        initiate_pipeline(Level::Debug);

        let (out_send_1, out_recv_1) = channel(100);
        let (out_send_2, out_recv_2) = channel(100);
        let pipeline = build_pipeline::<ThreadPoolTopographicalHandle>(
            Box::new(|bv, par| {
                let test_vec: BufferArray<i32, 8> =
                    BufferArray::<i32, 8>::new_with_value([1, 2, 3, 4, 5, 6, 7, 8]);
                let mut source: UnfinishedNodeBuilder<_, _, 0, 1> = UnfinishedNodeBuilder::<(), i32, 0, 1>::add_pipeline_source("test_source".to_string(), TestSourceI32Vec::new(test_vec), bv.clone(), par, CPU);

                let interleaved_separator: UnfinishedNodeBuilder<BufferArray<i32, 8>, BufferArray<i32, 4>, 1, 2> =
                    source
                        .attach_interleaved_separator::<2, 4>("separator 1".to_string())
                        .add_pipeline_sink(
                            "channel 1 sink".to_string(),
                            TestSinkI32Vec::new(out_send_1),
                            CPU
                        )
                        .add_pipeline_sink(
                            "channel 2 sink".to_string(),
                            TestSinkI32Vec::new(out_send_2),
                            CPU
                        );
            }
            ),
            PipelineParameters::standard_no_analytics(),
            async_runtime).unwrap();


        (pipeline, out_recv_1, out_recv_2)
    }

    #[test]
    fn test_interleaving_pipeline() {
        let rt = Arc::new(tokio::runtime::Runtime::new().unwrap());

        rt.block_on(async {
            let (mut pipeline, mut receiver1, mut receiver2) = generate_test_pipeline(rt.clone());
            pipeline.start();
            error!("test_linear_pipeline start");

            let vv1: [i32; 4] = *receiver1.recv().await.unwrap().read();
            let vv11: [i32; 4] = [1, 3, 5, 7];

            let vv2: [i32; 4] = *receiver2.recv().await.unwrap().read();
            let vv22: [i32; 4] = [2, 4, 6, 8];
            assert_eq!(vv1, vv11);
            assert_eq!(vv2, vv22);
        });
    }
}
