use async_trait::async_trait;
use std::fmt::Debug;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunModel {
    IO,
    CPU,
    Communicator,
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeState {
    ExecCpu,
    ExecIo,
    ExecCommunicate,
    Communicate,
    Stop,
}

#[async_trait]
pub trait GenericNode: Send + Sync + 'static {
    async fn run_senders(&mut self) -> Option<usize>; // this return value contains all the successors ready to run
    fn check_nth_satiated_edge_id(&self, edge_index: usize) -> Option<usize>;
    fn load_initial_state(&mut self);
    fn has_initial_state(&self) -> bool;
    fn get_num_inputs(&self) -> usize;
    fn get_num_outputs(&self) -> usize;
    fn is_ready_exec(&self) -> bool;
    fn get_successors(&self) -> Vec<usize>;
    fn get_predecessors(&self) -> Vec<usize>;
    fn get_run_model(&self) -> RunModel;
    fn call_thread_cpu(&mut self) {
        panic!("CPU thread is not implemented for this node type");
    }
    async fn call_thread_io(&mut self) {
        panic!("IO thread is not implemented for this node type");
    }
    fn next_state(&self, current_state: NodeState) -> NodeState;
}