use std::time::Duration;
use log::info;
use tokio::sync::mpsc::Sender;
use tokio::time::sleep;
use ZubrDSP::pipeline::construction_layer::node_types::pipeline_step::PipelineStep;
use ZubrDSP::pipeline::construction_layer::pipeline_traits::{Sink, Source};

#[derive(Debug)]
pub struct TestSourceI32 {
    input_test_vec: Vec<i32>,
    test_ctr: usize
}

impl TestSourceI32 {
    pub fn new(input_test_vec: Vec<i32>) -> Self {
        Self {
            input_test_vec,
            test_ctr: 0
        }
    }
}
#[async_trait::async_trait]
impl PipelineStep<(), i32, 0> for TestSourceI32 {
    fn run_cpu(&mut self, input: [(); 0]) -> Result<i32, ()> {
        let ret = Ok(self.input_test_vec[self.test_ctr]);
        self.test_ctr = (self.test_ctr + 1) % self.input_test_vec.len();
        ret
    }
    async fn run_io(&mut self, input: [(); 0]) -> Result<i32, ()> {
        let ret = Ok(self.input_test_vec[self.test_ctr]);
        sleep(Duration::from_millis(100)).await;
        self.test_ctr = (self.test_ctr + 1) % self.input_test_vec.len();
        ret
    }
}

impl Source for TestSourceI32 {}


#[derive(Debug)]
pub struct TestSinkI32 {
    output_test_vec: Vec<i32>,
    output_test_sender: Sender<Vec<i32>>,
    send_condition: usize
}
impl TestSinkI32 {
    pub fn new(sender: Sender<Vec<i32>>, send_condition: usize) -> Self {
        Self {
            output_test_vec: vec![],
            output_test_sender: sender,
            send_condition
        }
    }
}


#[async_trait::async_trait]
impl PipelineStep<i32, (), 1> for TestSinkI32 {
    fn run_cpu(&mut self, input: [i32; 1]) -> Result<(), ()> {
        self.output_test_vec.push(input[0]);
        info!("Logging {}", self.output_test_vec.len());
        if self.send_condition <= self.output_test_vec.len() {
            let _ = self.output_test_sender.try_send(self.output_test_vec.clone());
        }
        Ok(())
    }
    async fn run_io(&mut self, input: [i32; 1]) -> Result<(), ()> {
        self.output_test_vec.push(input[0]);
        sleep(Duration::from_millis(100)).await;
        if self.send_condition <= self.output_test_vec.len() {
            self.output_test_sender.send(self.output_test_vec.clone()).await.unwrap();
        }
        Ok(())
    }
}

impl Sink for TestSinkI32 {}


#[derive(Debug)]
pub struct TestLinearI32Mult {}
impl TestLinearI32Mult {
    pub fn new() -> Self { TestLinearI32Mult {} }
}

#[async_trait::async_trait]
impl PipelineStep<i32, i32, 1> for TestLinearI32Mult {
    fn run_cpu(&mut self, input: [i32; 1]) -> Result<i32, ()> {
        Ok(input[0] * 2)
    }
}


pub fn verify_input_output(input_test_vec: Vec<i32>, output_test_vec: Vec<i32>, net_effect_function: fn(i32) -> i32) {
    let real_output_vec: Vec<i32> = input_test_vec.iter().map(|x| net_effect_function(*x)).collect();
    assert_eq!(real_output_vec, output_test_vec);
}