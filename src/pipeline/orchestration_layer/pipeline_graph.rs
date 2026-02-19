use crate::pipeline::construction_layer::node_builder::{PipelineBuildVector};
use atomic_enum::atomic_enum;
use crossbeam::utils::CachePadded;
use itertools::Itertools;
use scc::HashMap as SCCHashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize};
use crate::pipeline::construction_layer::node_types::node_traits::CollectibleNode;


#[atomic_enum]
#[derive(PartialEq)]
pub enum PipelineNodeState {
    Run,
    Stop,
    Dependent
}

#[derive(Debug, Clone)]
pub struct NodeMasterReadData {
    num_executions: Arc<AtomicUsize>,
    last_execution_time_ns: Arc<AtomicU64>,
    current_state: Arc<PipelineNodeState>,
}
impl NodeMasterReadData {
    pub fn new() -> NodeMasterReadData {
        NodeMasterReadData {
            num_executions: Arc::new(AtomicUsize::new(0)),
            last_execution_time_ns: Arc::new(AtomicU64::new(0)),
            current_state: PipelineNodeState::Stop,
        }
    }

    pub fn update_analytics(&self, execution_time_ns: u64) {
        self.num_executions.fetch_add(1, std::sync::atomic::Ordering::Release);
        self.last_execution_time_ns.store(execution_time_ns, std::sync::atomic::Ordering::Release);
    }
}

// #[derive(Debug, Clone)]
// pub struct NodeMasterWriteData {
//     requested_state: PipelineNodeState,
// }
// impl NodeMasterWriteData {
//     pub fn new() -> Self {
//         Self {
//             requested_state: PipelineNodeState::Stop,
//         }
//     }
// }


#[derive(Debug, Clone)]
pub struct NodeImmutableData {
    name: String,
    is_source: bool,
    is_sink: bool,
    initially_stateful: bool,
}
impl NodeImmutableData {
    pub fn new(name: String, is_source: bool, is_sink: bool, initially_stateful: bool) -> Self {
        Self {
            name,
            is_source,
            is_sink,
            initially_stateful,
        }
    }
}

pub struct PipelineGraph {
    state_request_map: SCCHashMap<usize, PipelineNodeState>, // map to hold the requested state for each node. Can be modified by either neighbor nodes or the master thread
    master_read_array: Arc<[CachePadded<NodeMasterReadData>]>, // holds data mutable to the compute threads. The master (main thread) should read analytics and metadata from here
    node_immutable_data: Arc<[NodeImmutableData]>, // holds data that can be read by all but is immutable.
    mutable_state_map: SCCHashMap<usize, Box<dyn CollectibleNode>>, // holds the actual computation and node data. A node can only be held by one thread at once and should be popped from here when in use
}
impl PipelineGraph {
    pub fn new(build_vector: PipelineBuildVector) -> Self {
        let nodes = build_vector.consume();
        let mut master_read_vec = vec![CachePadded::new(NodeMasterReadData::new()); nodes.len()];
        let mut node_immutable_data =
            vec![NodeImmutableData::new(String::from(""), false, false, false); nodes.len()];
        let mutable_state_map = SCCHashMap::with_capacity(nodes.len());
        let state_request_map = SCCHashMap::new();

        for (id, name, node) in nodes {
            node_immutable_data[id] = NodeImmutableData::new(name, node.get_num_inputs() == 0, node.get_num_outputs() == 0, node.has_initial_state());
            *master_read_vec[id] = NodeMasterReadData::new();
            
            if node.is_source() || node.is_sink() {
                match state_request_map.insert_sync(id, PipelineNodeState::Stop) {
                    Ok(_) => {}
                    Err(_) => panic!("Node {} already exists in the graph", id),
                }
            }
            
            match mutable_state_map.insert_sync(id, node) {
                Ok(_) => {}
                Err(_) => panic!("Node {} already exists in the graph", id),
            }
        }
        Self {
            state_request_map,
            master_read_array: Arc::from(master_read_vec),
            node_immutable_data: Arc::from(node_immutable_data),
            mutable_state_map,
        }
    }

    pub fn get_all_sources(&self) -> Vec<usize> {
        let mut sources = vec![];
        for (id, immutable_data) in self.node_immutable_data.iter().enumerate() {
            if immutable_data.is_source {
                sources.push(id);
            }
        }
        sources
    }

    pub fn get_all_sinks(&self) -> Vec<usize> {
        let mut sinks = vec![];
        for (id, immutable_data) in self.node_immutable_data.iter().enumerate() {
            if immutable_data.is_sink {
                sinks.push(id);
            }
        }
        sinks
    }

    pub fn get_all_initially_stateful(&self) -> Vec<usize> {
        let mut initially_stateful = vec![];
        for (id, immutable_data) in self.node_immutable_data.iter().enumerate() {
            if immutable_data.initially_stateful {
                initially_stateful.push(id);
            }
        }
        initially_stateful
    }

    pub fn get_node_name(&self, id: usize) -> String {
        self.node_immutable_data[id].name.clone()
    }

    pub fn get_node_analytics(&self, id: usize) -> (usize, u64, PipelineNodeState) {
        (
            self.master_read_array[id].num_executions,
            self.master_read_array[id].last_execution_time_ns,
            self.master_read_array[id].current_state,
        )
    }

    pub fn stop_sink(&self, id: usize) {
        todo!()
    }

    pub async fn stop_source(&self, id: usize) -> bool {
        self.state_request_map.update_async(&id, |_, v| { *v = PipelineNodeState::Stop; *v }).await.is_some()
    }
    
    pub async fn start_source(&self, id: usize) -> bool {
        self.state_request_map.update_async(&id, |_, v| { *v = PipelineNodeState::Run; *v }).await.is_some()
    }
    
    pub fn update_analytics(&self, id: usize, execution_time_ns: u64) {
        self.master_read_array[id].update_analytics(execution_time_ns);
    }

    pub fn start_all(&self) {
        
    }

    pub fn stop_all(&self) {
        
    }
    
    pub fn get_node(&self, id: usize) -> Option<(usize, Box<dyn CollectibleNode>)> {
        self.mutable_state_map.remove_if_sync(&id, |_| true)
    }
    
    pub fn place_node(&self, id: usize, node: Box<dyn CollectibleNode>) -> Option<()> {
        self.mutable_state_map.insert_sync(id, node).ok()
    }
}
