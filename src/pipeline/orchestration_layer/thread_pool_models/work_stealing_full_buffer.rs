#![feature(mpmc_channel)]

use std::sync::atomic::AtomicBool;
use rayon::{ThreadPoolBuilder, ThreadPool};
use std::sync::Arc;
use tokio::{task, runtime};
use crate::pipeline::orchestration_layer::pipeline_graph::{PipelineGraph};

pub trait ThreadTaskTopographical {
    fn execute(&mut self) -> (Vec<Arc<dyn ThreadTaskTopographical>>, bool);
}

pub struct ThreadPoolTopographical {
    num_threads: usize,
    thread_pool: Arc<ThreadPool>,
    async_runtime: Arc<runtime::Runtime>,
    run_flag: Arc<AtomicBool>,
    graph: Arc<PipelineGraph>,
}
impl ThreadPoolTopographical {
    pub fn new(num_thread: usize) -> Self {
        if num_thread < 1 {
            // panic here
        }
        let run_flag = Arc::new(AtomicBool::new(true));

        Self {
            num_threads: num_thread,
            thread_pool: Arc::new(
                ThreadPoolBuilder::new()
                    .num_threads(num_thread)
                    .build()
                    .unwrap(),
            ),
            run_flag,
            graph: Arc::new(graph),
        }
    }

    pub fn stop(&mut self) {
        self.run_flag
            .store(false, std::sync::atomic::Ordering::Release);
    }

    pub fn start(&mut self) {
        self.run_flag
            .store(true, std::sync::atomic::Ordering::Release);

        let sources = self.graph.get_all_sources();
        for source in sources {
            let task_list = self.graph.clone();
            let pool = self.thread_pool.clone();
            let run_flag = self.run_flag.clone();
            pool.spawn(move || Self::thread_task(pool, source, task_list, run_flag));
        }
    }

    pub fn task_submit_successors(
        pool: Arc<ThreadPool>,
        edge: &PipelineAdjacencyEdge,
        task_list: Arc<PipelineGraph>,
        run_flag: Arc<AtomicBool>,
    ) {
        edge.num_executions_since_completion
            .fetch_add(1, std::sync::atomic::Ordering::Release);

        let successor_node = task_list.get_node(edge.destination_id);

        if !successor_node.is_running() && successor_node.predecessors_responsibility_fulfilled()
        // && is state okay
        {
            pool.spawn(move || {
                Self::thread_task(pool, edge.destination_id, task_list, run_flag)
            });
        }
    }

    pub fn thread_task(
        pool: Arc<ThreadPool>,
        task_id: usize,
        task_list: Arc<PipelineGraph>,
        run_flag: Arc<AtomicBool>,
    ) {
        // need run flag check here
        let task = task_list.get_node(task_id);
        let mutable_task_object = task.get_thread_object().get_mut().unwrap(); // system guarantees that nothing else is trying to run this at the same time. No need to lock
        mutable_task_object.call_thread();

        if !run_flag.load(std::sync::atomic::Ordering::Acquire) {
            return;
        }

        for edge in task.get_successors().iter() {
            let task_list_clone = task_list.clone();
            let pool_clone = pool.clone();
            let run_flag_clone = run_flag.clone();
            Self::task_submit_successors(
                pool_clone,
                edge,
                task_list_clone,
                run_flag_clone,
            );
        }

        if task.is_source() || task.predecessors_responsibility_fulfilled() {
            pool.spawn(move || Self::thread_task(pool, task_id, task_list, run_flag));
        }
    }
}
