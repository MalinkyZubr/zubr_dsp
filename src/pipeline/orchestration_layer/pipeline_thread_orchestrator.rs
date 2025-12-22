use crate::pipeline::threading_layer::pipeline_thread::CollectibleThread;
use rayon;
use rayon_core;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, Once};


static THRP_INIT: Once = Once::new();
pub struct PipelineThreadOrchestratorBuilder {
    threads: VecDeque<Arc<Mutex<Box<dyn CollectibleThread>>>>,
}
impl PipelineThreadOrchestratorBuilder {
    pub fn new() -> Self {
        Self {
            threads: VecDeque::new(),
        }
    }
    pub fn add_thread(&mut self, thread: Box<dyn CollectibleThread>) {
        self.threads.push_back(Arc::new(Mutex::new(thread)));
    }
    pub fn complete(self) -> PipelineThreadOrchestrator {
        PipelineThreadOrchestrator::new(self.threads)
    }
}

pub struct PipelineThreadOrchestrator {
    threads: VecDeque<Arc<Mutex<Box<dyn CollectibleThread>>>>,
    thread_pool: rayon::ThreadPool,
}
impl PipelineThreadOrchestrator {
    pub fn new(threads: VecDeque<Arc<Mutex<Box<dyn CollectibleThread>>>>) -> Self {
        let num_threads = threads.len();
        THRP_INIT.call_once(|| {
            rayon::ThreadPoolBuilder::new()
                .num_threads(num_threads)
                .build_global()
                .unwrap();
        });

        Self {
            threads: VecDeque::new(),
            thread_pool: rayon_core::ThreadPoolBuilder::new().build().unwrap(),
        }
    }

    pub fn run_single_tick(&mut self) {
        for _ in 0..self.threads.len() {
            let mut thread = self.threads.pop_front().unwrap();
            let mut thread_to_pass = thread.clone();

            //self.thread_pool.
            self.thread_pool.spawn(move || {
                thread_to_pass.lock().unwrap().call_thread(); // what if I fill up? that would be bad
            });

            self.threads.push_back(thread);
        }
    }
}
