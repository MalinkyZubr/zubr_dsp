use crate::pipeline::communication_layer::comms_implementations::linear_comms::{
    SingleReceiver, SingleSender,
};
use crate::pipeline::communication_layer::comms_implementations::multichannel_comms::{
    MultichannelReceiver, MultichannelSender, Reassembler,
};
use crate::pipeline::communication_layer::comms_implementations::multiplexed_comms::{
    Demultiplexer, Multiplexer,
};
use crate::pipeline::interfaces::{ODFormat, ReceiveType};
use crate::pipeline::pipeline_traits::Sharable;
use std::sync::mpsc::{RecvTimeoutError, SendError};

#[derive(Debug)]
pub enum NodeReceiver<I: Sharable> {
    SI(SingleReceiver<I>),
    MI(MultichannelReceiver<I>),
    REA(Reassembler<I>),
    DMI(Demultiplexer<I>),
    Dummy,
}
impl<I: Sharable> NodeReceiver<I> {
    pub fn receive(&mut self) -> Result<ReceiveType<I>, RecvTimeoutError> {
        match self {
            NodeReceiver::SI(receiver) => receiver.receive(),
            NodeReceiver::MI(receiver) => receiver.receive(),
            NodeReceiver::REA(receiver) => receiver.receive(),
            NodeReceiver::DMI(receiver) => receiver.receive(),
            NodeReceiver::Dummy => Ok(ReceiveType::Dummy),
        }
    }
}

pub enum NodeSender<O: Sharable> {
    SO(SingleSender<O>),
    MO(MultichannelSender<O>),
    MUO(Multiplexer<O>),
    Dummy,
}
impl<O: Sharable> NodeSender<O> {
    pub fn send(&mut self, data: ODFormat<O>) -> Result<(), SendError<O>> {
        match self {
            NodeSender::SO(sender) => sender.send(data),
            NodeSender::MO(sender) => sender.send_all(data),
            NodeSender::MUO(sender) => sender.send(data),
            NodeSender::Dummy => Ok(()),
        }
    }
}
