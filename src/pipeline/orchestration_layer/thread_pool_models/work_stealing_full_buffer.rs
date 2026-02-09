#![feature(mpmc_channel)]

use crate::pipeline::orchestration_layer::pipeline_graph::PipelineGraph;
use itertools::Itertools;
use rayon::{ThreadPool as RayonPool, ThreadPoolBuilder as RayonBuilder};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::{runtime::Builder as TokioBuilder, runtime::Runtime as TokioRuntime};

pub trait ThreadTaskTopographical {
    fn execute(&mut self) -> (Vec<Arc<dyn ThreadTaskTopographical>>, bool);
}

enum TaskType {
    Senders,
    Compute,
}
pub struct ThreadPoolTopographical {
    thread_pool: Arc<RayonPool>,
    async_runtime: Arc<TokioRuntime>,
    run_flag: Arc<AtomicBool>,
    graph: Arc<PipelineGraph>,
}
impl ThreadPoolTopographical {
    pub fn new(
        num_compute_thread: usize,
        num_async_thread: usize,
        graph: Arc<PipelineGraph>,
    ) -> Self {
        if num_compute_thread < 1 {
            // panic here
        }
        let run_flag = Arc::new(AtomicBool::new(true));

        Self {
            thread_pool: Arc::new(
                RayonBuilder::new()
                    .num_threads(num_compute_thread)
                    .build()
                    .unwrap(),
            ),
            async_runtime: Arc::new(
                TokioBuilder::new_multi_thread()
                    .worker_threads(num_async_thread)
                    .build()
                    .unwrap(),
            ),
            run_flag,
            graph,
        }
    }

    pub fn get_run_flag(&self) -> Arc<AtomicBool> {
        self.run_flag.clone()
    }

    pub fn task_submit(node_id: usize, thread_pool: Arc<Self>, task_type: TaskType) {
        let adjacency_node = thread_pool.graph.get_node(node_id).unwrap();

        match task_type {
            TaskType::Compute => {
                thread_pool
                    .thread_pool
                    .spawn(move || Self::thread_compute_task(thread_pool, node_id));
            }
            TaskType::Senders => {
                thread_pool
                    .async_runtime
                    .spawn(async move { Self::async_sender_task(thread_pool, node_id).await });
            }
        }
    }

    pub async fn async_sender_task(thread_pool: Arc<Self>, node_id: usize) {
        if !thread_pool
            .run_flag
            .load(std::sync::atomic::Ordering::Acquire)
        {
            return;
        }
        let adj_node = thread_pool.graph.get_node(node_id).unwrap();
        let compute_node = adj_node.get_thread_object().get_mut().unwrap();

        let mut increment_size;
        let mut edges_sent = compute_node
            .run_senders(&node_id, &mut increment_size)
            .await;

        // maybe this is a bad idea, one large operation, holds the await for too long. Profile later to identify if thats an issue
        // if it is, we extract senders and receivers up to the adjacency node level and do more direct computations
        for edge in adj_node.get_successors().iter() {
            if edges_sent.contains(&edge.get_destination_id()) {
                Self::task_submit(
                    edge.get_destination_id(),
                    thread_pool.clone(),
                    TaskType::Compute,
                );
                edge.increment_num_sends(increment_size as u64)
            }
        }

        if adj_node.is_source() || adj_node.predecessors_responsibility_fulfilled() {
            Self::task_submit(node_id, thread_pool, TaskType::Compute);
        }

        adj_node.exit_execution();
    }

    pub fn async_compute_task(node_id: usize, thread_pool: Arc<Self>) {
        // in the future this will be for gpu computations that are effectively IO bound from the CPu perspective
    }

    pub fn thread_compute_task(thread_pool: Arc<Self>, node_id: usize) {
        // need run flag check here
        if !thread_pool
            .run_flag
            .load(std::sync::atomic::Ordering::Acquire)
        {
            return;
        }
        let task = thread_pool.graph.get_node(node_id).unwrap();
        let mutable_task_object = task.get_thread_object().get_mut().unwrap(); // system guarantees that nothing else is trying to run this at the same time. No need to lock

        task.enter_execution();
        mutable_task_object.call_thread(&node_id);

        Self::task_submit(node_id, thread_pool, TaskType::Senders);
    }
}

pub struct ThreadPoolTopographicalHandle {
    run_flag: Arc<AtomicBool>,
    graph: Arc<PipelineGraph>,
    master_pool: Arc<ThreadPoolTopographical>,
}
impl ThreadPoolTopographicalHandle {
    pub fn new(
        run_flag: Arc<AtomicBool>,
        graph: Arc<PipelineGraph>,
        master_pool: Arc<ThreadPoolTopographical>,
    ) -> Self {
        Self {
            run_flag,
            graph,
            master_pool,
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
            ThreadPoolTopographical::task_submit(
                source,
                self.master_pool.clone(),
                TaskType::Compute,
            );
        }
        let initially_stateful = self.graph.get_all_initially_stateful();
        for node in initially_stateful {
            ThreadPoolTopographical::task_submit(
                node,
                self.master_pool.clone(),
                TaskType::Senders
            )
        }
    }
}

pub fn build_topographical_thread_pool(
    num_compute_thread: usize,
    num_async_thread: usize,
    graph: Arc<PipelineGraph>,
) -> ThreadPoolTopographicalHandle {
    let new_thread_pool: Arc<ThreadPoolTopographical> = Arc::new(ThreadPoolTopographical::new(
        num_compute_thread,
        num_async_thread,
        graph.clone(),
    ));
    let new_handle =
        ThreadPoolTopographicalHandle::new(new_thread_pool.get_run_flag(), graph, new_thread_pool);
    new_handle
}
