use crate::pipeline::threading_layer::pipeline_thread::CollectibleThread;
use rayon;
use rayon_core;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, Once};
use std::sync::atomic::{AtomicUsize, Ordering};

static THRP_INIT: Once = Once::new();
pub struct PipelineThreadOrchestratorBuilder {
    threads: Vec<Arc<Mutex<Box<dyn CollectibleThread>>>>,
}
impl PipelineThreadOrchestratorBuilder {
    pub fn new() -> Self {
        Self {
            threads: Vec::new(),
        }
    }
    pub fn add_thread(&mut self, thread: Box<dyn CollectibleThread>) {
        self.threads.push(Arc::new(Mutex::new(thread)));
    }
    pub fn complete(self) -> PipelineThreadOrchestrator {
        PipelineThreadOrchestrator::new(self.threads)
    }
}

pub struct PipelineThreadOrchestrator {
    thread_population: Vec<Arc<Mutex<Box<dyn CollectibleThread>>>>,
    thread_pool: rayon::ThreadPool,
    max_executing: usize,
    current_executing: Arc<AtomicUsize>,
    current_head: usize
}
impl PipelineThreadOrchestrator {
    pub fn new(threads: Vec<Arc<Mutex<Box<dyn CollectibleThread>>>>, num_threads: usize, max_executing: usize) -> Self {
        THRP_INIT.call_once(|| {
            rayon::ThreadPoolBuilder::new()
                .num_threads(num_threads)
                .build_global()
                .unwrap();
        });

        Self {
            thread_population: threads,
            thread_pool: rayon_core::ThreadPoolBuilder::new().build().unwrap(),
            max_executing,
            current_executing: Arc::new(AtomicUsize::new(0)),
            current_head: 0
        }
    }

    pub async fn run_single_tick(&mut self) {
        // pop from async queue, await until a step is available to pop
        if self.current_executing.load(Ordering::Relaxed) == self.max_executing {
            // await the global complete async condition
        }
        
        let current_executing_clone: Arc<AtomicUsize> = self.current_executing.clone();
        // must pass the actual instance of the thread object to the thread pool, because there is persistent internal state for threads
        // 
        self.thread_pool.spawn(move || { // need a system of async counter to say how many guys have been distributed into the thread pool
            thread_to_pass.lock().unwrap().call_thread(); // what if I fill up? that would be bad
            current_executing_clone.fetch_sub(1, Ordering::SeqCst);
        });
        
        self.current_executing.fetch_add(1, Ordering::SeqCst);

        self.threads.push_back(thread);
    }
}
