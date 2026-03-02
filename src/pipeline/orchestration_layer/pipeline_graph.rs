use crate::pipeline::construction_layer::node_builder::PipelineBuildVector;
use crate::pipeline::construction_layer::node_types::node_traits::CollectibleNode;
use crossbeam::utils::CachePadded;
use scc::HashMap as SCCHashMap;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, AtomicU8, AtomicUsize};
use std::sync::Arc;

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

#[derive(Debug, Clone)]
pub struct NodeMasterReadData {
    num_executions: Arc<AtomicUsize>,
    last_execution_time_ns: Arc<AtomicU64>,
    current_state: Arc<AtomicU8>,
}
impl NodeMasterReadData {
    pub fn new() -> NodeMasterReadData {
        NodeMasterReadData {
            num_executions: Arc::new(AtomicUsize::new(0)),
            last_execution_time_ns: Arc::new(AtomicU64::new(0)),
            current_state: Arc::new(AtomicU8::new(PipelineNodeState::Stop.into_u8())),
        }
    }

    pub fn update_analytics(&self, execution_time_ns: u64) {
        self.num_executions
            .fetch_add(1, std::sync::atomic::Ordering::Release);
        self.last_execution_time_ns
            .store(execution_time_ns, std::sync::atomic::Ordering::Release);
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
    state_request_map_source: SCCHashMap<usize, u8>, // map to hold the requested state for each node. Can be modified by either neighbor nodes or the master thread
    state_request_map_sink: SCCHashMap<usize, u8>,
    master_read_array: Arc<[CachePadded<NodeMasterReadData>]>, // holds data mutable to the compute threads. The master (main thread) should read analytics and metadata from here
    node_immutable_data: Arc<[NodeImmutableData]>, // holds data that can be read by all but is immutable.
    mutable_state_map: SCCHashMap<usize, Box<dyn CollectibleNode>>, // holds the actual computation and node data. A node can only be held by one thread at once and should be popped from here when in use
}
impl PipelineGraph {
    pub fn new(build_vector: Rc<RefCell<PipelineBuildVector>>) -> Self {
        let nodes = match Rc::try_unwrap(build_vector) {
            Ok(build_vector) => build_vector.into_inner().consume(),
            Err(_) => panic!("PipelineBuildVector is not cloneable"),
        };

        let mut master_read_vec = vec![CachePadded::new(NodeMasterReadData::new()); nodes.len()];
        let mut node_immutable_data =
            vec![NodeImmutableData::new(String::from(""), false, false, false); nodes.len()];
        let mutable_state_map = SCCHashMap::with_capacity(nodes.len());
        let state_request_map_source = SCCHashMap::new();
        let state_request_map_sink = SCCHashMap::new();

        for (id, name, node) in nodes {
            node_immutable_data[id] = NodeImmutableData::new(
                name,
                node.get_num_inputs() == 0,
                node.get_num_outputs() == 0,
                node.has_initial_state(),
            );
            *master_read_vec[id] = NodeMasterReadData::new();

            if node.is_source() {
                match state_request_map_source.insert_sync(id, PipelineNodeState::Stop.into_u8()) {
                    Ok(_) => {}
                    Err(_) => panic!("Node {} already exists in the graph", id),
                }
            }

            if node.is_sink() {
                match state_request_map_sink.insert_sync(id, PipelineNodeState::Stop.into_u8()) {
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
            state_request_map_source,
            state_request_map_sink,
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
            self.master_read_array[id]
                .num_executions
                .load(std::sync::atomic::Ordering::Acquire),
            self.master_read_array[id]
                .last_execution_time_ns
                .load(std::sync::atomic::Ordering::Acquire),
            PipelineNodeState::from_u8(
                self.master_read_array[id]
                    .current_state
                    .load(std::sync::atomic::Ordering::Acquire),
            ),
        )
    }

    pub fn stop_sink(&self, _id: usize) {
        todo!()
    }

    pub async fn stop_source(&self, id: usize) -> bool {
        self.state_request_map_source
            .update_async(&id, |_, v| {
                *v = PipelineNodeState::Stop.into_u8();
                *v
            })
            .await
            .is_some()
    }

    pub async fn start_source(&self, id: usize) -> bool {
        self.state_request_map_source
            .update_async(&id, |_, v| {
                *v = PipelineNodeState::Run.into_u8();
                *v
            })
            .await
            .is_some()
    }

    pub fn update_analytics(&self, id: usize, execution_time_ns: u64) {
        self.master_read_array[id].update_analytics(execution_time_ns);
    }

    pub fn start_all(&self) {}

    pub fn stop_all(&self) {}

    pub fn get_node(&self, id: usize) -> Option<(usize, Box<dyn CollectibleNode>)> {
        self.mutable_state_map.remove_if_sync(&id, |_| true)
    }

    pub fn place_node(&self, id: usize, node: Box<dyn CollectibleNode>) -> Option<()> {
        self.mutable_state_map.insert_sync(id, node).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::construction_layer::node_builder::PipelineParameters;

    // Mock CollectibleNode for testing
    #[derive(Debug)]
    struct MockNode {
        id: usize,
        num_inputs: usize,
        num_outputs: usize,
        has_initial: bool,
    }

    impl MockNode {
        fn new(id: usize, num_inputs: usize, num_outputs: usize, has_initial: bool) -> Self {
            Self {
                id,
                num_inputs,
                num_outputs,
                has_initial,
            }
        }
    }

    #[async_trait::async_trait]
    impl CollectibleNode for MockNode {
        fn is_ready_exec(&self) -> bool {
            true
        }
        fn get_successors(&self) -> Vec<usize> {
            vec![]
        }
        fn get_run_model(
            &self,
        ) -> crate::pipeline::construction_layer::node_types::node_traits::RunModel {
            crate::pipeline::construction_layer::node_types::node_traits::RunModel::CPU
        }
        fn get_num_inputs(&self) -> usize {
            self.num_inputs
        }
        fn get_num_outputs(&self) -> usize {
            self.num_outputs
        }
        async fn run_senders(&mut self, _id: usize) -> Option<Vec<usize>> {
            Some(vec![])
        }
        fn load_initial_state(&mut self) {}
        fn has_initial_state(&self) -> bool {
            self.has_initial
        }
    }

    fn create_test_build_vector() -> PipelineBuildVector {
        let params = PipelineParameters::new(16);
        let mut build_vector = PipelineBuildVector::new(params);

        // Add a source node (0 inputs, 2 outputs)
        build_vector.add_node((
            0,
            "source".to_string(),
            Box::new(MockNode::new(0, 0, 2, false)),
        ));

        // Add a processing node (2 inputs, 1 output, with initial state)
        build_vector.add_node((
            1,
            "processor".to_string(),
            Box::new(MockNode::new(1, 2, 1, true)),
        ));

        // Add a sink node (1 input, 0 outputs)
        build_vector.add_node((
            2,
            "sink".to_string(),
            Box::new(MockNode::new(2, 1, 0, false)),
        ));

        build_vector
    }

    #[test]
    fn test_pipeline_node_state_conversions() {
        assert_eq!(PipelineNodeState::Run.into_u8(), 0);
        assert_eq!(PipelineNodeState::Stop.into_u8(), 1);
        assert_eq!(PipelineNodeState::Dependent.into_u8(), 2);

        assert_eq!(PipelineNodeState::from_u8(0), PipelineNodeState::Run);
        assert_eq!(PipelineNodeState::from_u8(1), PipelineNodeState::Stop);
        assert_eq!(PipelineNodeState::from_u8(2), PipelineNodeState::Dependent);
    }

    #[test]
    #[should_panic(expected = "Invalid state value: 3")]
    fn test_pipeline_node_state_invalid_conversion() {
        PipelineNodeState::from_u8(3);
    }

    #[test]
    fn test_node_master_read_data_new() {
        let data = NodeMasterReadData::new();

        assert_eq!(
            data.num_executions
                .load(std::sync::atomic::Ordering::Acquire),
            0
        );
        assert_eq!(
            data.last_execution_time_ns
                .load(std::sync::atomic::Ordering::Acquire),
            0
        );
        assert_eq!(
            PipelineNodeState::from_u8(
                data.current_state
                    .load(std::sync::atomic::Ordering::Acquire)
            ),
            PipelineNodeState::Stop
        );
    }

    #[test]
    fn test_node_master_read_data_update_analytics() {
        let data = NodeMasterReadData::new();

        data.update_analytics(1500);

        assert_eq!(
            data.num_executions
                .load(std::sync::atomic::Ordering::Acquire),
            1
        );
        assert_eq!(
            data.last_execution_time_ns
                .load(std::sync::atomic::Ordering::Acquire),
            1500
        );

        data.update_analytics(2000);

        assert_eq!(
            data.num_executions
                .load(std::sync::atomic::Ordering::Acquire),
            2
        );
        assert_eq!(
            data.last_execution_time_ns
                .load(std::sync::atomic::Ordering::Acquire),
            2000
        );
    }

    #[test]
    fn test_node_immutable_data_new() {
        let data = NodeImmutableData::new("test_node".to_string(), true, false, true);

        assert_eq!(data.name, "test_node");
        assert!(data.is_source);
        assert!(!data.is_sink);
        assert!(data.initially_stateful);
    }

    #[test]
    fn test_pipeline_graph_new() {
        let build_vector = create_test_build_vector();
        let graph = PipelineGraph::new(Rc::new(RefCell::new(build_vector)));

        // Verify the graph was created successfully
        assert_eq!(graph.get_node_name(0), "source");
        assert_eq!(graph.get_node_name(1), "processor");
        assert_eq!(graph.get_node_name(2), "sink");
    }

    #[test]
    fn test_get_all_sources() {
        let build_vector = create_test_build_vector();
        let graph = PipelineGraph::new(Rc::new(RefCell::new(build_vector)));

        let sources = graph.get_all_sources();
        assert_eq!(sources.len(), 1);
        assert!(sources.contains(&0));
    }

    #[test]
    fn test_get_all_sinks() {
        let build_vector = create_test_build_vector();
        let graph = PipelineGraph::new(Rc::new(RefCell::new(build_vector)));

        let sinks = graph.get_all_sinks();
        assert_eq!(sinks.len(), 1);
        assert!(sinks.contains(&2));
    }

    #[test]
    fn test_get_all_initially_stateful() {
        let build_vector = create_test_build_vector();
        let graph = PipelineGraph::new(Rc::new(RefCell::new(build_vector)));

        let stateful = graph.get_all_initially_stateful();
        assert_eq!(stateful.len(), 1);
        assert!(stateful.contains(&1));
    }

    #[test]
    fn test_get_node_analytics() {
        let build_vector = create_test_build_vector();
        let graph = PipelineGraph::new(Rc::new(RefCell::new(build_vector)));

        let (executions, time, state) = graph.get_node_analytics(0);
        assert_eq!(executions, 0);
        assert_eq!(time, 0);
        assert_eq!(state, PipelineNodeState::Stop);
    }

    #[test]
    fn test_update_analytics() {
        let build_vector = create_test_build_vector();
        let graph = PipelineGraph::new(Rc::new(RefCell::new(build_vector)));

        graph.update_analytics(0, 1234);

        let (executions, time, _) = graph.get_node_analytics(0);
        assert_eq!(executions, 1);
        assert_eq!(time, 1234);
    }

    #[tokio::test]
    async fn test_start_stop_source() {
        let build_vector = create_test_build_vector();
        let graph = PipelineGraph::new(Rc::new(RefCell::new(build_vector)));

        // Start source
        let result = graph.start_source(0).await;
        assert!(result);

        // Stop source
        let result = graph.stop_source(0).await;
        assert!(result);

        // Try to start/stop non-existent source
        let result = graph.start_source(999).await;
        assert!(!result);
    }

    #[test]
    fn test_get_and_place_node() {
        let build_vector = create_test_build_vector();
        let graph = PipelineGraph::new(Rc::new(RefCell::new(build_vector)));

        // Get a node
        let node_data = graph.get_node(1);
        assert!(node_data.is_some());
        let (id, node) = node_data.unwrap();
        assert_eq!(id, 1);

        // Try to get the same node again (should be None since it was removed)
        let node_data2 = graph.get_node(1);
        assert!(node_data2.is_none());

        // Place the node back
        let result = graph.place_node(1, node);
        assert!(result.is_some());

        // Now we should be able to get it again
        let node_data3 = graph.get_node(1);
        assert!(node_data3.is_some());
    }

    #[test]
    fn test_get_nonexistent_node() {
        let build_vector = create_test_build_vector();
        let graph = PipelineGraph::new(Rc::new(RefCell::new(build_vector)));

        let node_data = graph.get_node(999);
        assert!(node_data.is_none());
    }

    #[test]
    fn test_start_stop_all() {
        let build_vector = create_test_build_vector();
        let graph = PipelineGraph::new(Rc::new(RefCell::new(build_vector)));

        // These methods should not panic
        graph.start_all();
        graph.stop_all();
    }
}
