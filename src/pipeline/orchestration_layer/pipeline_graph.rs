use std::collections::HashMap;
use crate::pipeline::construction_layer::pipeline_node::CollectibleThread;
use atomic_enum::atomic_enum;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use itertools::Itertools;
use crate::pipeline::construction_layer::builders::BuildingNode;

pub struct PipelineAdjacencyEdge {
    source_id: usize,
    destination_id: usize,
    num_executions_since_completion: AtomicU64,
    num_executions_to_complete: u64,
    is_stopped: Arc<AtomicBool>,
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

    pub fn increment_num_sends(&self, amount: u64) {
        self.num_executions_since_completion.fetch_add(amount, Ordering::Release);
    }

    pub fn decrement_num_sends(&self, amount: u64) {
        self.num_executions_since_completion
            .fetch_sub(amount, Ordering::Release);
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
    predecessors: Vec<Arc<PipelineAdjacencyEdge>>,
    successors: Vec<Arc<PipelineAdjacencyEdge>>,
    thread_object: Mutex<dyn CollectibleThread>,
    current_state: Arc<PipelineNodeState>,
    requested_state: Arc<PipelineNodeState>,
    currently_running: AtomicBool,
    node_id: usize,
    node_name: String,
}
impl PipelineAdjacencyNode {
    pub fn new(thread_object: Mutex<Box<dyn CollectibleThread>>, id: usize, node_name: String) -> Self {
        Self {
            predecessors: Vec::new(),
            successors: Vec::new(),
            thread_object,
            node_id: id,
            node_name,
            currently_running: AtomicBool::new(false),
            requested_state: Arc::new(PipelineNodeState::Stop),
            current_state: Arc::new(PipelineNodeState::Stop)
        }
    }

    pub fn enter_execution(&self) {
        self.currently_running.store(true, Ordering::Release);
        for predecessor in self.get_predecessors() {
            predecessor.record_consumption();
        }
    }

    pub fn exit_execution(&self) {
        self.currently_running.store(false, Ordering::Release);
    }

    pub fn get_predecessors(&self) -> &Vec<Arc<PipelineAdjacencyEdge>> {
        &self.predecessors
    }

    pub fn get_successors(&self) -> &Vec<Arc<PipelineAdjacencyEdge>> {
        &self.successors
    }

    pub fn get_thread_object(&self) -> &Mutex<dyn CollectibleThread> {
        &self.thread_object
    }

    pub fn get_name(&self) -> &String {
        &self.node_name
    }
    pub fn get_id(&self) -> usize {
        self.node_id
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

    pub fn predecessors_responsibility_fulfilled(&self) -> bool {
        for predecessor in self.predecessors.iter() {
            if !predecessor.responsibility_fulfilled() {
                return false;
            }
        }
        true
    }

    pub fn node_ready_execute(&self) -> bool {
        self.predecessors_responsibility_fulfilled() && !self.currently_running.load(Ordering::Acquire)
    }
}

pub struct PipelineGraph {
    adjacency_list: Arc<Vec<Arc<PipelineAdjacencyNode>>>,
}
impl PipelineGraph {
    fn create_graph_nodes(
        build_vector: Vec<BuildingNode>,
    ) -> (Vec<Arc<PipelineAdjacencyNode>>, Vec<(usize, HashMap<usize, usize>)>) {
        let mut partial_downstream_vec = Vec::with_capacity(build_vector.len());
        let mut adjacency_vec: Vec<(usize, HashMap<usize, usize>)> = Vec::with_capacity(build_vector.len());

        for node in build_vector { //
            adjacency_vec.push((node.id, node.successors));
            let adj_node = PipelineAdjacencyNode::new(Mutex::new(node.thread), node.id, node.name);

            partial_downstream_vec.push(Arc::new(adj_node)); // places the predecessors in the adjacency node
        } // Data type: Silly

        (partial_downstream_vec, adjacency_vec)
    }

    fn attach_edges(
        partial_downstream_vec: &mut Vec<Arc<PipelineAdjacencyNode>>,
        mut adjacency_vec: Vec<(usize, HashMap<usize, usize>)>
    ) {
        for source_adjacency in adjacency_vec.iter_mut() {
            for dest_adjacency in adjacency_vec.iter_mut() {
                let source_id = source_adjacency.0;
                let dest_id = dest_adjacency.0;
                if dest_adjacency.1.contains(&source_id) {
                    let stop_flag = partial_downstream_vec[source_id].thread_object.get_mut().unwrap().clone_stop_flag(dest_id);
                    let new_edge = Arc::new(PipelineAdjacencyEdge {
                        source_id: source_id,
                        destination_id: dest_id,
                        num_executions_since_completion: AtomicU64::new(0),
                        num_executions_to_complete: *source_adjacency.1.get(&dest_id).unwrap() as u64,
                        is_stopped: stop_flag
                    });
                    partial_downstream_vec[dest_adjacency.0].successors.push(new_edge.clone());
                    partial_downstream_vec[source_adjacency.0].predecessors.push(new_edge.clone());
                    source_adjacency.1.retain(|&x| x != dest_adjacency.0);
                }
            }
        }
    }

    pub fn new(build_vector: Vec<BuildingNode>) -> Self {
        let (mut partial_downstream_vec, adj_vec) = Self::create_graph_nodes(build_vector);
        Self::attach_edges(&mut partial_downstream_vec, adj_vec);
        Self {
            adjacency_list: Arc::new(partial_downstream_vec),
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
            //self.adjacency_list[node_id].requested_state = Arc::new(PipelineNodeState::Stop); // find better way
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
                    for next_predecessor in self.stop_sink(predecessor.get_destination_id()) {
                        if !predecessors_to_stop.contains(&next_predecessor) {
                            predecessors_to_stop.push(next_predecessor);
                        }
                    }
                }
                predecessors_to_stop
            }
            None => Vec::new()
        }
    }

    pub fn stop_source(&self, id: usize) {
        for node in self.stop_source_get_nodes(id) {
            self.adjacency_list[node].requested_state = Arc::new(PipelineNodeState::Stop); // fix later
        }
    }

    fn stop_source_get_nodes(&self, id: usize) -> Vec<usize> {
        let mut nodes_to_stop = Vec::new();
        for sink in self.stop_source_get_depending_sinks(id) {
            for node in self.stop_sink_get_nodes(sink) {
                if !nodes_to_stop.contains(&node) {
                    nodes_to_stop.push(node);
                }
            }
        }

        nodes_to_stop
    }

    fn stop_source_get_depending_sinks(&self, id: usize) -> Vec<usize> {
        let node = self.get_node(id);

        match node {
            Some(node) => {
                if node.is_sink() {
                    return vec![id];
                }
                let mut successors_to_stop = Vec::new();
                for successor in node.get_successors() {
                    for next_successor in self.stop_source_get_depending_sinks(successor.get_destination_id()) {
                        if !successors_to_stop.contains(&next_successor) {
                            successors_to_stop.push(next_successor);
                        }
                    }
                }
                successors_to_stop
            }
            None => Vec::new()
        }
    }
    
    pub fn start_all(&self) {
        
    }
    
    pub fn stop_all(&self) {
        
    }
}
