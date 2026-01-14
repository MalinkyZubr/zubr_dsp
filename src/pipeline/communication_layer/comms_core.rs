use std::fmt::Display;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use crate::pipeline::pipeline_traits::Sharable;
use std::sync::mpmc::RecvTimeoutError;
use std::sync::mpsc::{Receiver, SyncSender};
use std::time::Duration;


#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ChannelState {
    ERROR=2,
    STOPPED=1,
    OKAY=0
}
impl Display for ChannelState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChannelState::ERROR => write!(f, "ERROR"),
            ChannelState::STOPPED => write!(f, "STOPPED"),
            ChannelState::OKAY => write!(f, "OKAY"),
        }
    }
}


#[derive(Debug, Clone)]
pub struct ChannelMetadata {
    pub origin_id: String,
    pub critical_channel: bool
}
impl ChannelMetadata {
    pub fn new(origin_id: String, critical_channel: bool) -> Self {
        ChannelMetadata { origin_id, critical_channel }
    }
}


pub struct DataPacket<T: Sharable> {
    data: T,
    epoch: usize, // may want to solve ovf problem later with tens of thousands of samples per second. But for now wont worry
}


pub struct WrappedSender<T: Sharable> {
    sender: SyncSender<T>,
    channel_state: Arc<AtomicUsize>
}


#[derive(Debug)]
pub struct WrappedReceiver<T: Sharable> {
    receiver: Receiver<T>,
    channel_metadata: ChannelMetadata,
    channel_state: Arc<AtomicUsize>
}
impl<T: Sharable> WrappedReceiver<T> {
    pub fn new(receiver: Receiver<T>, channel_metadata: ChannelMetadata) -> Self {
        WrappedReceiver {
            receiver,
            channel_metadata
        }
    }
    fn result_handler(
        &self,
        result: &mut Result<T, RecvTimeoutError>,
        retry_num: &mut usize,
        retries: usize,
    ) -> bool {
        match result {
            Err(err) => {
                match err {
                    RecvTimeoutError::Timeout => *retry_num += 1,
                    _ => *retry_num = retries,
                };
                false
            }
            Ok(_) => true,
        }
    }
    pub fn recv(&mut self, timeout: u64, retries: usize) -> Result<T, RecvTimeoutError> {
        let mut retry_num = 0;

        let mut result = Err(RecvTimeoutError::Timeout);
        let mut success_flag = false;

        while retry_num < retries && !success_flag {
            result = self.receiver.recv_timeout(Duration::from_millis(timeout));

            success_flag = self.result_handler(&mut result, &mut retry_num, retries);
        }
        result
    }
    
    pub fn get_channel_metadata(&self) -> &ChannelMetadata {
        &self.channel_metadata
    }
}


pub trait WithChannelMetadata {
    fn get_channel_metadata(&self) -> Vec<ChannelMetadata>;
}


pub fn channel_wrapped<T: Sharable>(buffer_size: usize, channel_metadata: ChannelMetadata) -> (SyncSender<T>, WrappedReceiver<T>) {
    if buffer_size == 0 {
        panic!("Buffer size must be greater than 0");
    }
    let (sender, receiver) = std::sync::mpsc::sync_channel(buffer_size);
    (sender, WrappedReceiver::new(receiver, channel_metadata))
}