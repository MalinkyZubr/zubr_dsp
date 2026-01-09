use crate::pipeline::api::ODFormat;
use crate::pipeline::communication_layer::comms_core::{ChannelMetadata, WithChannelMetadata, WrappedReceiver};
use crate::pipeline::interfaces::ReceiveType;
use crate::pipeline::pipeline_traits::Sharable;
use std::sync::mpmc::{RecvTimeoutError, SendError};
use std::sync::mpsc::SyncSender;

pub struct MultichannelSender<T: Sharable> {
    senders: Vec<SyncSender<T>>,
}
impl<T: Sharable> MultichannelSender<T> {
    pub fn new() -> MultichannelSender<T> {
        MultichannelSender {
            senders: Vec::new(),
        }
    }
    fn decompose_send(&mut self, value: Vec<T>, result: &mut Result<(), SendError<T>>) {
        for (sender, point) in self.senders.iter_mut().zip(value) {
            *result = sender.send(point);
            match &result {
                Err(_) => break,
                _ => (),
            }
        }
    }
    fn standard_send(&mut self, value: T, result: &mut Result<(), SendError<T>>) {
        for index in 0..self.senders.len() {
            *result = self.senders[index].send(value.clone());
            match &result {
                Err(_) => break,
                _ => (),
            }
        }
    }
    fn series_send(&mut self, value: Vec<T>, result: &mut Result<(), SendError<T>>) {
        for point in value {
            for sender in self.senders.iter_mut() {
                *result = sender.send(point.clone());
                match &result {
                    Err(_) => break,
                    _ => (),
                }
            }
        }
    }
    fn repeat_send(&mut self, value: T, repeats: usize, result: &mut Result<(), SendError<T>>) {
        for _ in 0..repeats {
            for sender in self.senders.iter_mut() {
                *result = sender.send(value.clone());
                match &result {
                    Err(_) => break,
                    _ => (),
                }
            }
        }
    }
    pub fn send_all(&mut self, data: ODFormat<T>) -> Result<(), SendError<T>> {
        // all branches must be ready to receive
        let mut result = Ok(());

        match data {
            ODFormat::Standard(value) => self.standard_send(value, &mut result),
            ODFormat::Decompose(value) => self.decompose_send(value, &mut result),
            ODFormat::Series(value) => self.series_send(value, &mut result),
            ODFormat::Repeat(value, repeats) => self.repeat_send(value, repeats, &mut result),
        }

        result
    }
    pub fn add_sender(&mut self, sender: SyncSender<T>) {
        self.senders.push(sender);
    }
}

#[derive(Debug)]
pub struct MultichannelReceiver<T: Sharable> {
    receivers: Vec<WrappedReceiver<T>>,
    timeout: u64,
    retries: usize,
}

impl<T: Sharable> MultichannelReceiver<T> {
    pub fn new(timeout: u64, retries: usize) -> Self {
        MultichannelReceiver {
            receivers: Vec::new(),
            timeout,
            retries,
        }
    }
    fn receive_handler(
        result: Result<T, RecvTimeoutError>,
        proceed_flag: &mut bool,
        output: &mut Vec<T>,
        return_value: &mut Result<ReceiveType<T>, RecvTimeoutError>,
    ) {
        match result {
            Ok(received) => {
                if *proceed_flag {
                    output.push(received)
                };
            }
            Err(error) => {
                *proceed_flag = false;
                *return_value = Err(error);
            }
        }
    }
    pub fn receive(&mut self) -> Result<ReceiveType<T>, RecvTimeoutError> {
        let mut output = Vec::with_capacity(self.receivers.len());
        let mut proceed_flag = true;
        let mut return_value = Ok(ReceiveType::Multichannel(Vec::new()));

        for receiver in self.receivers.iter_mut() {
            let received = receiver.recv(self.timeout, self.retries);
            Self::receive_handler(received, &mut proceed_flag, &mut output, &mut return_value);
        }

        if output.len() == self.receivers.len() {
            return_value = Ok(ReceiveType::Multichannel(output));
        }

        return_value
    }
    pub fn add_receiver(&mut self, receiver: WrappedReceiver<T>) {
        self.receivers.push(receiver);
    }
}

impl<T: Sharable> WithChannelMetadata for MultichannelReceiver<T> {
    fn get_channel_metadata(&self) -> Vec<ChannelMetadata> {
        self.receivers.iter().map(|receiver| receiver.get_channel_metadata().clone()).collect()
    }
}

#[derive(Debug)]
pub struct Reassembler<T: Sharable> {
    receiver: WrappedReceiver<T>,
    num_receives: usize,
    timeout: u64,
    retries: usize,
}
impl<T: Sharable> Reassembler<T> {
    pub fn new(
        receiver: WrappedReceiver<T>,
        num_receives: usize,
        timeout: u64,
        retries: usize,
    ) -> Reassembler<T> {
        Self {
            receiver,
            num_receives,
            timeout,
            retries,
        }
    }

    pub fn receive(&mut self) -> Result<ReceiveType<T>, RecvTimeoutError> {
        let mut receive_vec = Vec::with_capacity(self.num_receives);

        for _ in 0..self.num_receives {
            match self.receiver.recv(self.timeout, self.retries) {
                Ok(received) => receive_vec.push(received),
                Err(error) => return Err(error),
            }
        }

        Ok(ReceiveType::Reassembled(receive_vec))
    }
}


impl<T: Sharable> WithChannelMetadata for Reassembler<T> {
    fn get_channel_metadata(&self) -> Vec<ChannelMetadata> {
        vec![self.receiver.get_channel_metadata().clone()]
    }
}