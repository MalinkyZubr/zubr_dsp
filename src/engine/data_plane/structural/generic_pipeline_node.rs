use async_trait::async_trait;
use std::fmt::Debug;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunModel {
    IO,
    CPU,
    Communicator,
}
impl RunModel {
    pub fn to_state(&self) -> NodeState {
        match self {
            RunModel::IO => NodeState::ExecIo,
            RunModel::CPU => NodeState::ExecCpu,
            RunModel::Communicator => NodeState::ExecCommunicate,
        }
    }
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
    fn get_satiated_edges(&self, num_satiated: usize) -> &[usize];
    fn load_initial_value(&mut self);
    fn has_initial_value(&self) -> bool;
    fn get_num_inputs(&self) -> usize;
    fn get_num_outputs(&self) -> usize;
    fn is_ready_exec(&self, state: NodeState) -> bool;
    fn get_successors(&self) -> Vec<usize>;
    fn get_predecessors(&self) -> Vec<usize>;
    fn get_run_model(&self) -> RunModel;
    fn call_thread_cpu(&mut self) -> Result<(), ()> {
        panic!("CPU thread is not implemented for this node type");
    }
    async fn call_thread_io(&mut self) -> Result<(), ()> {
        panic!("IO thread is not implemented for this node type");
    }
    fn next_state(&self, current_state: NodeState) -> NodeState; // all next states MUST have a transition away from stop state
    fn initial_state(&self) -> NodeState;
}