use crate::pipeline::construction_layer::pipeline_traits::Sharable;
use std::fmt::Display;
use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::sync::Arc;
use tokio::sync::mpsc::{
    error::SendError as TokioSendError, Receiver as TokioReceiver, Sender as TokioSender,
};

#[derive(Debug)]
pub struct WrappedSender<T: Sharable> {
    dest_id: usize,
    is_stopped: Arc<AtomicBool>,
    sender: TokioSender<T>,
    satiation_capacity: Arc<AtomicUsize>
}
impl<T: Sharable> WrappedSender<T> {
    pub fn new(sender: TokioSender<T>, dest_id: usize, is_stopped: Arc<AtomicBool>, satiation_capacity: Arc<AtomicUsize>) -> Self {
        WrappedSender {
            sender,
            is_stopped,
            dest_id,
            satiation_capacity
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

    pub fn channel_satiated(&self) -> bool {
        let num_elements = self.sender.max_capacity() - self.sender.capacity();
        num_elements >= self.satiation_capacity.load(std::sync::atomic::Ordering::Acquire)
    }
    
    pub fn set_satiation_capacity(&mut self, capacity: usize) {
        self.satiation_capacity.store(capacity, std::sync::atomic::Ordering::Release);
    }

    pub fn get_dest_id(&self) -> &usize {
        &self.dest_id
    }
}

pub async fn iterative_send<T: Sharable, const N: usize>(senders: &mut [WrappedSender<T>; N], data: T) -> Result<Vec<usize>, TokioSendError<T>> {
    let mut satiated_edges = Vec::new();
    for sender_idx in 0..senders.len() - 1 {
        let sender = &mut senders[sender_idx];
        
        match sender.send(data.clone()).await {
            Ok(()) => if sender.channel_satiated() {satiated_edges.push(sender.get_dest_id())},
            Err(err) => return Err(err)
        }
    }
    let last_sender = &mut senders[senders.len() - 1];
    match last_sender.send(data).await {
        Ok(()) => if last_sender.channel_satiated() {satiated_edges.push(last_sender.get_dest_id())},
        Err(err) => return Err(err)
    }
    Ok(satiated_edges)
}

#[derive(Debug)]
pub struct WrappedReceiver<T: Sharable> {
    source_id: usize,
    is_stopped: Arc<AtomicBool>,
    receiver: TokioReceiver<T>,
    satiation_capacity: Arc<AtomicUsize>
}
impl<T: Sharable>
    WrappedReceiver<T>
{
    pub fn new(receiver: TokioReceiver<T>, source_id: usize, is_stopped: Arc<AtomicBool>, satiation_capacity: Arc<AtomicUsize>) -> Self {
        WrappedReceiver {
            source_id,
            is_stopped,
            receiver,
            satiation_capacity
        }
    }

    pub fn recv(&mut self) -> Option<T> {
        self.receiver.blocking_recv()
    }

    pub fn channel_statiated(&self) -> bool {
        self.receiver.len() >= self.satiation_capacity.load(std::sync::atomic::Ordering::Acquire)
    }
    
    pub fn set_satiation_capacity(&mut self, capacity: usize) {
        self.satiation_capacity.store(capacity, std::sync::atomic::Ordering::Release);
    }
}

pub fn channel_wrapped<T: Sharable>(
    buffer_size: usize,
    source_id: usize,
    dest_id: usize,
) -> (
    WrappedSender<T>,
    WrappedReceiver<T>,
    Arc<AtomicBool>,
) {
    if buffer_size == 0 {
        panic!("Buffer size must be greater than 0");
    }
    let (sender, receiver) = tokio::sync::mpsc::channel(buffer_size);
    let is_stopped = Arc::new(AtomicBool::new(false));
    let satiation_capacity: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(1));
    (
        WrappedSender::new(sender, dest_id, is_stopped.clone(), satiation_capacity.clone()),
        WrappedReceiver::new(receiver, source_id, is_stopped.clone(), satiation_capacity),
        is_stopped,
    )
}
