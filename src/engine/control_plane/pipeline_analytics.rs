use std::collections::{HashMap, VecDeque};
use scc::Queue;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use async_trait::async_trait;
use log::error;
use num::integer::Roots;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use crate::engine::control_plane::background_proc_manager::BackgroundTaskTrait;
use crate::engine::control_plane::node_wrapper::NodeWrapper;
use crate::engine::data_plane::structural::generic_pipeline_node::RunModel;
use crate::engine::zubr_dsp_config::PipelineAnalyticsParameters;

#[derive(Debug, Clone)]
pub struct NodeAnalytics { // would be better to also have executions per second
    pub id: usize,
    pub name: String,
    pub run_model: RunModel,
    pub average_execution_time: u128,
    pub standard_deviation_execution_time: u128,
    pub current_state: u8,
}

#[derive(Debug)]
pub struct PipelineAnalyticsSink {
    analytic_receiver: Receiver<NodeAnalytics>,
    analytic_sender: Sender<NodeAnalytics>,
    analytic_proxy_queue: Arc<Queue<NodeAnalytics>>,
    analytic_proxy_queue_size: usize,
    analytics_interval: usize
}
impl PipelineAnalyticsSink {
    pub fn new(pipeline_analytics_parameters: PipelineAnalyticsParameters) -> PipelineAnalyticsSink {
        let (sender, receiver) = channel(pipeline_analytics_parameters.analytics_sink_buffer_size);
        let mut analytics_queue_size = pipeline_analytics_parameters.analytics_sink_buffer_size;

        if analytics_queue_size < 8 {
            analytics_queue_size = 8
        }

        PipelineAnalyticsSink {
            analytic_receiver: receiver,
            analytic_sender: sender,
            analytic_proxy_queue: Arc::new(Queue::new()),
            analytic_proxy_queue_size: analytics_queue_size,
            analytics_interval: pipeline_analytics_parameters.analytics_interval
        }
    }

    pub fn generate_source(&self, id: usize, name: String, run_model: RunModel) -> PipelineAnalyticsSource {
        PipelineAnalyticsSource::new(self.analytic_sender.clone(), id, name, run_model, self.analytics_interval)
    }

    pub fn get_proxy_queue(&self) -> Arc<Queue<NodeAnalytics>> {
        self.analytic_proxy_queue.clone()
    }
}


#[async_trait]
impl BackgroundTaskTrait for PipelineAnalyticsSink {
    async fn task_run(&mut self) {
        let analytic = self.analytic_receiver.recv().await;
        match analytic {
            Some(analytic) => {
                if self.analytic_proxy_queue.len() > self.analytic_proxy_queue_size {
                    self.analytic_proxy_queue.pop(); // enforce the queue size limitation
                }
                let _ = self.analytic_proxy_queue.push(analytic);
            }
            None => (),
        }
    }
}


struct RollingStatTracker {
    buffer: VecDeque<u128>,
    buffer_size: u128,
    current_average: u128,
    current_standard_deviation: u128,
}
impl RollingStatTracker {
    pub fn new(buffer_size: usize) -> RollingStatTracker {
        Self {
            buffer: VecDeque::from(vec![0; buffer_size]),
            buffer_size: buffer_size as u128,
            current_average: 0,
            current_standard_deviation: 0
        }
    }

    pub fn push_sample(&mut self, value: u128) {
        // https://nestedsoftware.com/2018/03/27/calculating-standard-deviation-on-streaming-data-253l.23919.html
        let oldest_value = self.buffer.pop_front().unwrap(); // we can guarantee there will always be a value inside
        self.buffer.push_back(value);

        let old_average = self.current_average;

        self.current_average = old_average + value.abs_diff(oldest_value) / self.buffer_size;
        self.current_standard_deviation = (self.current_standard_deviation + value.abs_diff(oldest_value) *
            (value + oldest_value.abs_diff(self.current_average.abs_diff(old_average))) /
            (self.buffer_size - 1)).sqrt();
    }

    pub fn get_stats(&self) -> (u128, u128) {
        (self.current_average, self.current_standard_deviation)
    }
}

pub struct PipelineAnalyticsSource { // this needs to be redone so it only sends once every hundred or thousand executions. Mpsc calls every iteration will be SUPEr expensive
    analytic_sender: Sender<NodeAnalytics>,
    id: usize,
    name: String,
    run_model: RunModel,
    rolling_stat_tracker: RollingStatTracker,
    current_state: u8,
    start: Instant,
    analytics_interval: usize,
    analytics_index: usize,
}
impl PipelineAnalyticsSource {
    pub fn new(sender: Sender<NodeAnalytics>, id: usize, name: String, run_model: RunModel, analytics_interval: usize) -> PipelineAnalyticsSource {
        PipelineAnalyticsSource {
            analytic_sender: sender,
            id,
            name,
            run_model,
            rolling_stat_tracker: RollingStatTracker::new(analytics_interval),
            current_state: 0,
            start: Instant::now(),
            analytics_interval,
            analytics_index: 0,
        }
    }

    pub fn enter_execution(&mut self) {
        self.start = Instant::now();
    }

    pub fn exit_execution(&mut self) {
        let dur = self.start.elapsed();

        self.rolling_stat_tracker.push_sample(dur.as_nanos() as u128);
        self.analytics_index += 1;

        if self.analytics_index >= self.analytics_interval {
            let _ = self.analytic_sender.try_send(self.to_analytics());
            self.analytics_index = 0;
        }
    }

    pub fn to_analytics(&self) -> NodeAnalytics {
        let (average_exec_time, standard_deviation_exec_time) = self.rolling_stat_tracker.get_stats();
        NodeAnalytics {
            id: self.id,
            name: self.name.clone(),
            run_model: self.run_model.clone(),
            average_execution_time: average_exec_time as u128,
            standard_deviation_execution_time: standard_deviation_exec_time as u128,
            current_state: self.current_state,
        }
    }

    pub fn set_state(&mut self, state: u8) {
        self.current_state = state;
    }
}
