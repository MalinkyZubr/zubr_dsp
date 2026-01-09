use crate::pipeline::api::ODFormat;
use crate::pipeline::communication_layer::comms_core::{ChannelMetadata, WithChannelMetadata, WrappedReceiver};
use crate::pipeline::interfaces::ReceiveType;
use crate::pipeline::pipeline_traits::Sharable;
use std::sync::mpmc::{RecvTimeoutError, SendError};
use std::sync::mpsc::SyncSender;

#[derive(Debug)]
pub struct SingleSender<T: Sharable> {
    sender: SyncSender<T>,
}
impl<T: Sharable> SingleSender<T> {
    pub fn new(sender: SyncSender<T>) -> Self {
        SingleSender { sender }
    }
    fn repeat_send(&mut self, value: T, repeats: usize, result: &mut Result<(), SendError<T>>) {
        for _ in 0..repeats {
            *result = self.sender.send(value.clone());
            match &result {
                Err(_) => break,
                _ => (),
            }
        }
    }
    fn series_send(&mut self, value: Vec<T>, result: &mut Result<(), SendError<T>>) {
        for point in value {
            *result = self.sender.send(point);
            match &result {
                Err(_) => break,
                _ => (),
            }
        }
    }
    pub fn send(&mut self, value: ODFormat<T>) -> Result<(), SendError<T>> {
        let mut result = Ok(());

        match value {
            ODFormat::Decompose(mut data_vec) => result = Err(SendError(data_vec.pop().unwrap())),
            ODFormat::Standard(value) => result = self.sender.send(value),
            ODFormat::Repeat(value, repeats) => self.repeat_send(value, repeats, &mut result),
            ODFormat::Series(value) => self.series_send(value, &mut result),
        }

        result
    }
}

#[derive(Debug)]
pub struct SingleReceiver<T: Sharable> {
    receiver: WrappedReceiver<T>,
    timeout: u64,
    retries: usize,
}
impl<T: Sharable> SingleReceiver<T> {
    pub fn new(receiver: WrappedReceiver<T>, timeout: u64, retries: usize) -> SingleReceiver<T> {
        SingleReceiver {
            receiver,
            timeout,
            retries,
        }
    }
    pub fn receive(&mut self) -> Result<ReceiveType<T>, RecvTimeoutError> {
        match self.receiver.recv(self.timeout, self.retries) {
            Ok(result) => Ok(ReceiveType::Single(result)),
            Err(err) => Err(err),
        }
    }
    pub fn extract_receiver(self) -> WrappedReceiver<T> {
        self.receiver
    }
}


impl<T: Sharable> WithChannelMetadata for SingleReceiver<T> {
    fn get_channel_metadata(&self) -> Vec<ChannelMetadata> {
        vec![self.receiver.get_channel_metadata().clone()]
    }
}
