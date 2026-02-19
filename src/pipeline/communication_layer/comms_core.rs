use crate::pipeline::construction_layer::pipeline_traits::Sharable;
use std::fmt::Display;
use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::sync::Arc;
use tokio::select;
use tokio::sync::mpsc::{
    error::SendError as TokioSendError, Receiver as TokioReceiver, Sender as TokioSender,
};
use tokio::sync::Notify;

#[derive(Debug)]
pub struct WrappedSender<T: Sharable> {
    dest_id: usize,
    is_stopped: bool,
    backpressure_notify: Arc<Notify>,
    sender: TokioSender<T>,
    satiation_capacity: Arc<AtomicUsize>
}
impl<T: Sharable> WrappedSender<T> {
    pub fn new(sender: TokioSender<T>, dest_id: usize, backpressure_notify: Arc<Notify>, satiation_capacity: Arc<AtomicUsize>) -> Self {
        WrappedSender {
            sender,
            is_stopped: true,
            dest_id,
            satiation_capacity,
            backpressure_notify
        }
    }
    pub async fn send(&mut self, data: T) -> Result<(), TokioSendError<T>> {
        select! {
            output = self.sender.send(data) => { output },
            _ = self.backpressure_notify.notified() => {
                self.is_stopped = !self.is_stopped;
                Ok(())
            }
        }
    }

    pub fn is_stopped(&self) -> bool {
        self.is_stopped
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
    let mut satiated_edges: Vec<usize> = Vec::new();
    for sender_idx in 0..senders.len() - 1 {
        let sender = &mut senders[sender_idx];
        
        match sender.send(data.clone()).await {
            Ok(()) => if sender.channel_satiated() {satiated_edges.push(*sender.get_dest_id())},
            Err(err) => return Err(err)
        }
    }
    let last_sender = &mut senders[senders.len() - 1];
    match last_sender.send(data).await {
        Ok(()) => if last_sender.channel_satiated() {satiated_edges.push(*last_sender.get_dest_id())},
        Err(err) => return Err(err)
    }
    Ok(satiated_edges)
}

#[derive(Debug)]
pub struct WrappedReceiver<T: Sharable> {
    source_id: usize,
    is_stopped: bool,
    receiver: TokioReceiver<T>,
    backpressure_notify: Arc<Notify>,
    satiation_capacity: Arc<AtomicUsize>
}
impl<T: Sharable>
    WrappedReceiver<T>
{
    pub fn new(receiver: TokioReceiver<T>, source_id: usize, backpressure_notify: Arc<Notify>, satiation_capacity: Arc<AtomicUsize>) -> Self {
        WrappedReceiver {
            source_id,
            is_stopped: true,
            receiver,
            satiation_capacity,
            backpressure_notify
        }
    }

    pub fn recv(&mut self) -> Option<T> {
        self.receiver.blocking_recv()
    }

    pub async fn recv_async(&mut self) -> Option<T> {
        self.receiver.recv().await
    }

    pub fn is_stopped(&self) -> bool {
        self.is_stopped
    }

    pub fn set_state(&mut self, stopped: bool) {
        if stopped == self.is_stopped {return;}
        self.is_stopped = stopped;
        if stopped {self.backpressure_notify.notify_one();}
    }

    pub fn channel_satiated(&self) -> bool {
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
) {
    if buffer_size == 0 {
        panic!("Buffer size must be greater than 0");
    }
    let (sender, receiver) = tokio::sync::mpsc::channel(buffer_size);
    let backpressure_notify = Arc::new(Notify::new());
    let satiation_capacity: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(1));
    (
        WrappedSender::new(sender, dest_id, backpressure_notify.clone(), satiation_capacity.clone()),
        WrappedReceiver::new(receiver, source_id, backpressure_notify, satiation_capacity),
    )
}
