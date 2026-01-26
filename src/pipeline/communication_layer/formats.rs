use futures::SinkExt;
use strum::Display;
use crate::pipeline::api::Sharable;
use crate::pipeline::communication_layer::comms_core::WrappedSender;

#[derive(Debug, Clone, Display)]
pub enum ReceiveType<T: Sharable> {
    /*
    This is a very important enum!
    Single: Only a single value was received
    Reassembled: Multiple values were received and they were reassembled from a series sender somewhere previously
    Multichannel: This data is the aggregation of data received from multiple channels simultaneously
    Dummy: this channel is not configured

    The pipeline step must handle how the internal behavior responds to each of these types!
     */
    Single(T),
    Reassembled(Vec<T>),
    Multichannel(Vec<T>),
    Dummy
}
impl<T: Sharable> crate::pipeline::interfaces::ReceiveType<T> {
    pub fn to_string(&self) -> &str {
        match self {
            Self::Single(_) => "Single",
            Self::Reassembled(_) => "Reassembled",
            Self::Multichannel(_) => "Multichannel",
            Self::Dummy => "Dummy"
        }
    }
}


#[derive(Debug, Clone)]
pub enum ODFormat<T: Sharable> { // Output Data Format
    /*
    This is a very important enum!
    When you return data from a pipeline step, this defines how the pipeline treats that data. What do you want the pipeline to do with this data?
    Decompose:
        Single Out Behavior: Errors, decomposition only supported for multi out
        Multi Out Behavior: Gives a vector to the multi sender, each element of the vector is sent to a separate channel
            Example: 2 channel audio data is de-Decompose, separated into 2 vectors, and processed by different branches of the pipeline
    Series:
        Single Out Behavior: Iterates over the vector from start to finish sending elements to a single consumer in sequence
            Example: I am performing an overlap add convolution on some data, and need to break it into chunks. I chunk it into a vector of vectors,
            and return it so the sender sends each chunk separately in sequence
        Multiple Out Behavior: Replicates above behavior but round robin on multiple channels (eg, element 0 goes to channel 0, then channel 1, then element 1 goes to channel 0 etc)
    Standard:
        Single Out Behavior: Just sends the data as is once
        Multiple Out Behavior: Just sends the data as is once but to many channels

    In general: multiplexer holds exact same behavior as the single out
    */
    Decompose(Vec<T>),
    Series(Vec<T>),
    Standard(T)
}

impl<T: Sharable> ODFormat<T> {
    pub fn unwrap_standard(self) -> T {
        match self {
            ODFormat::Standard(x) => x,
            _ => panic!()
        }
    }

    pub async fn send(self, sender: &mut Vec<WrappedSender<T>>) -> Vec<usize> {
        match self {
            Self::Decompose(values) => Self::send_decompose(values, sender).await,
            Self::Series(values) => Self::send_series(values, sender).await,
            Self::Standard(value) => Self::send_standard(value, sender).await
        }
    }

    async fn send_standard(value: T, sender: &mut Vec<WrappedSender<T>>) -> Vec<usize> { // need error handling here
        let mut return_vec = Vec::with_capacity(sender.len());
        for sender_unit in sender.iter_mut().skip(1) {
            if !sender_unit.is_stopped() {
                sender_unit.send(value.clone()).await;
                return_vec.push(*sender_unit.get_dest_id());
            }
        }
        if !sender[0].is_stopped() {
            sender[0].send(value).await;
            return_vec.push(sender[0].get_dest_id());
        }
    }

    async fn send_decompose(values: Vec<T>, sender: &mut Vec<WrappedSender<T>>) -> Vec<usize> {
        if values.len() != sender.len() {
            panic!("Decompose and standard senders must have the same number of elements"); // better error handling
        }
        let mut return_vec = Vec::with_capacity(sender.len());
        for (value, sender_unit) in values.into_iter().zip(sender.iter_mut()) {
            if !sender_unit.is_stopped() {
                sender_unit.send(value).await;
                return_vec.push(*sender_unit.get_dest_id());
            }
        }

        return_vec
    }

    async fn send_series(values: Vec<T>, sender: &mut Vec<WrappedSender<T>>) -> Vec<usize> {
        let mut return_vec = Vec::with_capacity(sender.len());
        for value in values {
            for sender_unit in sender.iter_mut().skip(1) {
                if !sender_unit.is_stopped() {
                    sender_unit.send(value.clone()).await;
                    return_vec.push(*sender_unit.get_dest_id());
                }
            }
            if !sender[0].is_stopped() {
                sender[0].send(value).await;
                return_vec.push(sender[0].get_dest_id());
            }
        }
    }
}