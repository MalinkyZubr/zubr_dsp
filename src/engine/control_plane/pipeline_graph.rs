use crate::engine::data_plane::construction::node_build_vector::{PipelineBuildVector, PreparedNode};
use crate::engine::data_plane::structural::generic_pipeline_node::GenericNode;
use crossbeam::utils::CachePadded;
use log::{info, warn};
use scc::HashMap as SCCHashMap;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, AtomicU8, AtomicUsize};
use std::sync::Arc;
use std::time;
use std::time::UNIX_EPOCH;
use iced::widget::text::success;
use crate::engine::control_plane::node_wrapper::NodeWrapper;

#[derive(PartialEq, Debug)]
pub enum PipelineNodeState {
    Run,
    Stop,
    Dependent,
}
impl PipelineNodeState {
    pub fn into_u8(self) -> u8 {
        match self {
            PipelineNodeState::Run => 0,
            PipelineNodeState::Stop => 1,
            PipelineNodeState::Dependent => 2,
        }
    }

    pub fn from_u8(u8: u8) -> PipelineNodeState {
        match u8 {
            0 => PipelineNodeState::Run,
            1 => PipelineNodeState::Stop,
            2 => PipelineNodeState::Dependent,
            _ => panic!("Invalid state value: {}", u8),
        }
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


pub struct PipelineGraph {
    nodes: SCCHashMap<usize, NodeWrapper>, // holds the actual computation and node data. A node can only be held by one thread at once and should be popped from here when in use
    source_ids: Vec<usize>,
    sink_ids: Vec<usize>,
    initially_stateful_ids: Vec<usize>,
    num_nodes: usize,
}
impl PipelineGraph {
    pub fn new(mut node_wrappers: Vec<NodeWrapper>) -> Self {
        let mut nodes = SCCHashMap::new();
        let mut source_ids = Vec::new();
        let mut sink_ids = Vec::new();
        let mut initially_stateful_ids = Vec::new();

        let num_nodes = node_wrappers.len();

        info!("GRAPH NODE ID NAME MAPPINGS (FOR DEBUG):");
        for node_wrapper in node_wrappers.drain(..) {
            if node_wrapper.is_source() {
                source_ids.push(node_wrapper.get_id());
            }
            else if node_wrapper.is_sink() {
                sink_ids.push(node_wrapper.get_id());
            }
            if node_wrapper.has_initial_state() {
                initially_stateful_ids.push(node_wrapper.get_id());
            }
            let node_id = node_wrapper.get_id();
            match nodes.insert_sync(node_id, node_wrapper) {
                Ok(_) => {}
                Err(_) => panic!("Node {} already exists in the graph", node_id),
            }
        }
        
        Self {
            source_ids,
            sink_ids,
            initially_stateful_ids,
            nodes,
            num_nodes,
        }
    }
    
    pub fn get_num_nodes(&self) -> usize {
        self.num_nodes
    }
    
    pub fn get_all_nodes(&self) -> Vec<NodeWrapper> {
        let mut nodes = Vec::with_capacity(self.nodes.len());

        for id in 0..self.num_nodes {
            if let Some((_id, node)) = self.nodes.remove_if_sync(&id, |_| true) {
                nodes.push(node);
            }
        }

        nodes
    }

    // pub fn get_all_sources(&self) -> Vec<Box<dyn GenericNode>> {
    //     // does not guarantee that all sources are present
    //     let mut nodes = Vec::with_capacity(self.nodes.len());
    // 
    //     for id in 0..self.num_nodes {
    //         if let Some((_id, node)) = self.nodes.remove_if_sync(&id, |_| true) {
    //             if node.is_source() {
    //                 nodes.push(node);
    //             }
    //         }
    //     }
    // 
    //     nodes
    // }
    // 
    // pub fn get_all_sinks(&self) -> Vec<Box<dyn GenericNode>> {
    //     // does not guarantee that all sources are present
    //     let mut nodes = Vec::with_capacity(self.nodes.len());
    // 
    //     for id in 0..self.num_nodes {
    //         if let Some((_id, node)) = self.nodes.remove_if_sync(&id, |_| true) {
    //             if node.is_sink() {
    //                 nodes.push(node);
    //             }
    //         }
    //     }
    // 
    //     nodes
    // }
    // 
    // pub fn get_all_initially_stateful(&self) -> Vec<Box<dyn GenericNode>> {
    //     // does not guarantee that all sources are present
    //     let mut nodes = Vec::with_capacity(self.nodes.len());
    // 
    //     for id in 0..self.num_nodes {
    //         if let Some((_id, node)) = self.nodes.remove_if_sync(&id, |_| true) {
    //             if node.has_initial_state() {
    //                 nodes.push(node);
    //             }
    //         }
    //     }
    // 
    //     nodes
    // }
    
    // pub fn get_all_start_nodes(&self) -> Vec<Box<dyn GenericNode>> {
    //     let mut sources = self.get_all_sources();
    //     sources.extend(self.get_all_initially_stateful());
    //     sources
    // }
    
    pub fn get_all_start_nodes(&self) -> Vec<usize> {
        let mut sources = self.source_ids.clone();
        sources.extend(self.initially_stateful_ids.clone());
        sources
    }

    pub fn stop_sink(&self, _id: usize) {
        todo!()
    }

    // pub async fn stop_source(&self, id: usize) -> bool {
    //     self.state_request_map_source
    //         .update_async(&id, |_, v| {
    //             *v = PipelineNodeState::Stop.into_u8();
    //             *v
    //         })
    //         .await
    //         .is_some()
    // }
    // 
    // pub async fn start_source(&self, id: usize) -> bool {
    //     self.state_request_map_source
    //         .update_async(&id, |_, v| {
    //             *v = PipelineNodeState::Run.into_u8();
    //             *v
    //         })
    //         .await
    //         .is_some()
    // }
    // 
    // pub fn update_analytics(&self, id: usize, execution_time_ns: u64) {
    //     self.master_read_array[id].update_analytics(execution_time_ns);
    // }
    // 
    // pub fn start_all(&self) {}
    // 
    // pub fn stop_all(&self) {}
    // 
    pub fn get_node(&self, id: usize) -> Option<(usize, NodeWrapper)> {
        self.nodes.remove_if_sync(&id, |_| true)
    }
    // 
    pub fn place_node(&self, id: usize, node: NodeWrapper) -> Option<()> {
        self.nodes.insert_sync(id, node).ok()
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::engine::construction::unfinished_node_builder::PipelineParameters;
// 
//     // Mock CollectibleNode for testing
//     #[derive(Debug)]
//     struct MockNode {
//         id: usize,
//         num_inputs: usize,
//         num_outputs: usize,
//         has_initial: bool,
//     }
// 
//     impl MockNode {
//         fn new(id: usize, num_inputs: usize, num_outputs: usize, has_initial: bool) -> Self {
//             Self {
//                 id,
//                 num_inputs,
//                 num_outputs,
//                 has_initial,
//             }
//         }
//     }
// 
//     #[async_trait::async_trait]
//     impl GenericNode for MockNode {
//         fn check_nth_satiated_edge_id(&self, _edge_index: usize) -> Option<usize> {
//             Some(0)
//         }
//         fn is_ready_exec(&self) -> bool {
//             true
//         }
//         fn get_successors(&self) -> Vec<usize> {
//             vec![]
//         }
//         fn get_run_model(&self) -> crate::engine::structural::generic_pipeline_node::RunModel {
//             crate::engine::structural::generic_pipeline_node::RunModel::CPU
//         }
//         fn get_num_inputs(&self) -> usize {
//             self.num_inputs
//         }
//         fn get_num_outputs(&self) -> usize {
//             self.num_outputs
//         }
//         async fn run_senders(&mut self, _id: usize) -> Option<usize> {
//             Some(0)
//         }
//         fn load_initial_state(&mut self) {}
//         fn has_initial_state(&self) -> bool {
//             self.has_initial
//         }
// 
//         fn get_predecessors(&self) -> Vec<usize> {
//             vec![]
//         }
//     }
// 
//     fn create_test_build_vector() -> PipelineBuildVector {
//         let params = PipelineParameters::new(16);
//         let mut build_vector = PipelineBuildVector::new(params);
// 
//         // Add a source node (0 inputs, 2 outputs)
//         build_vector.add_node((
//             0,
//             "source".to_string(),
//             Box::new(MockNode::new(0, 0, 2, false)),
//         ));
// 
//         // Add a processing node (2 inputs, 1 output, with initial state)
//         build_vector.add_node((
//             1,
//             "processor".to_string(),
//             Box::new(MockNode::new(1, 2, 1, true)),
//         ));
// 
//         // Add a sink node (1 input, 0 outputs)
//         build_vector.add_node((
//             2,
//             "sink".to_string(),
//             Box::new(MockNode::new(2, 1, 0, false)),
//         ));
// 
//         build_vector
//     }
// 
//     #[test]
//     fn test_pipeline_node_state_conversions() {
//         assert_eq!(PipelineNodeState::Run.into_u8(), 0);
//         assert_eq!(PipelineNodeState::Stop.into_u8(), 1);
//         assert_eq!(PipelineNodeState::Dependent.into_u8(), 2);
// 
//         assert_eq!(PipelineNodeState::from_u8(0), PipelineNodeState::Run);
//         assert_eq!(PipelineNodeState::from_u8(1), PipelineNodeState::Stop);
//         assert_eq!(PipelineNodeState::from_u8(2), PipelineNodeState::Dependent);
//     }
// 
//     #[test]
//     #[should_panic(expected = "Invalid state value: 3")]
//     fn test_pipeline_node_state_invalid_conversion() {
//         PipelineNodeState::from_u8(3);
//     }
// 
//     #[test]
//     fn test_node_master_read_data_new() {
//         let data = NodeMasterReadData::new();
// 
//         assert_eq!(
//             data.num_executions
//                 .load(std::sync::atomic::Ordering::Acquire),
//             0
//         );
//         assert_eq!(
//             data.last_execution_time_ns
//                 .load(std::sync::atomic::Ordering::Acquire),
//             0
//         );
//         assert_eq!(
//             PipelineNodeState::from_u8(
//                 data.current_state
//                     .load(std::sync::atomic::Ordering::Acquire)
//             ),
//             PipelineNodeState::Stop
//         );
//     }
// 
//     #[test]
//     fn test_node_master_read_data_update_analytics() {
//         let data = NodeMasterReadData::new();
// 
//         data.update_analytics(1500);
// 
//         assert_eq!(
//             data.num_executions
//                 .load(std::sync::atomic::Ordering::Acquire),
//             1
//         );
//         assert_eq!(
//             data.last_execution_time_ns
//                 .load(std::sync::atomic::Ordering::Acquire),
//             1500
//         );
// 
//         data.update_analytics(2000);
// 
//         assert_eq!(
//             data.num_executions
//                 .load(std::sync::atomic::Ordering::Acquire),
//             2
//         );
//         assert_eq!(
//             data.last_execution_time_ns
//                 .load(std::sync::atomic::Ordering::Acquire),
//             2000
//         );
//     }
// 
//     #[test]
//     fn test_node_immutable_data_new() {
//         let data = NodeImmutableData::new("test_node".to_string(), true, false, true);
// 
//         assert_eq!(data.name, "test_node");
//         assert!(data.is_source);
//         assert!(!data.is_sink);
//         assert!(data.initially_stateful);
//     }
// 
//     #[test]
//     fn test_pipeline_graph_new() {
//         let build_vector = create_test_build_vector();
//         let graph = PipelineGraph::new(Rc::new(RefCell::new(build_vector)));
// 
//         // Verify the graph was created successfully
//         assert_eq!(graph.get_node_name(0), "source");
//         assert_eq!(graph.get_node_name(1), "processor");
//         assert_eq!(graph.get_node_name(2), "sink");
//     }
// 
//     #[test]
//     fn test_get_all_sources() {
//         let build_vector = create_test_build_vector();
//         let graph = PipelineGraph::new(Rc::new(RefCell::new(build_vector)));
// 
//         let sources = graph.get_all_sources();
//         assert_eq!(sources.len(), 1);
//         assert!(sources.contains(&0));
//     }
// 
//     #[test]
//     fn test_get_all_sinks() {
//         let build_vector = create_test_build_vector();
//         let graph = PipelineGraph::new(Rc::new(RefCell::new(build_vector)));
// 
//         let sinks = graph.get_all_sinks();
//         assert_eq!(sinks.len(), 1);
//         assert!(sinks.contains(&2));
//     }
// 
//     #[test]
//     fn test_get_all_initially_stateful() {
//         let build_vector = create_test_build_vector();
//         let graph = PipelineGraph::new(Rc::new(RefCell::new(build_vector)));
// 
//         let stateful = graph.get_all_initially_stateful();
//         assert_eq!(stateful.len(), 1);
//         assert!(stateful.contains(&1));
//     }
// 
//     #[test]
//     fn test_get_node_analytics() {
//         let build_vector = create_test_build_vector();
//         let graph = PipelineGraph::new(Rc::new(RefCell::new(build_vector)));
// 
//         let (executions, time, state, tsle) = graph.get_node_analytics(0);
//         assert_eq!(executions, 0);
//         assert_eq!(time, 0);
//         assert_eq!(state, PipelineNodeState::Stop);
//     }
// 
//     #[test]
//     fn test_update_analytics() {
//         let build_vector = create_test_build_vector();
//         let graph = PipelineGraph::new(Rc::new(RefCell::new(build_vector)));
// 
//         graph.update_analytics(0, 1234);
// 
//         let (executions, time, _, tsle) = graph.get_node_analytics(0);
//         assert_eq!(executions, 1);
//         assert_eq!(time, 1234);
//     }
// 
//     #[tokio::test]
//     async fn test_start_stop_source() {
//         let build_vector = create_test_build_vector();
//         let graph = PipelineGraph::new(Rc::new(RefCell::new(build_vector)));
// 
//         // Start source
//         let result = graph.start_source(0).await;
//         assert!(result);
// 
//         // Stop source
//         let result = graph.stop_source(0).await;
//         assert!(result);
// 
//         // Try to start/stop non-existent source
//         let result = graph.start_source(999).await;
//         assert!(!result);
//     }
// 
//     #[test]
//     fn test_get_and_place_node() {
//         let build_vector = create_test_build_vector();
//         let graph = PipelineGraph::new(Rc::new(RefCell::new(build_vector)));
// 
//         // Get a node
//         let node_data = graph.get_node(1);
//         assert!(node_data.is_some());
//         let (id, node) = node_data.unwrap();
//         assert_eq!(id, 1);
// 
//         // Try to get the same node again (should be None since it was removed)
//         let node_data2 = graph.get_node(1);
//         assert!(node_data2.is_none());
// 
//         // Place the node back
//         let result = graph.place_node(1, node);
//         assert!(result.is_some());
// 
//         // Now we should be able to get it again
//         let node_data3 = graph.get_node(1);
//         assert!(node_data3.is_some());
//     }
// 
//     #[test]
//     fn test_get_nonexistent_node() {
//         let build_vector = create_test_build_vector();
//         let graph = PipelineGraph::new(Rc::new(RefCell::new(build_vector)));
// 
//         let node_data = graph.get_node(999);
//         assert!(node_data.is_none());
//     }
// 
//     #[test]
//     fn test_start_stop_all() {
//         let build_vector = create_test_build_vector();
//         let graph = PipelineGraph::new(Rc::new(RefCell::new(build_vector)));
// 
//         // These methods should not panic
//         graph.start_all();
//         graph.stop_all();
//     }
// }
