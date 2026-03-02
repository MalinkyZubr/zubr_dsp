use std::time::Duration;
use log::{error, info};
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
    output_test_sender: Sender<i32>,
}
impl TestSinkI32 {
    pub fn new(sender: Sender<i32>) -> Self {
        Self {
            output_test_sender: sender,
        }
    }
}


#[async_trait::async_trait]
impl PipelineStep<i32, (), 1> for TestSinkI32 {
    fn run_cpu(&mut self, input: [i32; 1]) -> Result<(), ()> {
        let _ = self.output_test_sender.try_send(input[0]);
        Ok(())
    }
    async fn run_io(&mut self, input: [i32; 1]) -> Result<(), ()> {
        let _ = self.output_test_sender.send(input[0]).await;
        Ok(())
    }
}

impl Sink for TestSinkI32 {}


#[derive(Debug)]
pub struct TestSourceI32Vec {
    input_test_vec: Vec<i32>,
}
impl TestSourceI32Vec{
    pub fn new(input_test_vec: Vec<i32>) -> Self {
        Self {
            input_test_vec,
        }
    }
}
impl PipelineStep<(), Vec<i32>, 0> for TestSourceI32Vec {
    fn run_cpu(&mut self, input: [(); 0]) -> Result<Vec<i32>, ()> {
        Ok(self.input_test_vec.clone())
    }
}
impl Source for TestSourceI32Vec {}


#[derive(Debug)]
pub struct TestSinkI32Vec {
    output_test_sender: Sender<Vec<i32>>,
}
impl TestSinkI32Vec {
    pub fn new(sender: Sender<Vec<i32>>) -> Self {
        Self {
            output_test_sender: sender,
        }
    }
}
impl PipelineStep<Vec<i32>, (), 1> for TestSinkI32Vec {
    fn run_cpu(&mut self, input: [Vec<i32>; 1]) -> Result<(), ()> {
        let _ = self.output_test_sender.try_send(input[0].clone());
        Ok(())
    }
}
impl Sink for TestSinkI32Vec {}


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
    async fn run_io(&mut self, input: [i32; 1]) -> Result<i32, ()> {
        Ok(input[0] * 2)
    }
}


#[derive(Debug)]
pub struct TestAdder {}
impl TestAdder {
    pub fn new() -> Self { TestAdder {} }
}

#[async_trait::async_trait]
impl<const N: usize> PipelineStep<i32, i32, N> for TestAdder {
    fn run_cpu(&mut self, input: [i32; N]) -> Result<i32, ()> {
        Ok(input.iter().sum())
    }
}


pub fn verify_input_output(input_test_vec: Vec<i32>, output_test_vec: Vec<i32>, net_effect_function: fn(i32) -> i32) {
    let real_output_vec: Vec<i32> = input_test_vec.iter().map(|x| net_effect_function(*x)).collect();
    assert_eq!(real_output_vec, output_test_vec);
}