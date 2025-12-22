use crate::pipeline::api::*;
use crate::pipeline::communication_layer::comms_top_level::{NodeReceiver, NodeSender};
use crate::pipeline::interfaces::{ODFormat, PipelineStep, ReceiveType};
use crate::pipeline::pipeline_traits::{HasID, Sharable};
use crossbeam_queue::ArrayQueue;
use std::fmt::Debug;
use std::sync::mpsc::RecvTimeoutError;
use std::sync::Arc;

#[derive(Debug, PartialEq, Clone)]
pub enum PipelineStepResult {
    Success,
    SendError,
    RecvTimeoutError(RecvTimeoutError),
    ComputeError(String),
}

#[derive(Debug, Copy, Clone)]
struct DummyStep {}
impl<T: Sharable> PipelineStep<T, T> for DummyStep {
    fn run_SISO(&mut self, input: T) -> Result<ODFormat<T>, String> {
        Ok(ODFormat::Standard(input))
    }
}

pub struct PipelineNode<I: Sharable, O: Sharable> {
    pub input: NodeReceiver<I>,
    pub output: NodeSender<O>,
    pub id: String,
    tap: Option<Arc<ArrayQueue<ODFormat<O>>>>,
}

impl<I: Sharable, O: Sharable> HasID for PipelineNode<I, O> {
    fn get_id(&self) -> String {
        self.id.clone()
    }

    fn set_id(&mut self, id: &str) {
        self.id = id.to_string();
    }
}

impl<I: Sharable, O: Sharable> PipelineNode<I, O> {
    pub fn new() -> PipelineNode<I, O> {
        PipelineNode {
            input: NodeReceiver::Dummy,
            output: NodeSender::Dummy,
            id: "".to_string(),
            tap: None,
        }
    }

    pub fn call(&mut self, step: &mut Box<dyn PipelineStep<I, O>>) -> PipelineStepResult {
        let received_result = self.input.receive();
        match received_result {
            Err(err) => {
                let result = PipelineStepResult::RecvTimeoutError(err);
                result
            } // must have a way to handle if it is a dummy
            Ok(val) => {
                let result = self.route_computation(val, step);
                result
            }
        }
    }

    fn compute_handler(&mut self, output_data: Result<ODFormat<O>, String>) -> PipelineStepResult {
        match output_data {
            Err(err) => PipelineStepResult::ComputeError(err),
            Ok(extracted_data) => {
                let extracted_data = self.push_to_tap(extracted_data);
                match self.output.send(extracted_data) {
                    Err(_) => PipelineStepResult::SendError,
                    Ok(_) => PipelineStepResult::Success,
                }
            }
        }
    }
    fn push_to_tap(&mut self, output_data: ODFormat<O>) -> ODFormat<O> {
        match &mut self.tap {
            None => output_data,
            Some(tap) => {
                let cloned_output = output_data.clone();
                tap.push(output_data);
                cloned_output
            }
        }
    }

    fn route_computation(
        &mut self,
        input_data: ReceiveType<I>,
        step: &mut Box<dyn PipelineStep<I, O>>,
    ) -> PipelineStepResult {
        //log_message(format!("CRITICAL: NodeID: {}, Received message: {}", &self.id, &input_data), Level::Debug);
        self.compute_handler(match (input_data, &self.output) {
            (ReceiveType::Single(t), NodeSender::SO(_) | NodeSender::MUO(_)) => step.run_SISO(t),
            (ReceiveType::Single(t), NodeSender::MO(_)) => step.run_SIMO(t),
            (ReceiveType::Multichannel(t), NodeSender::SO(_) | NodeSender::MUO(_)) => {
                step.run_MISO(t)
            }
            (ReceiveType::Multichannel(t), NodeSender::MO(_)) => step.run_MIMO(t),
            (ReceiveType::Reassembled(t), NodeSender::SO(_) | NodeSender::MUO(_)) => {
                step.run_REASO(t)
            }
            (ReceiveType::Reassembled(t), NodeSender::MO(_)) => step.run_REAMO(t),
            (ReceiveType::Dummy, NodeSender::SO(_)) => step.run_DISO(),
            (ReceiveType::Single(t), NodeSender::Dummy) => step.run_SIDO(t),
            (_, _) => Err(String::from("Received bad message from pipeline step")),
        })
    }
}
