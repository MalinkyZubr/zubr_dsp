use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicU8, AtomicUsize};
use std::time;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use crate::engine::orchestration_layer::pipeline_graph::PipelineNodeState;
use crossbeam_queue::ArrayQueue;
use tokio::sync::Notify;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::task::JoinSet;
use crate::engine::structural::generic_pipeline_node::GenericNode;

pub struct PipelineAnalyticsSink {
    analytic_receiver: Receiver<NodeAnalytics>,
    analytic_sender: Sender<NodeAnalytics>,
}
impl PipelineAnalyticsSink {
    pub fn new(channel_size: usize) -> PipelineAnalyticsSink {
        let (sender, receiver) = channel(channel_size);
        PipelineAnalyticsSink {
            analytic_receiver: receiver,
            analytic_sender: sender,
        }
    }
    
    pub fn generate_source(&self) -> Sender<NodeAnalytics> {
        self.analytic_sender.clone()
    }
    
    pub async fn get_analytics(&mut self) -> NodeAnalytics {
        self.analytic_receiver.recv().await.unwrap()
    }
}


#[derive(Debug, Clone)]
pub struct NodeAnalytics {
    id: usize,
    num_executions: u128,
    last_execution_time_ns: u128,
    last_execution_instant_sec: u64,
    current_state: u8,
}
pub struct PipelineAnalyticsSource {
    analytic_sender: Sender<NodeAnalytics>,
    id: usize,
    num_executions: u128,
    last_execution_time_ns: u128,
    last_execution_instant_sec: u64,
    current_state: u8,
    start: Instant
}
impl PipelineAnalyticsSource {
    pub fn new(sender: Sender<NodeAnalytics>, id: usize) -> PipelineAnalyticsSource {
        PipelineAnalyticsSource {
            analytic_sender: sender,
            id,
            num_executions: 0,
            last_execution_time_ns: 0,
            last_execution_instant_sec: 0,
            current_state: 0,
            start: Instant::now(),
        }   
    }

    pub fn enter_execution(&mut self) {
        self.start = Instant::now();
    }

    pub fn exit_execution(&mut self) {
        let dur = self.start.elapsed();

        self.last_execution_time_ns = dur.as_nanos();
        self.num_executions += 1;
        self.last_execution_instant_sec = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let _ = self.analytic_sender.try_send(self.to_analytics());
    }
    
    pub fn to_analytics(&self) -> NodeAnalytics {
        NodeAnalytics {
            id: self.id,
            num_executions: self.num_executions,
            last_execution_time_ns: self.last_execution_time_ns,
            last_execution_instant_sec: self.last_execution_instant_sec,
            current_state: self.current_state,
        }
    }

    pub fn set_state(&mut self, state: u8) {
        self.current_state = state;
    }
}