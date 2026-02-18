use std::fmt::Display;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize};
use crate::pipeline::construction_layer::pipeline_traits::Sharable;
use std::sync::mpmc::RecvTimeoutError;
use std::sync::mpsc::{Receiver, SyncSender};
use tokio::sync::mpsc::{Sender as TokioSender, Receiver as TokioReceiver, error::SendError as TokioSendError};
use std::time::Duration;


#[derive(Debug)]
pub struct WrappedSender<T: Sharable> {
    dest_id: usize,
    is_stopped: Arc<AtomicBool>,
    sender: TokioSender<T>,
}
impl<T: Sharable> WrappedSender<T> {
    pub fn new(sender: TokioSender<T>, dest_id: usize) -> Self {
        WrappedSender {
            sender,
            is_stopped: Arc::new(AtomicBool::new(false)),
            dest_id,
        }
    }
    pub async fn send(&mut self, data: T) -> Result<(), TokioSendError<T>> {
        self.sender.send(data).await // this is all dummy code for now. We will need to change this later to handle errors properly
    }

    pub fn is_stopped(&self) -> bool {
        self.is_stopped.load(std::sync::atomic::Ordering::Acquire)
    }

    pub fn clone_stop_flag(&self) -> Arc<AtomicBool> {
        self.is_stopped.clone()
    }

    pub fn set_dest_id(&mut self, new_id: usize) {
        self.dest_id = new_id;
    }

    pub fn get_dest_id(&self) -> &usize {
        &self.dest_id
    }
}


#[derive(Debug)]
pub struct WrappedReceiver<T: Sharable> {
    source_id: usize,
    receiver: TokioReceiver<T>,
    num_receives: usize
}
impl<T: Sharable> WrappedReceiver<T> {
    pub fn new(receiver: TokioReceiver<T>, num_receives: usize, source_id: usize) -> Self {
        WrappedReceiver {
            source_id,
            receiver,
            num_receives
        }
    }

    pub fn recv(&mut self) -> Option<T> {
        self.receiver.blocking_recv()
    }

    pub fn set_num_receives(&mut self, num_receives: usize) {
        self.num_receives = num_receives;
    }

    pub fn set_source_id(&mut self, source_id: usize) {
        self.source_id = source_id;
    }

    pub fn get_source_id(&self) -> usize {
        self.source_id
    }
}


pub fn channel_wrapped<T: Sharable>(buffer_size: usize) -> (WrappedSender<T>, WrappedReceiver<T>) {
    if buffer_size == 0 {
        panic!("Buffer size must be greater than 0");
    }
    let (sender, receiver) = tokio::sync::mpsc::channel(buffer_size);
    (WrappedSender::new(sender, 0), WrappedReceiver::new(receiver, 1, 0))
}

