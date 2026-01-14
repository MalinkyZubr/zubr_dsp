#![feature(mpmc_channel)]

use crate::pipeline::orchestration_layer::pipeline_graph::{
    PipelineAdjacencyMap, PipelineAdjacencyNode,
};
use crate::pipeline::orchestration_layer::thread_pool::{
    StaticThreadPoolTopographical, StaticThreadTaskTopographical,
};
use crossterm::ExecutableCommand;
use std::sync::atomic::Ordering;
use std::sync::Once;
use crate::pipeline::construction_layer::pipeline_node::CollectibleThread;
// would all be more efficient and intuitive if there was an actual well defined
// conception of successor and predecessor relationships between threads/nodes
// this way minimal synchronization would be needed. That way, only when predecessors are unfulfilled is there a lockup further down the line

impl StaticThreadTaskTopographical for PipelineAdjacencyNode {
    fn execute(&mut self) -> (Vec<Box<dyn StaticThreadTaskTopographical + Send>>, bool) { // need checks to prevent concurrently running same node more than once at a time
        for predecessor in self.predecessors.iter() {
            *predecessor.record_consumption();
        }

        { // need to ensure that the mutable ref is valid for minimum time
            let mut thread_object_mut = self.thread_object.get_mut().unwrap();
            thread_object_mut.execute().unwrap();
        }
        self.num_executions_since_completion
            .fetch_add(1, Ordering::Release);

        let mut ready_successors = Vec::new();
        for successor in self.successors.iter() {
            if successor.predecessors_responsibility_fulfilled() {
                let successor_clone = successor.clone();
                ready_successors.push(successor_clone);
            }
        }
        (ready_successors, self.is_source())
    }
}


static THRP_INIT: Once = Once::new();


pub struct PipelineThreadOrchestrator {
    thread_adjacency_map: PipelineAdjacencyMap,
    thread_pool: StaticThreadPoolTopographical,
    num_nodes: usize
}
impl PipelineThreadOrchestrator {
    pub fn new(threads: Vec<Box<dyn CollectibleThread>>, num_threads: usize) -> Self {
        let num_nodes = threads.len();
        Self {
            thread_adjacency_map: PipelineAdjacencyMap::new(threads),
            thread_pool: StaticThreadPoolTopographical::new(num_threads),
            num_nodes
        }
    }

    pub fn start_pipeline(&mut self) {
        // start all sources
        let mut initial_submitter = self.thread_pool.acquire_submitter();
        for node in self.thread_adjacency_map.get_all_sources() {
            initial_submitter.submit(node);
        }
    }
    
    pub fn stop_pipeline(&mut self) {
        self.thread_pool.shutdown();
    }
    
    pub fn get_num_nodes(&self) -> usize {
        self.num_nodes
    }
}
