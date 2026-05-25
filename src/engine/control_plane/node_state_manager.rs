use crate::engine::data_plane::structural::generic_pipeline_node::{GenericNode, NodeState};
use std::collections::HashMap;
use tokio::sync::broadcast::{
    channel as broadcast_channel, Receiver as BroadcastReceiver, Sender as BroadcastSender,
};
use tokio::sync::watch;
use tokio::sync::watch::{Receiver as WatchReceiver, Sender as WatchSender, channel as WatchChannel, error};
use crate::engine::data_plane::construction::node_build_vector::PreparedNode;

#[derive(Clone)]
pub struct NodeStateAuthPacket {
    node_id: usize,
    stop: bool,
}

pub struct InternodeStopAuthorityFactory {
    sender: BroadcastSender<NodeStateAuthPacket>,
}
impl InternodeStopAuthorityFactory {
    pub fn new(channel_size: usize) -> Self {
        let (sender, _) = broadcast_channel(channel_size);
        Self { sender }
    }

    pub fn new_internode_stop_authority(
        &self,
        node: &PreparedNode
    ) -> InternodeStopAuthority {
        let mut predecessor_stopped_map = HashMap::new();
        let mut successor_stopped_map = HashMap::new();
        for predecessor in node.node.get_predecessors() {
            predecessor_stopped_map.insert(predecessor, false);
        }
        for successor in node.node.get_successors() {
            successor_stopped_map.insert(successor, false); // need to manually start
        }
        InternodeStopAuthority {
            receiver: self.sender.subscribe(),
            sender: self.sender.clone(),
            predecessor_stopped_map,
            successor_stopped_map,
            owner_id: node.id,
        }
    }
}

pub struct InternodeStopAuthority {
    receiver: BroadcastReceiver<NodeStateAuthPacket>,
    sender: BroadcastSender<NodeStateAuthPacket>,
    predecessor_stopped_map: HashMap<usize, bool>, // false is running, true is stopped
    successor_stopped_map: HashMap<usize, bool>,
    owner_id: usize,
}
impl InternodeStopAuthority {
    fn determine_adjacency_state(&self) -> bool {
        if self.predecessor_stopped_map.values().any(|&x| x)
            || self.successor_stopped_map.values().all(|&x| !x)
        {
            false
        } else {
            true
        }
    }

    pub fn send_stop_update(&self, stop: bool) {
        let _ = self.sender.send(NodeStateAuthPacket {
            node_id: self.owner_id,
            stop,
        });
    }

    pub fn update_states(&mut self) -> bool {
        let mut update = false;
        let mut stop = false;
        while let Ok(packet) = self.receiver.try_recv() {
            if self.predecessor_stopped_map.contains_key(&packet.node_id) {
                update = true;
                self.predecessor_stopped_map
                    .insert(packet.node_id, packet.stop);
            } else if self.successor_stopped_map.contains_key(&packet.node_id) {
                update = true;
                self.successor_stopped_map
                    .insert(packet.node_id, packet.stop);
            } else if self.owner_id == packet.node_id {
                update = true;
                stop = true;
            }
        }

        if update && !stop {
            stop = self.determine_adjacency_state();
            self.send_stop_update(stop);
        }

        stop
    }
}


pub struct ExternalStopSource {
    stop_sources: HashMap<usize, WatchSender<bool>>,
    stop_requested: HashMap<usize, bool>,
}
impl ExternalStopSource {
    pub fn new() -> Self {
        Self {
            stop_sources: HashMap::new(),
            stop_requested: HashMap::new(),
        }
    }
    pub fn register_endpoint(&mut self, node_id: usize) -> WatchReceiver<bool> {
        let (sender, receiver) = watch::channel(false);
        self.stop_sources.insert(node_id, sender);
        self.stop_requested.insert(node_id, false);
        receiver
    }
    
    pub fn update_stop_request(&mut self, node_id: usize, stop_requested: bool) -> Result<(), error::SendError<bool>>{
        self.stop_requested.insert(node_id, stop_requested);
        self.stop_sources.get_mut(&node_id).unwrap().send(stop_requested)
    }
}


pub trait NodeStateManager: Send + Sync {
    fn get_state(&self) -> NodeState;
    fn update_state(&mut self, node: &Box<dyn GenericNode>);
}


pub struct EndpointNodeStateManager {
    state: NodeState,
    stop_requested: bool,
    stop_receiver: WatchReceiver<bool>,
    internode_stop_authority: InternodeStopAuthority,
}
impl EndpointNodeStateManager {
    pub fn new(
        stop_receiver: WatchReceiver<bool>,
        internode_stop_authority: InternodeStopAuthority,
        node: &Box<dyn GenericNode>,
    ) -> Self {
        Self {
            state: node.initial_state(),
            stop_requested: false,
            stop_receiver,
            internode_stop_authority,
        }
    }
}
impl NodeStateManager for EndpointNodeStateManager {
    fn get_state(&self) -> NodeState {
        self.state
    }
    fn update_state(&mut self, node: &Box<dyn GenericNode>) {
        let internode_stopped = self.internode_stop_authority.update_states();

        match self.stop_receiver.has_changed() {
            Ok(changed) => {
                if changed {
                    let new_stop_requested = self.stop_receiver.borrow().clone();
                    if self.stop_requested != new_stop_requested {
                        self.stop_requested = new_stop_requested;
                        self.internode_stop_authority
                            .send_stop_update(self.stop_requested);
                    }

                    self.stop_receiver.mark_changed();
                }
            }
            _ => (),
        }
        if internode_stopped || self.stop_requested {
            self.state = NodeState::Stop;
        } else {
            self.state = node.next_state(self.state);
        }
    }
}

pub struct GeneralNodeStateManager {
    // depends only on surrounding nodes
    state: NodeState,
    internode_stop_authority: InternodeStopAuthority,
}
impl GeneralNodeStateManager {
    pub fn new(
        internode_stop_authority: InternodeStopAuthority,
        node: &Box<dyn GenericNode>,
    ) -> Self {
        Self {
            state: node.initial_state(), // initial state here
            internode_stop_authority,
        }
    }
}
impl NodeStateManager for GeneralNodeStateManager {
    fn get_state(&self) -> NodeState {
        self.state
    }
    fn update_state(&mut self, node: &Box<dyn GenericNode>) {
        let stopped = self.internode_stop_authority.update_states();
        if stopped {
            self.state = NodeState::Stop;
        } else {
            self.state = node.next_state(self.state);
        }
    }
}

pub fn generate_node_state_manager(
    node: &PreparedNode,
    internode_stop_authority_factory: &InternodeStopAuthorityFactory,
    stop_receiver: Option<WatchReceiver<bool>>,
) -> Box<dyn NodeStateManager> {
    let internode_stop_authority = internode_stop_authority_factory.new_internode_stop_authority(
        node
    );
    
    match stop_receiver {
        Some(stop_receiver) => Box::new(EndpointNodeStateManager::new(
            stop_receiver,
            internode_stop_authority,
            &node.node
        )),
        None => Box::new(GeneralNodeStateManager::new(
            internode_stop_authority,
            &node.node
        ))
    }
}
