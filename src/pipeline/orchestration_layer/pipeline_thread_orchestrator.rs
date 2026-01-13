use crate::pipeline::threading_layer::pipeline_thread::CollectibleThread;
use async_channel::{bounded, Receiver, Sender};
use rayon;
use rayon_core;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, Once};
use crate::pipeline::orchestration_layer::pipeline_graph::{PipelineAdjacencyMap, PipelineAdjacencyNode};
// would all be more efficient and intuitive if there was an actual well defined
// conception of successor and predecessor relationships between threads/nodes
// this way minimal synchronization would be needed. That way, only when predecessors are unfulfilled is there a lockup further down the line

static THRP_INIT: Once = Once::new();
pub struct PipelineThreadOrchestratorBuilder {
    threads: Vec<Arc<Mutex<Option<Box<dyn CollectibleThread>>>>>,
}
impl PipelineThreadOrchestratorBuilder {
    pub fn new() -> Self {
        Self {
            threads: Vec::new(),
        }
    }
    pub fn add_thread(&mut self, thread: Box<dyn CollectibleThread>) {
        self.threads.push(Arc::new(Mutex::new(Some(thread))));
    }
    // pub fn complete(self) -> PipelineThreadOrchestrator {
    //     PipelineThreadOrchestrator::new(self.threads)
    // }
}

pub struct PipelineThreadOrchestrator {
    thread_adjacency_map: Arc<PipelineAdjacencyMap>,
    thread_pool: rayon::ThreadPool,
    max_executing: usize,
    current_executing: Arc<AtomicUsize>,
    current_head: usize,
}
impl PipelineThreadOrchestrator {
    pub fn new(
        threads: Vec<Box<dyn CollectibleThread>>,
        num_threads: usize,
        max_executing: usize,
    ) -> Self {
        THRP_INIT.call_once(|| {
            rayon::ThreadPoolBuilder::new()
                .num_threads(num_threads)
                .build_global()
                .unwrap();
        });

        let notification_channel: (Sender<()>, Receiver<()>) = bounded(1);
        Self {
            thread_adjacency_map: Arc::new(PipelineAdjacencyMap::new(threads)),
            thread_pool: rayon_core::ThreadPoolBuilder::new().build().unwrap(),
            max_executing,
            current_executing: Arc::new(AtomicUsize::new(0)),
            current_head: 0,
        }
    }
    
    fn dispatch_thread_to_pool(&mut self, thread_id: &String) {
        let adj_map_clone = self.thread_adjacency_map.clone();
        self.thread_pool.spawn(move ||
            {
                let adjacency_node: &PipelineAdjacencyNode = *adj_map_clone[thread_id];
                *adjacency_node.thread_object.lock().unwrap().call_thread();
                
                *adjacency_node.num_executions_since_completion += 1;
                
                for successor_id in *adjacency_node.successors {
                    if adj_map_clone.check_dependencies_satisfied(successor_id) {
                        rayon::scope(|s| s.spawn(|| self.dispatch_thread_to_pool(successor_id)));
                    }
                }
                if adjacency_node.is_source() {
                    rayon::scope(|s| s.spawn(|| self.dispatch_thread_to_pool(thread_id)));
                }
            }
        )
    }
    
    pub fn start_pipeline(&mut self) { // start all sources
        for id in *self.thread_adjacency_map.get_all_sources() {
            self.dispatch_thread_to_pool(id);
        }
    }
    
    pub fn 
}
