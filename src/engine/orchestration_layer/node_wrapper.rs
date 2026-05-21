use std::future::Future;
use std::collections::HashMap;
use crate::engine::orchestration_layer::pipeline_analytics::PipelineAnalyticsSource;
use crate::engine::structural::generic_pipeline_node::{GenericNode, NodeState};
use tokio::sync::watch::{Receiver as WatchReceiver, Sender as WatchSender, channel as watch_channel};
use tokio::sync::broadcast::{Receiver as BroadcastReceiver, Sender as BroadcastSender, channel as broadcast_channel};


#[derive(Clone)]
pub struct NodeStateAuthPacket {
    node_id: usize,
    stop: bool
}


pub struct InternodeStopAuthorityFactory {
    sender: BroadcastSender<NodeStateAuthPacket>
}
impl InternodeStopAuthorityFactory {
    pub fn new(channel_size: usize) -> Self {
        let (sender, _) = broadcast_channel(channel_size);
        Self {
            sender
        }
    }

    pub fn new_internode_authority(&self, owner_id: usize, node: Box<dyn GenericNode>) -> InternodeStopAuthority {
        let mut predecessor_stopped_map = HashMap::new();
        let mut successor_stopped_map = HashMap::new();
        for predecessor in node.get_predecessors() {
            predecessor_stopped_map.insert(predecessor, false);
        }
        for successor in node.get_successors() {
            successor_stopped_map.insert(successor, false); // need to manually start
        }
        InternodeStopAuthority {
            receiver: self.sender.subscribe(),
            sender: self.sender.clone(),
            predecessor_stopped_map,
            successor_stopped_map,
            owner_id
        }
    }
}


pub struct InternodeStopAuthority {
    receiver: BroadcastReceiver<NodeStateAuthPacket>,
    sender: BroadcastSender<NodeStateAuthPacket>,
    predecessor_stopped_map: HashMap<usize, bool>, // false is running, true is stopped
    successor_stopped_map: HashMap<usize, bool>,
    owner_id: usize
}
impl InternodeStopAuthority {
    fn determine_adjacency_state(&self) -> bool {
        if self.predecessor_stopped_map.values().any(|&x| x) || self.successor_stopped_map.values().all(|&x| !x) {
            false
        }
        else {
            true
        }
    }
    
    pub fn update_states(&mut self) -> bool{
        let mut update = false;
        let mut stop = false;
        while let Ok(packet) = self.receiver.try_recv() {
            if self.predecessor_stopped_map.contains_key(&packet.node_id) {
                update = true;
                self.predecessor_stopped_map.insert(packet.node_id, packet.stop);
            }
            else if self.successor_stopped_map.contains_key(&packet.node_id) {
                update = true;
                self.successor_stopped_map.insert(packet.node_id, packet.stop);
            }
            else if self.owner_id == packet.node_id {
                stop = true;
                update = true;
            }
        }
        
        if update && !stop {
            stop = self.determine_adjacency_state();
            let _ = self.sender.send(
                NodeStateAuthPacket {
                    node_id: self.owner_id,
                    stop
                }
            );
        }
        
        stop
    }
}


pub trait NodeStateManager {
    fn get_state(&self) -> NodeState;
    fn update_state(&mut self, node: &Box<dyn GenericNode>);
}

pub struct SourceNodeStateManager {
    state: NodeState,
    watch_receiver: WatchReceiver<bool>,
    internode_authority: InternodeStopAuthority
}
pub struct SinkNodeStateManager {
    state: NodeState,
    watch_receiver: WatchReceiver<bool>,
    internode_authority: InternodeStopAuthority
}
impl SinkNodeStateManager {
    pub fn new(watch_receiver: WatchReceiver<bool>, internode_authority: InternodeStopAuthority) -> Self {
        Self {
            state: NodeState::ExecIo,
            watch_receiver,
            internode_authority
        }
    }
}
pub struct GeneralNodeStateManager { // depends only on surrounding nodes
    state: NodeState,
    internode_authority: InternodeStopAuthority
}
impl GeneralNodeStateManager {
    pub fn new(internode_authority: InternodeStopAuthority) -> Self {
        Self {
            state: , // initial state here
            internode_authority
        }
    }
}
impl NodeStateManager for GeneralNodeStateManager {
    fn get_state(&self) -> NodeState {
        self.state
    }
    fn update_state(&mut self, node: &Box<dyn GenericNode>) {
        let stopped = self.internode_authority.update_states();
        if stopped {
            self.state = NodeState::Stop;
        }
        else {
            self.state = node.next_state(self.state);
        }
    }
}


pub enum RunType {
    CPU(Box<dyn FnOnce() -> NodeWrapper>),
    IO(Box<dyn Future<Output=NodeWrapper>>),
    COMM(Box<dyn Future<Output=(NodeWrapper, Option<usize>)>>)
}


pub struct NodeWrapper {
    id: usize,
    name: String,
    node: Box<dyn GenericNode>,
    analytic_source: Option<PipelineAnalyticsSource>,
    state_manager: Box<dyn NodeStateManager>
}
impl NodeWrapper {
    pub fn new(id: usize, name: String, node: Box<dyn GenericNode>) -> Self {
        let state_manager = node_state_manager_factory(&node);
        Self {
            id,
            name,
            node,
            analytic_source: None,
            state_manager
        }
    }

    fn wrapped_run_exec_cpu(mut wrapper: Self) -> Box<dyn FnOnce() -> NodeWrapper> {
        Box::new(move || {
            wrapper.state_manager.update_state(&wrapper.node);
            match wrapper.analytic_source.as_mut() {
                Some(mut analytic_source) => {
                    analytic_source.enter_execution();
                    wrapper.node.call_thread_cpu();
                    analytic_source.exit_execution();
                }
                None => wrapper.node.call_thread_cpu()
            }
            wrapper
        })
    }

    fn wrapped_run_exec_io(mut wrapper: Self) -> Box<dyn Future<Output=NodeWrapper>> {
        Box::new(async move {
            wrapper.state_manager.update_state(&wrapper.node);
            match wrapper.analytic_source.as_mut() {
                Some(mut analytic_source) => {
                    analytic_source.enter_execution();
                    wrapper.node.call_thread_io().await;
                    analytic_source.exit_execution();
                }
                None => wrapper.node.call_thread_io().await
            }
            wrapper
        })
    }

    fn wrapped_run_exec_comm(mut wrapper: Self) -> Box<dyn Future<Output=(NodeWrapper, Option<usize>)>> {
        Box::new(async move {
            wrapper.state_manager.update_state(&wrapper.node);
            let mut successors;
            match wrapper.analytic_source.as_mut() {
                Some(mut analytic_source) => {
                    analytic_source.enter_execution();
                    successors = wrapper.node.run_senders().await;
                    analytic_source.exit_execution();
                }
                None => successors = wrapper.node.run_senders().await
            }
            (wrapper, successors)
        })
    }

    fn wrapped_run_comm(mut wrapper: Self) -> Box<dyn Future<Output=(NodeWrapper, Option<usize>)>> {
        Box::new(async move {
            let successors = wrapper.node.run_senders().await;
            (wrapper, successors)
        })
    }

    pub fn generate_run(mut self) -> Result<RunType, Self> {
        match self.state_manager.get_state() {
            NodeState::ExecCpu => Ok(RunType::CPU(Self::wrapped_run_exec_cpu(self))),
            NodeState::ExecIo => Ok(RunType::IO(Self::wrapped_run_exec_io(self))),
            NodeState::ExecCommunicate => Ok(RunType::COMM(Self::wrapped_run_exec_comm(self))),
            NodeState::Communicate => Ok(RunType::COMM(Self::wrapped_run_comm(self))),
            NodeState::Stop => Err(self)
        }
    }

    pub fn get_state(&self) -> NodeState {
        self.state_manager.get_state()
    }

    pub fn get_id(&self) -> usize {
        self.id
    }

    pub fn attach_analytics_source(&mut self, source: PipelineAnalyticsSource) {
        self.analytic_source = Some(source);
    }

    async fn run_senders(&mut self) {
        self.node.run_senders().await;
    }

    pub fn get_successors(&self) -> Vec<usize> {
        self.node.get_successors()
    }

    pub fn get_predecessors(&self) -> Vec<usize> {
        self.node.get_predecessors()
    }

    pub fn is_ready_exec(&self) -> bool {
        self.node.is_ready_exec()
    }

    pub fn is_source(&self) -> bool {
        self.node.get_num_inputs() == 0
    }

    pub fn is_sink(&self) -> bool {
        self.node.get_num_outputs() == 0
    }
}