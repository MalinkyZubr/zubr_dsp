use async_trait::async_trait;
use std::fmt::Debug;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

#[async_trait]
pub trait CollectibleNode {
    async fn run_senders(&mut self, id: usize) -> Vec<usize>; // this return value contains all the successors ready to run
    fn clone_output_stop_flag(&self, id: usize) -> Option<Arc<AtomicBool>>;
    fn load_initial_state(&mut self);
    fn has_initial_state(&self) -> bool;
    fn get_num_inputs(&self) -> usize;
    fn get_num_outputs(&self) -> usize;
    fn call_thread_cpu(&mut self, id: usize) {
        panic!("CPU thread is not implemented for this node type");
    }
    async fn call_thread_io(&mut self, id: usize) {
        panic!("IO thread is not implemented for this node type");
    }
}
