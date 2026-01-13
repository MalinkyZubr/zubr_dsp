use std::collections;
use collections::HashMap;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU64, Ordering};
use crate::pipeline::threading_layer::pipeline_thread::CollectibleThread;


pub struct PipelineAdjacencyNode {
    pub predecessors: Vec<String>,
    pub successors: Vec<String>,
    pub thread_object: Mutex<Box<dyn CollectibleThread>>,
    pub num_executions_since_completion: AtomicU64,
    pub num_executions_to_complete: u64
}
impl PipelineAdjacencyNode {
    pub fn new(thread_object: Mutex<Box<dyn CollectibleThread>>) -> Self {
        Self {
            predecessors: thread_object.get_channel_metadata().iter().map(|x| x.origin_id).collect(),
            successors: Vec::new(),
            thread_object,
            num_executions_since_completion: Arc::new(AtomicU64::new(0)),
            num_executions_to_complete: Arc::new(0) // needs to be derived from the channel metadata somehow
        }
    }
    
    pub fn is_source(&self) -> bool {
        self.predecessors.len() == 0
    }
}


pub struct PipelineAdjacencyMap {
    adjacency_map: HashMap<String, PipelineAdjacencyNode>
}
impl PipelineAdjacencyMap {
    fn construct_partial_downstream(collectible_thread_vector: Vec<Box<dyn CollectibleThread>>) -> HashMap<String, PipelineAdjacencyNode> {
        let mut partial_downstream_map = HashMap::new();
        
        for thread in collectible_thread_vector {
            let node = PipelineAdjacencyNode::new(Mutex::new(thread.clone()));
            partial_downstream_map.insert(thread.get_id(), node); // places the predecessors in the adjacency node
        } // Data type: Silly
        
        partial_downstream_map
    }
    
    fn construct_full_adjacency_map(mut partial_downstream_map: HashMap<String, PipelineAdjacencyNode>) -> HashMap<String, PipelineAdjacencyNode> {
        for (dest_id, _) in partial_downstream_map.iter() {
            for (source_id, _) in partial_downstream_map.iter() { // iterate over the source list of each and every node
                partial_downstream_map[source_id].successors.push(dest_id.clone());
            }
        }
        
        partial_downstream_map
    }
    
    pub fn new(collectible_thread_vector: Vec<Box<dyn CollectibleThread>>) -> Self {
        let partial_downstream_map = PipelineAdjacencyMap::construct_partial_downstream(collectible_thread_vector);
        
        PipelineAdjacencyMap {
            adjacency_map: PipelineAdjacencyMap::construct_full_adjacency_map(partial_downstream_map)
        }
    }
    
    pub fn check_dependencies_satisfied(&self, id: &String) -> bool {
        for dependency in self.adjacency_map[id].predecessors.iter() {
            let other_dependency = &self.adjacency_map[dependency];
            if other_dependency.num_executions_since_completion.load(Ordering::Acquire) < *other_dependency.num_executions_to_complete {
                return false;
            }
        }
        true
    }
    
    pub fn get_all_sources(&self) -> Vec<String> {
        let mut sources: Vec<String> = Vec::new();
        
        for (thread_id, thread_obj) in self.adjacency_map.iter() {
            if thread_obj.is_source() {
                sources.push(thread_id.clone());
            }
        }
        
        sources
    }
}