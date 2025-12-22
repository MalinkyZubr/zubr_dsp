#[cfg(test)]
mod pipeline_tests {
    use std::sync::atomic::AtomicUsize;
    use std::sync::mpsc;
    use std::thread::sleep;
    use std::sync::Arc;
    use crate::pipeline::api::*;
    use crate::pipeline::orchestration_layer::logging::initialize_logger;
    use super::*;


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
    }

    struct Dummy2b{}
    impl PipelineStep<u32, u32> for Dummy2b {
        fn run_SISO(&mut self, input: u32) -> Result<ODFormat<u32>, String> {
            Ok(ODFormat::Standard(input + 2))
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
    fn test_multiplexed_pipeline_assembly() {
        initialize_logger();

        log_message("Staring linear pipeline construction".to_string(), Level::Debug);

        let mut pipeline = ConstructingPipeline::new(3, 1000, 1, 2, 3, 1000);
        let input_pair = mpsc::sync_channel(1);
        let (output_sender, output_receiver) = mpsc::channel();
        // 
        log_message("input output mpsc communicators created".to_string(), Level::Debug);
        
        let multiplexer_selector = Arc::new(AtomicUsize::new(0));

        let mut multiplexer_builder = NodeBuilder::start_pipeline("Pipeline Origin", Dummy1 { receiver: input_pair.1 }, &pipeline)
            .mutltiplexer_begin("Test Multiplexer", multiplexer_selector.clone());

        let mut demultiplexer = demultiplexer_begin("Demultiplexer", multiplexer_selector.clone(), &pipeline);
        
        multiplexer_builder.multiplexer_add()
            .attach("branch 1 operator", Dummy2a {})
            .multiplex_branch_end(&mut demultiplexer);

        multiplexer_builder.multiplexer_add()
            .attach("branch 2 operator", Dummy2b {})
            .multiplex_branch_end(&mut demultiplexer);
        
        multiplexer_builder.multiplexer_lock(Dummy2a {});
        
        demultiplexer.demultiplexer_lock(Dummy2a {})
            .cap_pipeline("Multiplexed sink", Dummy3 { sender: output_sender});
        
        
        log_message("Pipeline Path Designed".to_string(), Level::Debug);

        let mut pipeline = pipeline.finish_pipeline();

        log_message("Pipeline finished construction".to_string(), Level::Debug);

        pipeline.start();

        log_message("Pipeline started execution".to_string(), Level::Debug);

        //sleep(std::time::Duration::from_millis(1000));

        input_pair.0.send(1).unwrap();
        let result = output_receiver.recv().unwrap();
        log_message(format!("First message yielded: {}", &result), Level::Debug);
        assert_eq!(result, 5);
        
        multiplexer_selector.store(1, std::sync::atomic::Ordering::SeqCst);

        input_pair.0.send(2).unwrap();
        let result = output_receiver.recv().unwrap();
        log_message(format!("Second message yielded: {}", &result), Level::Debug);
        assert_eq!(result, 7);

        pipeline.kill();

        log_message("Pipeline execution ended".to_string(), Level::Debug);
    }
}
    