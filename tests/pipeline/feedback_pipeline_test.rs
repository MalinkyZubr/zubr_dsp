#[cfg(test)]
mod pipeline_tests {
    use std::sync::mpsc;
    use crate::pipeline::api::ConstructingPipeline;
    use crate::pipeline::api::*;
    use crate::pipeline::orchestration_layer::logging::initialize_logger;
    use crate::pipeline::construction_layer::pipeline_node::joint_feedback_begin;
    use crate::pipeline::pipeline_traits::{Sink, Source};
    use crate::pipeline::interfaces::{ODFormat, PipelineStep};

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
    fn test_feedback_pipeline_assembly() {
        initialize_logger();
        // oftentimes it will be easier to contain a feedback loop directly inside of the pipeline step rather than constructing one within the 
        // framework of several steps as shown below. Feedback internal to a step offers finer grained control, and doesnt lose any performance
        // however if you really want supreme super separation of concerns because you're a good engineer feel free to use this for macro scale feedback loops
        // also if you dont want to keep manually reimplementing output feedback logic. Input feedback logic you should do yourself. But that isnt so hard. Is it?
        
        // can y[n-1] (on the pipeline scale). Mathematically this either means true y[n-1] in the case of a scalar type, or between
        // y[n - 1], y[n - k] for a k-sized vector type 
        let mut pipeline = ConstructingPipeline::new(3, 1000, 1, 2, 3, 1000);

        let input_pair = mpsc::sync_channel(1);
        let (output_sender, output_receiver) = mpsc::channel();
        
        let mut feedback_joint = joint_feedback_begin("Test Feedback Joint", &mut pipeline);

        NodeBuilder::start_pipeline("test_source", Dummy1 {receiver: input_pair.1}, &mut pipeline)
            .attach("step 1", Dummy2 {})
            .branch_end(&mut feedback_joint);
        
        let mut lazy_node = feedback_joint.joint_add_lazy();
        
        let mut test_split = feedback_joint.joint_lock(Dummy2 {})
            .split_begin("Test Split");
        
        test_split.split_add()
            .cap_pipeline("Exit", Dummy3 {sender: output_sender});
        
        lazy_node.joint_link_lazy("Feedback Node", Dummy2 {}, test_split.split_add());
        
        test_split.split_lock(Dummy2 {});

        let mut pipeline = pipeline.finish_pipeline();
        pipeline.start();

        input_pair.0.send(1);
        let result = output_receiver.recv().unwrap();
        dbg!(&result);
        assert_eq!(result, 4);
        
        input_pair.0.send(1);
        let result = output_receiver.recv().unwrap();
        dbg!(&result);
        assert_eq!(result, 9);
        
        pipeline.kill(); 
    }
}