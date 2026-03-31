use std::time::Duration;
use log::{debug, warn};
use tokio::sync::mpsc::Sender;
use tokio::time::sleep;
use zubr_dsp::pipeline::communication_layer::data_management::{BufferArray, DataWrapper};
use zubr_dsp::pipeline::construction_layer::node_types::pipeline_step::PipelineStep;
use zubr_dsp::pipeline::construction_layer::pipeline_traits::{Sink, Source};

pub struct TestSourceI32<const BS: usize> {
    input_test_vec: BufferArray<i32, BS>,
    test_ctr: usize,
}

impl<const BS: usize> TestSourceI32<BS> {
    pub fn new(input_test_vec: BufferArray<i32, BS>) -> Self {
        Self {
            input_test_vec,
            test_ctr: 0,
        }
    }
}
#[async_trait::async_trait]
impl<const BS: usize> PipelineStep<(), i32, 0> for TestSourceI32<BS> {
    fn run_cpu(
        &mut self,
        _input: &mut [DataWrapper<()>; 0],
        output: &mut DataWrapper<i32>,
    ) -> Result<(), ()> {
        output.copy_from(self.input_test_vec.get(self.test_ctr));
        self.test_ctr = (self.test_ctr + 1) % self.input_test_vec.len();
        Ok(())
    }
    async fn run_io(
        &mut self,
        _input: &mut [DataWrapper<()>; 0],
        output: &mut DataWrapper<i32>,
    ) -> Result<(), ()> {
        output.copy_from(self.input_test_vec.get(self.test_ctr));
        sleep(Duration::from_millis(100)).await;
        self.test_ctr = (self.test_ctr + 1) % self.input_test_vec.len();
        Ok(())
    }
}

impl<const BS: usize> Source for TestSourceI32<BS> {}

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
    fn run_cpu(
        &mut self,
        input: &mut [DataWrapper<i32>; 1],
        _output: &mut DataWrapper<()>,
    ) -> Result<(), ()> {
        let mut output = 0;
        input[0].swap(&mut output);
        let _ = self.output_test_sender.try_send(output);
        Ok(())
    }
    async fn run_io(
        &mut self,
        input: &mut [DataWrapper<i32>; 1],
        _output: &mut DataWrapper<()>,
    ) -> Result<(), ()> {
        let mut output = 0;
        input[0].swap(&mut output);
        let _ = self.output_test_sender.send(output).await;
        Ok(())
    }
}

impl Sink for TestSinkI32 {}

pub struct TestSourceI32Vec<const BS: usize> {
    input_test_vec: BufferArray<i32, BS>,
}
impl<const BS: usize> TestSourceI32Vec<BS> {
    pub fn new(input_test_vec: BufferArray<i32, BS>) -> Self {
        Self { input_test_vec }
    }
}
impl<const BS: usize> PipelineStep<(), BufferArray<i32, BS>, 0> for TestSourceI32Vec<BS> {
    fn run_cpu(
        &mut self,
        _input: &mut [DataWrapper<()>; 0],
        output: &mut DataWrapper<BufferArray<i32, BS>>,
    ) -> Result<(), ()> {
        output.copy_from(&self.input_test_vec);
        debug!("INTERNAL LOGGING SOURCE: {:?}", output.read().read());
        Ok(())
    }
}
impl<const BS: usize> Source for TestSourceI32Vec<BS> {}

#[derive(Debug)]
pub struct TestSinkI32Vec<const BS: usize> {
    output_test_sender: Sender<BufferArray<i32, BS>>,
}
impl<const BS: usize> TestSinkI32Vec<BS> {
    pub fn new(sender: Sender<BufferArray<i32, BS>>) -> Self {
        Self {
            output_test_sender: sender,
        }
    }
}
impl<const BS: usize> PipelineStep<BufferArray<i32, BS>, (), 1> for TestSinkI32Vec<BS> {
    fn run_cpu(
        &mut self,
        input: &mut [DataWrapper<BufferArray<i32, BS>>; 1],
        _output: &mut DataWrapper<()>,
    ) -> Result<(), ()> {
        debug!("INTERNAL LOGGING SINK: {:?}", input[0].read().read());
        let mut send_value = BufferArray::new();
        input[0].swap(&mut send_value);
        let _ = self.output_test_sender.try_send(send_value);
        Ok(())
    }
}
impl<const BS: usize> Sink for TestSinkI32Vec<BS> {}

#[derive(Debug)]
pub struct TestLinearI32Mult {}
impl TestLinearI32Mult {
    pub fn new() -> Self {
        TestLinearI32Mult {}
    }
}

#[async_trait::async_trait]
impl PipelineStep<i32, i32, 1> for TestLinearI32Mult {
    fn run_cpu(
        &mut self,
        input: &mut [DataWrapper<i32>; 1],
        output: &mut DataWrapper<i32>,
    ) -> Result<(), ()> {
        *input[0].read() *= 2;
        output.swap_st(&mut input[0]);
        
        warn!("INTERNAL LOGGING MULT INPUT: {}", *input[0].read());
        warn!("INTERNAL LOGGING MULT OUTPUT: {}", *output.read());
        Ok(())
    }
    async fn run_io(
        &mut self,
        input: &mut [DataWrapper<i32>; 1],
        output: &mut DataWrapper<i32>,
    ) -> Result<(), ()> {
        *input[0].read() *= 2;
        output.swap_st(&mut input[0]);
        Ok(())
    }
}

#[derive(Debug)]
pub struct TestAdder {}
impl TestAdder {
    pub fn new() -> Self {
        TestAdder {}
    }
}

#[async_trait::async_trait]
impl<const N: usize> PipelineStep<i32, i32, N> for TestAdder {
    fn run_cpu(
        &mut self,
        input: &mut [DataWrapper<i32>; N],
        output: &mut DataWrapper<i32>,
    ) -> Result<(), ()> {
        let mut internal_value = 0;
        for i in input.iter_mut() {
            internal_value += *i.read();
        }
        output.swap(&mut internal_value);
        Ok(())
    }
}

pub fn verify_input_output<const BS: usize>(
    input_test_vec: BufferArray<i32, BS>,
    output_test_vec: BufferArray<i32, BS>,
    net_effect_function: fn(i32) -> i32,
) {
    let test_vec: Vec<i32> = input_test_vec
        .read()
        .iter()
        .map(|x| net_effect_function(*x))
        .collect();
    let real_output_vec: BufferArray<i32, BS> =
        BufferArray::new_with_value(test_vec.try_into().unwrap());
    assert_eq!(*real_output_vec.read(), *output_test_vec.read());
}
