#[cfg(test)]
mod pipeline_tests {
    use std::sync::mpsc;
    use crate::pipeline::api::ConstructingPipeline;
    use crate::pipeline::api::*;
    use crate::pipeline::orchestration_layer::logging::initialize_logger;
    use crate::pipeline::interfaces::ODFormat;

    struct Dummy1 {
        receiver: mpsc::Receiver<u32>
    }
    impl PipelineStep<(), u32> for Dummy1 {
        fn run_DISO(&mut self) -> Result<ODFormat<u32>, String> {
            match self.receiver.recv_timeout(std::time::Duration::from_millis(2000)) {
                Ok(val) => Ok(ODFormat::Standard(val + 1)),
                Err(_) => Err("Timeout error".to_string())
            }
        }
    }
    impl Source for Dummy1 {}

    struct Dummy2{}
    impl PipelineStep<u32, u32> for Dummy2 {
        fn run_SISO(&mut self, input: u32) -> Result<ODFormat<u32>, String> {
            Ok(ODFormat::Standard(input + 1))
        }
        
        fn run_MISO(&mut self, input: Vec<u32>) -> Result<ODFormat<u32>, String> {
            Ok(ODFormat::Standard(input.iter().sum()))
        }
        
        fn run_SIMO(&mut self, input: u32) -> Result<ODFormat<u32>, String> {
            Ok(ODFormat::Standard(input + 1))
        }
    }

    struct Dummy3{
        sender: mpsc::Sender<u32>,
    }
    impl PipelineStep<u32, ()> for Dummy3 {
        fn run_SIDO(&mut self, input: u32) -> Result<ODFormat<()>, String> {
            self.sender.send(input).unwrap();
            Ok(ODFormat::Standard(()))
        }
    }
    impl Sink for Dummy3 {}

    #[test]
    fn test_branch_pipeline_assembly() {
        initialize_logger();
        
        let mut pipeline = ConstructingPipeline::new(3, 1000, 1, 2, 3, 1000);

        let input_pair = mpsc::sync_channel(1);
        let (output_sender, output_receiver) = mpsc::channel();
        
        let mut split = NodeBuilder::start_pipeline("test_source", Dummy1 {receiver: input_pair.1}, &mut pipeline)
            .attach("step 1", Dummy2 {}).split_begin("test_split");

        let mut test_joint = joint_begin("Test Joint", &mut pipeline);
        
        split.split_add()
            .attach("branch_1 node 1", Dummy2 {})
            .attach("branch_1 node 2", Dummy2 {})
            .branch_end(&mut test_joint);
        
        split.split_add()
            .attach("branch_2 node 1", Dummy2 {})
            .branch_end(&mut test_joint);
        
        split.split_lock(Dummy2 {});
        
        test_joint.joint_lock(Dummy2 {})
            .attach("Post Joint Test Node", Dummy2 {})
            .cap_pipeline("Test Cap", Dummy3 { sender: output_sender.clone() });
        
        let mut pipeline = pipeline.finish_pipeline();
        
        pipeline.start();

        input_pair.0.send(1).unwrap();
        let result = output_receiver.recv().unwrap();
        dbg!(&result);
        assert_eq!(result, 12);

        pipeline.kill();
    }
}