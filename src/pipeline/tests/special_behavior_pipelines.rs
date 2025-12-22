#[cfg(test)]
mod pipeline_tests {
    use std::sync::mpsc;
    use std::thread::sleep;
    use futures::AsyncReadExt;
    use crate::pipeline::api::*;
    use crate::pipeline::orchestration_layer::logging::initialize_logger;
    use super::*;


    struct Dummy1a {
        receiver: mpsc::Receiver<Vec<u32>>
    }
    impl PipelineStep<(), Vec<u32>> for Dummy1a {
        fn run_DISO(&mut self) -> Result<ODFormat<Vec<u32>>, String> {
            match self.receiver.recv_timeout(std::time::Duration::from_millis(2000)) {
                Ok(val) => Ok(ODFormat::Standard(val)),
                Err(_) => Err("Timeout error".to_string())
            }
        }
    }
    impl Source for Dummy1a {}

    struct Dummy1b {
        receiver: mpsc::Receiver<u32>
    }
    impl PipelineStep<(), u32> for Dummy1b {
        fn run_DISO(&mut self) -> Result<ODFormat<u32>, String> {
            match self.receiver.recv_timeout(std::time::Duration::from_millis(2000)) {
                Ok(val) => Ok(ODFormat::Standard(val + 1)),
                Err(_) => Err("Timeout error".to_string())
            }
        }
    }
    impl Source for Dummy1b {}

    struct Dummy2a{}
    impl PipelineStep<Vec<u32>, u32> for Dummy2a {
        fn run_SIMO(&mut self, input: Vec<u32>) -> Result<ODFormat<u32>, String> {
            Ok(ODFormat::Decompose(input))
        }
    }

    struct Dummy2b{}
    impl PipelineStep<u32, u32> for Dummy2b {
        fn run_SISO(&mut self, input: u32) -> Result<ODFormat<u32>, String> {
            Ok(ODFormat::Repeat(input + 1, 3))
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
    fn test_interleaved_pipeline_assembly() {
        initialize_logger();

        log_message("Staring linear pipeline construction".to_string(), Level::Debug);

        let mut pipeline = ConstructingPipeline::new(3, 1000, 1, 2, 3, 1000);
        let input_pair = mpsc::sync_channel(1);
        let (output_sender_1, output_receiver_1) = mpsc::channel();
        let (output_sender_2, output_receiver_2) = mpsc::channel();
        // 
        log_message("input output mpsc communicators created".to_string(), Level::Debug);

        let mut split = NodeBuilder::start_pipeline("interleaved source", Dummy1a {receiver: input_pair.1}, &pipeline)
            .split_begin("de-interleaving split");
        
        split.split_add()
            .cap_pipeline("De-interleave sink 1", Dummy3 { sender: output_sender_1 });
        
        split.split_add()
            .cap_pipeline("De-interleave sink 2", Dummy3 { sender: output_sender_2 });
        
        split.split_lock(Dummy2a {});

        log_message("Pipeline Path Designed".to_string(), Level::Debug);

        let mut pipeline = pipeline.finish_pipeline();

        log_message("Pipeline finished construction".to_string(), Level::Debug);

        pipeline.start();

        log_message("Pipeline started execution".to_string(), Level::Debug);

        //sleep(std::time::Duration::from_millis(1000));

        input_pair.0.send(vec![1,2]).unwrap();
        let result1 = output_receiver_1.recv().unwrap();
        let result2 = output_receiver_2.recv().unwrap();
        log_message(format!("First message yielded: {}, {}", &result1, &result2), Level::Debug);
        assert_eq!(result1, 1);
        assert_eq!(result2, 2);

        input_pair.0.send(vec![3,4]).unwrap();
        let result1 = output_receiver_1.recv().unwrap();
        let result2 = output_receiver_2.recv().unwrap();
        log_message(format!("First message yielded: {}, {}", &result1, &result2), Level::Debug);
        assert_eq!(result1, 3);
        assert_eq!(result2, 4);

        pipeline.kill();

        log_message("Pipeline execution ended".to_string(), Level::Debug);
    }
    
    #[test]
    fn test_repeat_pipeline_assembly() {
        initialize_logger();

        log_message("Staring linear pipeline construction".to_string(), Level::Debug);

        let mut pipeline = ConstructingPipeline::new(3, 1000, 1, 2, 3, 1000);
        let input_pair = mpsc::sync_channel(1);
        let (output_sender, output_receiver) = mpsc::channel();
        // 
        log_message("input output mpsc communicators created".to_string(), Level::Debug);

        NodeBuilder::start_pipeline("test_source", Dummy1b { receiver: input_pair.1 }, &pipeline)
            .attach("step 1", Dummy2b {})
            .cap_pipeline("test_sink", Dummy3 { sender: output_sender });

        log_message("Pipeline Path Designed".to_string(), Level::Debug);

        let mut pipeline = pipeline.finish_pipeline();

        log_message("Pipeline finished construction".to_string(), Level::Debug);

        pipeline.start();

        log_message("Pipeline started execution".to_string(), Level::Debug);

        //sleep(std::time::Duration::from_millis(1000));

        input_pair.0.send(1).unwrap();
        let result = output_receiver.recv().unwrap();
        log_message(format!("First message yielded: {}", &result), Level::Debug);
        assert_eq!(result, 3);

        let result = output_receiver.recv().unwrap();
        log_message(format!("Second message yielded: {}", &result), Level::Debug);
        assert_eq!(result, 3);

        let result = output_receiver.recv().unwrap();
        log_message(format!("Third message yielded: {}", &result), Level::Debug);
        assert_eq!(result, 3);
        
        pipeline.kill();

        log_message("Pipeline execution ended".to_string(), Level::Debug);
    }
}