#![feature(mpmc_channel)]

use crate::pipeline::orchestration_layer::pipeline_graph::{PipelineAdjacencyMap, PipelineAdjacencyNode};
use crate::pipeline::threading_layer::pipeline_thread::CollectibleThread;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicUsize;
use std::sync::mpmc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex, Once};
use std::thread;
// would all be more efficient and intuitive if there was an actual well defined
// conception of successor and predecessor relationships between threads/nodes
// this way minimal synchronization would be needed. That way, only when predecessors are unfulfilled is there a lockup further down the line


pub struct StaticThreadPoolSubmitter {
    sender: Sender<Box<dyn Fn(String)>>,
}
impl StaticThreadPoolSubmitter {
    pub fn submit(&self, function: Box<dyn Fn()>) {
        self.sender.send(function).unwrap();
    }
}


pub struct StaticThreadPool {
    num_threads: usize,
    threads: Vec<thread::JoinHandle<()>>,
    sender: Sender<Box<dyn Fn(String) -> Vec<Box<dyn Fn(String) + Send>> + Send>>,
    runflag: Arc<AtomicBool>,
}
impl StaticThreadPool {
    pub fn new(num_thread: usize) -> Self {
        assert!(num_thread > 0);
        let runflag = Arc::new(AtomicBool::new(true));
        let (sender, receiver) = channel::<Box<dyn Fn()>>();
        let mut threads = Vec::with_capacity(num_thread);

        for _ in 0..num_thread {
            let runflag_clone = runflag.clone();
            let receiver_clone = receiver.clone();
            let sender_clone = sender.clone();
            threads.push(
                thread::spawn(move || {
                    while runflag_clone.load(std::sync::atomic::Ordering::Acquire) {
                        let received_function: Box<dyn Fn() -> Vec<Box<dyn Fn() + Send>> + Send> = receiver_clone.recv().unwrap();
                        let successor_functions: Vec<Box<dyn Fn()>> = received_function();

                        for successor_function in successor_functions {
                            sender_clone.send(successor_function).unwrap();
                        }
                    }
                })
            )
        }
        Self {
            num_threads: num_thread,
            threads,
            runflag,
            sender,
        }
    }

    pub fn acquire_submitter(&self) -> StaticThreadPoolSubmitter {
        StaticThreadPoolSubmitter {
            sender: self.sender.clone()
        }
    }

    pub fn shutdown(&mut self) {
        self.runflag.store(false, std::sync::atomic::Ordering::Release);
        for thread in self.threads.drain(..) {
            thread.join().unwrap();
        }
    }
}

fn node_thread_function(adj_map: Arc<PipelineAdjacencyMap>) -> Vec<Box<dyn Fn() + Send>> {
    let adjacency_node: &PipelineAdjacencyNode = *adj_map[thread_id];
    *adjacency_node.thread_object.lock().unwrap().call_thread();

    *adjacency_node.num_executions_since_completion += 1;

    let mut successors = Vec::new();

    for successor_id in *adjacency_node.successors {
        if adj_map.check_dependencies_satisfied(successor_id) {
            successors.push(*adj_map.adjac[successor_id])
        }
    }
    if adjacency_node.is_source() {
        successors.push
    }
}


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
    thread_pool: StaticThreadPool,
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
        let submitter = self.thread_pool.acquire_submitter();

        self.thread_pool.spawn(move ||
            {
                let adjacency_node: &PipelineAdjacencyNode = *adj_map_clone[thread_id];
                *adjacency_node.thread_object.lock().unwrap().call_thread();

                *adjacency_node.num_executions_since_completion += 1;

                let successors = Vec::new()

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
}
