use log::info;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::thread::{JoinHandle, Thread, spawn as thread_spawn};
use tokio::sync::oneshot::{Sender, Receiver, channel};
use tokio::{runtime as TokioRuntime, select};
use tokio::sync::watch::{Receiver as WatchReceiver, Sender as WatchSender, channel as watch_channel};
use crate::pipeline::construction_layer::node_types::node_traits::{CollectibleNode, RunModel};
use crate::pipeline::orchestration_layer::pipeline_graph::PipelineGraph;


#[derive(PartialEq, Clone, Copy)]
enum ThreadOrder {
    Run,
    Stop,
    Kill
}


pub struct ThreadPoolPerNode {
    thread_pool: Vec<JoinHandle<()>>,
    run_watcher: WatchSender<ThreadOrder>,
    graph: Arc<PipelineGraph>,
}
impl ThreadPoolPerNode {
    pub fn new(graph: Arc<PipelineGraph>) -> Self {
        info!("Creating ThreadPoolPerNode");

        let mut mutable_states = graph.get_all_mutable_state();
        let mut thread_handles = Vec::with_capacity(mutable_states.len());
        let runflag = Arc::new(AtomicBool::new(false));
        let (run_sender, run_receiver) = watch_channel(ThreadOrder::Run);

        for (id, mutable_state) in mutable_states.into_iter().enumerate() {
            let receiver_clone = run_receiver.clone();
            thread_handles.push(thread_spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap();

                rt.block_on(
                    async move {
                        Self::mutable_state_loop(mutable_state, id, receiver_clone).await;
                    }
                )
            }))
        };

        Self {
            thread_pool: thread_handles,
            run_watcher: run_sender,
            graph,
        }
    }

    async fn mutable_state_loop(mut mutable_state: Box<dyn CollectibleNode>, id: usize, mut run_watcher: WatchReceiver<ThreadOrder>) {
        let mut internal_state = ThreadOrder::Stop;

        while internal_state != ThreadOrder::Kill {
            match mutable_state.get_run_model() {
                RunModel::CPU => {
                    let id_clone = id.clone();
                    mutable_state = tokio::task::spawn_blocking(move || {
                        mutable_state.call_thread_cpu(id_clone);
                        mutable_state
                    }).await.unwrap();
                }
                RunModel::IO => {
                    select! {
                        _ = mutable_state.call_thread_io(id) => {}
                        new_internal_state = run_watcher.changed() => {
                            internal_state = *run_watcher.borrow();
                        }
                    }
                }
                RunModel::Communicator => {
                    select! {
                        _ = mutable_state.run_senders(id) => {}
                        new_internal_state = run_watcher.changed() => {
                            internal_state = *run_watcher.borrow();
                        }
                    }
                }
            }
        }
    }

    fn start_threads(&self) {
        self.run_watcher.send(ThreadOrder::Run).unwrap();
    }

    fn stop_threads(&self) {
        self.run_watcher.send(ThreadOrder::Stop).unwrap();
    }

    fn kill_threads(mut self) {
        self.run_watcher.send(ThreadOrder::Kill).unwrap();

        for thread in self.thread_pool {
            let _ = thread.join();
        }
    }
}

/// Public handle used by integration tests / callers (mirrors the work-stealing handle API).
pub struct ThreadPoolPerNodeHandle {
    pool: Option<ThreadPoolPerNode>,
}
impl ThreadPoolPerNodeHandle {
    pub fn new(graph: Arc<PipelineGraph>) -> Self {
        Self {
            pool: Some(ThreadPoolPerNode::new(graph)),
        }
    }

    pub fn start(&mut self, _async_runtime: &tokio::runtime::Runtime) {
        if let Some(pool) = self.pool.as_ref() {
            pool.start_threads();
        }
    }

    pub fn stop(&mut self) {
        if let Some(pool) = self.pool.as_ref() {
            pool.stop_threads();
        }
    }

    pub fn kill(mut self) {
        if let Some(pool) = self.pool.take() {
            pool.kill_threads();
        }
    }
}

pub fn build_per_node_thread_pool(graph: Arc<PipelineGraph>) -> ThreadPoolPerNodeHandle {
    ThreadPoolPerNodeHandle::new(graph)
}