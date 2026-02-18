use crate::pipeline::construction_layer::node_builder::{PipelineBuildVector};
use atomic_enum::atomic_enum;
use crossbeam::utils::CachePadded;
use itertools::Itertools;
use scc::HashMap as SCCHashMap;
use std::sync::Arc;
use crate::pipeline::construction_layer::node_types::node_traits::CollectibleNode;

#[atomic_enum]
#[derive(PartialEq)]
pub enum PipelineNodeState {
    Run,
    Stop,
    Error,
}

#[derive(Debug, Clone)]
pub struct NodeMasterReadData {
    num_executions: usize,
    last_execution_time_ns: u64,
    current_state: PipelineNodeState,
}
impl NodeMasterReadData {
    pub fn new() -> NodeMasterReadData {
        NodeMasterReadData {
            num_executions: 0,
            last_execution_time_ns: 0,
            current_state: PipelineNodeState::Stop,
        }
    }

    pub fn update_analytics(&mut self, execution_time_ns: u64) {
        self.num_executions += 1;
        self.last_execution_time_ns = execution_time_ns;
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
    state_request_array: Arc<[PipelineNodeState]>, // array to hold the requested state for each node. Can be modified by either neighbor nodes or the master thread
    master_read_array: Arc<[CachePadded<NodeMasterReadData>]>, // holds data mutable to the compute threads. The master (main thread) should read analytics and metadata from here
    node_immutable_data: Arc<[NodeImmutableData]>, // holds data that can be read by all but is immutable.
    mutable_state_map: SCCHashMap<usize, Box<dyn CollectibleNode>>, // holds the actual computation and node data. A node can only be held by one thread at once and should be popped from here when in use
}
impl PipelineGraph {
    pub fn new(build_vector: PipelineBuildVector) -> Self {
        let nodes = build_vector.consume();
        let state_request_vec = vec![PipelineNodeState::Stop; nodes.len()];
        let mut master_read_vec = vec![CachePadded::new(NodeMasterReadData::new()); nodes.len()];
        let mut node_immutable_data =
            vec![NodeImmutableData::new(String::from(""), false, false, false); nodes.len()];
        let mutable_state_map = SCCHashMap::with_capacity(nodes.len());

        for (id, name, node) in nodes {
            node_immutable_data[id] = NodeImmutableData::new(name, node.get_num_inputs() == 0, node.get_num_outputs() == 0, node.has_initial_state());
            *master_read_vec[id] = NodeMasterReadData::new();
            mutable_state_map.insert_sync(id, node);
        }
        Self {
            state_request_array: Arc::from(state_request_vec),
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
    fn stop_sink_get_nodes(&self, id: usize) -> Vec<usize> {
        todo!()
    }

    pub fn stop_source(&self, id: usize) {
        todo!()
    }

    fn stop_source_get_nodes(&self, id: usize) -> Vec<usize> {
        todo!()
    }

    fn stop_source_get_depending_sinks(&self, id: usize) -> Vec<usize> {
        todo!()
    }

    fn detect_improperly_handled_loops(&self) -> bool {
        todo!()
    }

    pub fn get_all_predecessors(&self, node_id: usize) -> Vec<usize> {
        todo!()
    }

    pub fn start_all(&self) {}

    pub fn stop_all(&self) {}
}
