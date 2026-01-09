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
    global_finish_condition: (Sender<()>, Receiver<()>),
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
            global_finish_condition: notification_channel,
        }
    }
    
    fn dispatch_thread_to_pool(&mut self, thread_id: &String) {
        let adj_map_clone = self.thread_adjacency_map.clone();
        self.thread_pool.spawn(move ||
            {
                let adjacency_node: &PipelineAdjacencyNode = *adj_map_clone[thread_id];
                *adjacency_node.thread_object.lock().unwrap().call_thread();
                
                *adjacency_node.num_executions_since_completion += 1
                
                if adj_map_clone.check_dependencies_satisfied(adjacency_node.)
            }
        )
    }
    
    pub async fn start_pipeline(&mut self) {
        // you need to run only the first node
        // rules for pipeline: 
        //      No source, submit itself and successor(s) to thread pool. use atomics
        //      otherwise, just check successor for completion and submit it to thread pool
        //      
    }
}
