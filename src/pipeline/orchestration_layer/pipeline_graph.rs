use crate::pipeline::construction_layer::pipeline_node::CollectibleThread;
use collections::HashMap;
use std::collections;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};


pub struct PipelineAdjacencyEdge {
    source: Arc<PipelineAdjacencyNode>,
    destination: Arc<PipelineAdjacencyNode>,
    num_executions_since_completion: AtomicU64,
    num_executions_to_complete: u64,
}


pub struct PipelineAdjacencyNode {
    pub predecessors: Vec<Arc<PipelineAdjacencyNode>>,
    pub successors: Vec<Arc<PipelineAdjacencyNode>>,
    pub thread_object: Mutex<Box<dyn CollectibleThread>>,
    is_running: 
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
            num_executions_since_completion: Arc::new(AtomicU64::new(0)),
            num_executions_to_complete: Arc::new(0), // needs to be derived from the channel metadata somehow
        }
    }
    
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::Acquire)
    }
    
    pub fn is_source(&self) -> bool {
        self.predecessors.len() == 0
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

pub struct PipelineAdjacencyMap {
    adjacency_map: HashMap<String, Arc<PipelineAdjacencyNode>>,
}
impl PipelineAdjacencyMap {
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

    pub fn get_all_sources(&self) -> Vec<Arc<PipelineAdjacencyNode>> {
        let mut sources: Vec<Arc<PipelineAdjacencyNode>> = Vec::new();

        for (_, thread_obj) in self.adjacency_map.iter() {
            if thread_obj.is_source() {
                let thread_clone = thread_obj.clone();
                sources.push(thread_clone);
            }
        }

        sources
    }
}
