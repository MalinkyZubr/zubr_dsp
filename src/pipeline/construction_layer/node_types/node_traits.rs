use async_trait::async_trait;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Instant;
use crate::pipeline::construction_layer::node_types::pipeline_node::PipelineNode;
use crate::pipeline::construction_layer::pipeline_traits::Sharable;

#[async_trait]
pub trait CollectibleNode {
    async fn run_senders(&mut self, id: usize, increment_size: &mut usize) -> Vec<usize>; // this return value contains all the successors ready to run
    fn clone_output_stop_flag(&self, id: usize) -> Option<Arc<AtomicBool>>;
    fn load_initial_state(&mut self);
    fn has_initial_state(&self) -> bool;
}

pub trait CPUCollectibleNode: Send + CollectibleNode {
    fn call_thread_cpu(&mut self, id: usize);
}

#[async_trait]
pub trait IOCollectibleNode: Send + CollectibleNode {
    async fn call_thread_io(&mut self, id: usize);
}