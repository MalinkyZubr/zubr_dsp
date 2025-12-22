#[cfg(test)]
mod pipeline_tests {
    use std::sync::mpsc;
    use crate::pipeline::api::ConstructingPipeline;
    use crate::pipeline::api::*;
    use crate::pipeline::orchestration_layer::logging::initialize_logger;
    use crate::pipeline::interfaces::PipelineStep;
    use crate::pipeline::pipeline_traits::{Sink, Source};
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

    struct Dummy2a{}
    impl PipelineStep<u32, u32> for Dummy2a {
        fn run_SISO(&mut self, input: u32) -> Result<ODFormat<u32>, String> {
            Ok(ODFormat::Standard(input + 1))
        }

        fn run_REASO(&mut self, input: Vec<u32>) -> Result<ODFormat<u32>, String> {
            Ok(ODFormat::Standard(input.iter().sum()))
        }
    }

    struct Dummy2b{}
    impl PipelineStep<u32, u32> for Dummy2b {
        fn run_SISO(&mut self, input: u32) -> Result<ODFormat<u32>, String> {
            Ok(ODFormat::Series(vec![input + 1, input + 2, input + 3]))
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
    fn test_series_pipeline_assembly() {
        initialize_logger();
        
        let mut pipeline = ConstructingPipeline::new(3, 1000, 1, 2, 3, 1000);

        let input_pair = mpsc::sync_channel(1);
        let (output_sender, output_receiver) = mpsc::channel();
        
        log_message(format!("Starting pipeline construction"), Level::Debug);
        
        NodeBuilder::start_pipeline("Series test source", Dummy1 { receiver: input_pair.1 }, &pipeline)
            .attach("Series Expander", Dummy2b {})
            .attach("Series operator", Dummy2a {})
            .add_reassembler(3)
            .attach("Series aggregator", Dummy2a {})
            .cap_pipeline("Series test sink", Dummy3 {sender: output_sender});

        let mut pipeline = pipeline.finish_pipeline();
        log_message(format!("Finished pipeline construction"), Level::Debug);
        
        pipeline.start();
        log_message(format!("Pipeline started"), Level::Debug);

        input_pair.0.send(1);
        let result = output_receiver.recv().unwrap();
        dbg!(&result);
        assert_eq!(result, 15);

        input_pair.0.send(2);
        let result = output_receiver.recv().unwrap();
        dbg!(&result);
        assert_eq!(result, 18);

        pipeline.kill();
    }
}