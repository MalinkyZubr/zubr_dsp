use crate::pipeline::api::ODFormat;
use crate::pipeline::communication_layer::comms_core::{ChannelMetadata, WithChannelMetadata, WrappedReceiver};
use crate::pipeline::interfaces::ReceiveType;
use crate::pipeline::pipeline_traits::Sharable;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpmc::{RecvTimeoutError, SendError};
use std::sync::mpsc::SyncSender;
use std::sync::Arc;

#[derive(Debug)]
pub struct Multiplexer<T: Sharable> {
    senders: Vec<SyncSender<T>>,
    channel: Arc<AtomicUsize>, // external control for the channel selection
}
impl<T: Sharable> Multiplexer<T> {
    pub fn new(channel: Arc<AtomicUsize>) -> Multiplexer<T> {
        Multiplexer {
            senders: Vec::new(),
            channel,
        }
    }
    pub fn send(&mut self, input: ODFormat<T>) -> Result<(), SendError<T>> {
        match self.senders.get_mut(self.channel.load(Ordering::Acquire)) {
            Some(sender) => Self::send_logic(sender, input),
            None => Err(SendError(self.index_error_unwrap(input))),
        }
    }
    fn repeat_send(
        selected_sender: &mut SyncSender<T>,
        value: T,
        repeats: usize,
        result: &mut Result<(), SendError<T>>,
    ) {
        for _ in 0..repeats {
            *result = selected_sender.send(value.clone());
            match &result {
                Err(_) => break,
                _ => (),
            }
        }
    }
    fn series_send(
        selected_sender: &mut SyncSender<T>,
        value: Vec<T>,
        result: &mut Result<(), SendError<T>>,
    ) {
        for unit in value {
            *result = selected_sender.send(unit);
            match &result {
                Err(_) => break,
                _ => (),
            }
        }
    }
    fn send_logic(
        selected_sender: &mut SyncSender<T>,
        input: ODFormat<T>,
    ) -> Result<(), SendError<T>> {
        let mut result = Ok(());

        match input {
            ODFormat::Standard(value) => result = selected_sender.send(value),
            ODFormat::Repeat(value, repeats) => {
                Self::repeat_send(selected_sender, value, repeats, &mut result)
            }
            ODFormat::Decompose(_) => panic!("ODFormat Decompose not compatible with multiplexer"),
            ODFormat::Series(value) => Self::series_send(selected_sender, value, &mut result),
        }

        result
    }
    fn index_error_unwrap(&self, input: ODFormat<T>) -> T {
        match input {
            ODFormat::Standard(result) | ODFormat::Repeat(result, _) => result,
            ODFormat::Series(mut result) | ODFormat::Decompose(mut result) => result.pop().unwrap(),
        }
    }

    pub fn add_sender(&mut self, sender: SyncSender<T>) {
        self.senders.push(sender);
    }
}

#[derive(Debug)]
pub struct Demultiplexer<T: Sharable> {
    receivers: Vec<WrappedReceiver<T>>,
    channel: Arc<AtomicUsize>, // external control for the channel selection
    timeout: u64,
    retries: usize,
}
impl<T: Sharable> Demultiplexer<T> {
    pub fn new(channel: Arc<AtomicUsize>, timeout: u64, retries: usize) -> Demultiplexer<T> {
        Demultiplexer {
            receivers: Vec::new(),
            channel,
            timeout,
            retries,
        }
    }
    pub fn receive(&mut self) -> Result<ReceiveType<T>, RecvTimeoutError> {
        match self.receivers.get_mut(self.channel.load(Ordering::Acquire)) {
            Some(receiver) => match receiver.recv(self.timeout, self.retries) {
                Ok(received) => Ok(ReceiveType::Single(received)),
                Err(err) => Err(err),
            },
            None => Err(RecvTimeoutError::Disconnected),
        }
    }
    pub fn add_receiver(&mut self, receiver: WrappedReceiver<T>) {
        self.receivers.push(receiver);
    }
}

impl<T: Sharable> WithChannelMetadata for Demultiplexer<T> {
    fn get_channel_metadata(&self) -> Vec<ChannelMetadata> {
        self.receivers.iter().map(|receiver| receiver.get_channel_metadata().clone()).collect()
    }
}
