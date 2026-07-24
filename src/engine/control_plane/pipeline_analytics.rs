use scc::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc::{channel, Receiver, Sender};
use crate::engine::control_plane::node_wrapper::NodeWrapper;
use crate::engine::data_plane::structural::generic_pipeline_node::RunModel;


#[derive(Debug, Clone)]
pub struct NodeAnalytics {
    pub id: usize,
    pub name: String,
    pub run_model: RunModel,
    pub num_executions: u128,
    pub last_execution_time_ns: u128,
    pub last_execution_instant_sec: u64,
    pub current_state: u8,
}
pub struct PipelineAnalyticsSink {
    analytic_receiver: Option<Receiver<NodeAnalytics>>,
    analytic_sender: Sender<NodeAnalytics>,
    analytic_storage: Arc<HashMap<usize, NodeAnalytics>>,
}
impl PipelineAnalyticsSink {
    pub fn new(channel_size: usize) -> PipelineAnalyticsSink {
        let (sender, receiver) = channel(channel_size);
        PipelineAnalyticsSink {
            analytic_receiver: Some(receiver),
            analytic_sender: sender,
            analytic_storage: Arc::new(HashMap::new()),
        }
    }
    pub fn check_task_started(&self) -> bool {
        self.analytic_receiver.is_none()
    }

    pub fn generate_source(&self, id: usize, name: String, run_model: RunModel) -> PipelineAnalyticsSource {
        PipelineAnalyticsSource::new(self.analytic_sender.clone(), id, name, run_model)
    }

    pub async fn get_analytics_task(
        mut analytic_receiver: Receiver<NodeAnalytics>,
        analytic_storage: Arc<HashMap<usize, NodeAnalytics>>,
    ) -> () {
        let analytic = analytic_receiver.recv().await;
        match analytic {
            Some(analytic) => {
                let _ = analytic_storage.insert_async(analytic.id, analytic).await;
            }
            None => (),
        }
    }
    
    pub async fn get_analytics_direct(&mut self) {
        let analytic = self.analytic_receiver.as_mut().unwrap().recv().await;
        match analytic {
            Some(analytic) => {
                let _ = self.analytic_storage.insert_async(analytic.id, analytic).await;
            }
            None => (),
        }
    }

    pub fn get_analytics(&self, id: usize) -> Option<NodeAnalytics> {
        match self.analytic_storage.get_sync(&id) {
            Some(value) => Some(value.clone()),
            None => None,
        }
    }
}


pub struct PipelineAnalyticsSource {
    analytic_sender: Sender<NodeAnalytics>,
    id: usize,
    name: String,
    run_model: RunModel,
    num_executions: u128,
    last_execution_time_ns: u128,
    last_execution_instant_sec: u64,
    current_state: u8,
    start: Instant,
}
impl PipelineAnalyticsSource {
    pub fn new(sender: Sender<NodeAnalytics>, id: usize, name: String, run_model: RunModel) -> PipelineAnalyticsSource {
        PipelineAnalyticsSource {
            analytic_sender: sender,
            id,
            name,
            run_model,
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
            name: self.name.clone(),
            run_model: self.run_model.clone(),
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
