#![feature(mpmc_channel)]

use crate::pipeline::construction_layer::node_types::node_traits::{CollectibleNode, RunModel};
use crate::pipeline::orchestration_layer::pipeline_graph::PipelineGraph;
use itertools::Itertools;
use rayon::{ThreadPool as RayonPool, ThreadPoolBuilder as RayonBuilder};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::{runtime::Builder as TokioBuilder, runtime::Runtime as TokioRuntime};

pub trait ThreadTaskTopographical {
    fn execute(&mut self) -> (Vec<Arc<dyn ThreadTaskTopographical>>, bool);
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

    pub fn is_running(&self) -> bool {
        self.run_flag.load(std::sync::atomic::Ordering::Acquire)
    }

    fn task_execute(thread_pool: Arc<Self>, node_id: usize) {
        let (id, mut node) = match thread_pool
            .graph
            .get_node(node_id) {
                Some(node) => node,
                None => return // if this is the case, the node must already be running. hence it cannot be executed
        };
        Self::task_execute_direct(thread_pool, id, node);
    }
    fn task_execute_direct(thread_pool: Arc<Self>, id: usize, node: Box<dyn CollectibleNode>) {
        if !thread_pool.is_running()
        {
            thread_pool.graph.place_node(id, node).unwrap_or_else(|| {
                panic!("Node not found in graph (should never happen)")
            });
            return
        }
        
        let thread_pool_clone = thread_pool.clone();
        let run_model = node.get_run_model();

        match run_model {
            RunModel::CPU => {
                if !node.is_ready_exec() {
                    thread_pool.graph.place_node(id, node).unwrap_or_else(|| panic!("Node not found in graph (should be impossible)"));
                    return;
                }
                thread_pool.thread_pool.spawn(move || {
                    Self::thread_compute_task(thread_pool_clone, id, node);
                });
            }
            RunModel::IO => {
                if !node.is_ready_exec() {
                    thread_pool.graph.place_node(id, node).unwrap_or_else(|| panic!("Node not found in graph (should be impossible)"));
                    return;
                }
                thread_pool.async_runtime.spawn(async move {
                    Self::async_compute_task(thread_pool_clone, id, node).await;
                });
            }
            RunModel::Communicator => {
                thread_pool
                    .async_runtime
                    .spawn(async move { Self::async_sender_task(thread_pool_clone, id, node).await });
            }
        }
    }

    async fn async_sender_task(thread_pool: Arc<Self>, id: usize, mut node: Box<dyn CollectibleNode>) {
        let satiated_channels = node.run_senders(id).await;
        for successor_id in satiated_channels {
            Self::task_execute(thread_pool.clone(), successor_id);
        }

        Self::task_execute_direct(thread_pool, id, node); 
    }

    async fn async_compute_task(thread_pool: Arc<Self>, id: usize, mut node: Box<dyn CollectibleNode>) {
        let start = std::time::Instant::now();
        node.call_thread_io(id).await;
        thread_pool.graph.update_analytics(id, start.elapsed().as_nanos() as u64);
        Self::task_execute(thread_pool, id);
    }

    fn thread_compute_task(thread_pool: Arc<Self>, id: usize, mut node: Box<dyn CollectibleNode>) {
        let start = std::time::Instant::now();
        node.call_thread_cpu(id);
        thread_pool.graph.update_analytics(id, start.elapsed().as_nanos() as u64);
        Self::task_execute(thread_pool, id);
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
            ThreadPoolTopographical::task_submit(node, self.master_pool.clone(), TaskType::Senders)
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
