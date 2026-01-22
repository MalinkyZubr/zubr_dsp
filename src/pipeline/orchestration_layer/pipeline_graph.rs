use crate::pipeline::construction_layer::pipeline_node::CollectibleThread;
use atomic_enum::atomic_enum;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use itertools::Itertools;

pub struct PipelineAdjacencyEdge {
    source_id: usize,
    destination_id: usize,
    num_executions_since_completion: AtomicU64,
    num_executions_to_complete: u64,
}
impl PipelineAdjacencyEdge {
    pub fn get_source_id(&self) -> usize {
        self.source_id
    }

    pub fn get_destination_id(&self) -> usize {
        self.destination_id
    }

    pub fn responsibility_fulfilled(&self) -> bool {
        self.num_executions_since_completion.load(Ordering::Acquire) >= self.num_executions_to_complete
    }

    pub fn increment_num_executions(&self) {
        self.num_executions_since_completion.fetch_add(1, Ordering::Release);
    }

    pub fn decrement_num_executions(&self) {
        if self.responsibility_fulfilled() {
            self.num_executions_since_completion.fetch_sub(self.num_executions_to_complete, Ordering::Release);
        }
    }
}


#[atomic_enum]
#[derive(PartialEq)]
pub enum PipelineNodeState {
    Run,
    Stop,
    Error,
}


pub struct PipelineAdjacencyNode {
    predecessors: Vec<PipelineAdjacencyEdge>,
    successors: Vec<PipelineAdjacencyEdge>,
    thread_object: Mutex<dyn CollectibleThread>,
    current_state: Arc<PipelineNodeState>,
    requested_state: Arc<PipelineNodeState>,
    node_name: String,
}
impl PipelineAdjacencyNode {
    pub fn new(thread_object: Mutex<Box<dyn CollectibleThread>>) -> Self {
        Self {
            predecessors: thread_object
                .get_channel_metadata()
                .iter()
                .map(|x| x.origin_id)
                .collect(),
            successors: Vec::new(),
            thread_object,
        }
    }

    pub fn get_predecessors(&self) -> &Vec<PipelineAdjacencyEdge> {
        &self.predecessors
    }

    pub fn get_successors(&self) -> &Vec<PipelineAdjacencyEdge> {
        &self.successors
    }

    pub fn get_thread_object(&self) -> &Mutex<dyn CollectibleThread> {
        &self.thread_object
    }

    pub fn get_name(&self) -> &String {
        &self.node_name
    }

    pub fn is_running(&self) -> bool {
        *self.current_state == PipelineNodeState::Run
    }

    pub fn is_source(&self) -> bool {
        self.predecessors.len() == 0
    }

    pub fn is_sink(&self) -> bool {
        self.successors.len() == 0
    }

    pub fn record_consumption(&self) {
        self.num_executions_since_completion
            .fetch_sub(self.num_executions_to_complete, Ordering::Release);
    }

    pub fn responsibility_fulfilled(&self) -> bool {
        self.num_executions_since_completion.load(Ordering::Acquire)
            >= self.num_executions_to_complete
    }

    pub fn predecessors_responsibility_fulfilled(&self) -> bool {
        for predecessor in self.predecessors.iter() {
            if !predecessor.responsibility_fulfilled() {
                return false;
            }
        }
        true
    }

    pub fn node_ready_execute(&self) -> bool {
        self.predecessors_responsibility_fulfilled() &&
    }
}

pub struct PipelineGraph {
    adjacency_list: Arc<Vec<Arc<PipelineAdjacencyNode>>>,
}
impl PipelineGraph {
    fn construct_partial_downstream(
        collectible_thread_vector: Vec<Box<dyn CollectibleThread>>,
    ) -> HashMap<String, Arc<PipelineAdjacencyNode>> {
        let mut partial_downstream_map = HashMap::new();

        for thread in collectible_thread_vector { // 
            let thread_id = thread.get_id();
            let node = PipelineAdjacencyNode::new(Mutex::new(thread));

            partial_downstream_map.insert(thread_id, Arc::new(node)); // places the predecessors in the adjacency node
        } // Data type: Silly

        partial_downstream_map
    }

    fn attach_predecessors(
        mut partial_downstream_map: HashMap<String, Arc<PipelineAdjacencyNode>>,
    ) -> HashMap<String, Arc<PipelineAdjacencyNode>> {
        for (_, node) in partial_downstream_map.iter_mut() {
            for metadata in *node.thread_object.lock().unwrap().get_channel_metadata() {
                node.predecessors
                    .push(partial_downstream_map[&metadata.origin_id].clone());
            }
        }

        partial_downstream_map
    }

    fn construct_full_adjacency_map(
        mut partial_downstream_map: HashMap<String, Arc<PipelineAdjacencyNode>>,
    ) -> HashMap<String, Arc<PipelineAdjacencyNode>> {
        for (_, dest_node) in partial_downstream_map.iter() {
            for source_node in dest_node.predecessors {
                // iterate over the source list of each and every node
                source_node.successors.push(dest_node.clone());
            }
        }

        partial_downstream_map
    }

    pub fn new(collectible_thread_vector: Vec<Box<dyn CollectibleThread>>) -> Self {
        let partial_downstream_map =
            PipelineAdjacencyMap::construct_partial_downstream(collectible_thread_vector);
        let partial_downstream_map =
            PipelineAdjacencyMap::attach_predecessors(partial_downstream_map);

        PipelineAdjacencyMap {
            adjacency_map: PipelineAdjacencyMap::construct_full_adjacency_map(
                partial_downstream_map,
            ),
        }
    }

    pub fn get_all_sources(&self) -> Vec<usize> {
        let mut sources: Vec<usize> = Vec::new();

        for (index, thread_obj) in self.adjacency_list.iter().enumerate() {
            if thread_obj.is_source() {
                sources.push(index);
            }
        }

        sources
    }

    pub fn get_node(&self, node_id: usize) -> Option<Arc<PipelineAdjacencyNode>> {
        *self.adjacency_list.get(node_id).clone()
    }

    pub fn stop_sink(&self, id: usize) {
        let nodes_to_stop = self.stop_sink_get_nodes(id);
        for node_id in nodes_to_stop {
            self.adjacency_list[node_id].requested_state = Arc::new(PipelineNodeState::Stop);
        }
    }
    fn stop_sink_get_nodes(&self, id: usize) -> Vec<usize> {
        let node = self.get_node(id);

        match node {
            Some(node) => {
                if node.is_source() {
                    return vec![id];
                }
                let mut predecessors_to_stop = Vec::new();
                for predecessor in node.get_predecessors() {
                    for nest_predecessor in self.stop_sink(predecessor.get_destination_id()) {
                        if !predecessors_to_stop.contains(&nest_predecessor) {
                            predecessors_to_stop.push(nest_predecessor);
                        }
                    }
                }
                predecessors_to_stop
            }
            None => Vec::new()
        }
    }

    fn stop_source_get_nodes(&self, id: usize) -> Vec<usize> {
        let node = self.get_node(id);


    }

    fn stop_source_get_forward_nodes(&)
}
