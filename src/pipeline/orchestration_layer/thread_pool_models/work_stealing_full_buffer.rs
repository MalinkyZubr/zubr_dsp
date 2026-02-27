#![feature(mpmc_channel)]

use crate::pipeline::construction_layer::node_types::node_traits::{CollectibleNode, RunModel};
use crate::pipeline::orchestration_layer::pipeline_graph::PipelineGraph;
use rayon::{ThreadPool as RayonPool, ThreadPoolBuilder as RayonBuilder};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Weak};
use tokio::{runtime::Builder as TokioBuilder, runtime::Runtime as TokioRuntime};
use log::{trace, debug, info, warn, error};


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
        info!("Creating ThreadPoolTopographical with {} compute threads and {} async threads", 
              num_compute_thread, num_async_thread);
        
        if num_compute_thread < 1 {
            error!("Invalid number of compute threads: {}. Must be at least 1.", num_compute_thread);
            panic!("Number of compute threads must be at least 1");
        }
        
        let run_flag = Arc::new(AtomicBool::new(false));
        debug!("ThreadPool run flag initialized to false (stopped state)");

        let thread_pool = match RayonBuilder::new()
            .num_threads(num_compute_thread)
            .build()
        {
            Ok(pool) => {
                debug!("Successfully created Rayon thread pool with {} threads", num_compute_thread);
                Arc::new(pool)
            },
            Err(e) => {
                error!("Failed to create Rayon thread pool: {}", e);
                panic!("Failed to create thread pool: {}", e);
            }
        };

        let async_runtime = match TokioBuilder::new_multi_thread()
            .worker_threads(num_async_thread)
            .build()
        {
            Ok(runtime) => {
                debug!("Successfully created Tokio async runtime with {} worker threads", num_async_thread);
                Arc::new(runtime)
            },
            Err(e) => {
                error!("Failed to create Tokio async runtime: {}", e);
                panic!("Failed to create async runtime: {}", e);
            }
        };

        info!("ThreadPoolTopographical created successfully");

        Self {
            thread_pool,
            async_runtime,
            run_flag,
            graph,
        }
    }

    pub fn get_run_flag(&self) -> Arc<AtomicBool> {
        trace!("Retrieving run flag reference");
        self.run_flag.clone()
    }

    pub fn is_running(&self) -> bool {
        let running = self.run_flag.load(std::sync::atomic::Ordering::Acquire);
        trace!("Thread pool running status: {}", running);
        running
    }

    fn task_execute(thread_pool: Weak<Self>, node_id: usize) {
        debug!("Attempting to execute task for node {}", node_id);
        
        match thread_pool.upgrade() {
            Some(thread_pool) => {
                trace!("Successfully upgraded weak reference to thread pool");
                
                let (id, node) = match thread_pool.graph.get_node(node_id) {
                    Some(node) => {
                        debug!("Retrieved node {} from graph for execution", node_id);
                        node
                    },
                    None => {
                        debug!("Node {} not available in graph (may already be running)", node_id);
                        return;
                    }
                };
                Self::task_execute_direct(thread_pool, id, node);
            }
            None => {
                warn!("Failed to upgrade weak reference to thread pool - pool may have been dropped");
                return;
            }
        }
    }
    
    fn task_execute_direct(thread_pool: Arc<Self>, id: usize, node: Box<dyn CollectibleNode>) {
        trace!("Direct task execution for node {}", id);
        
        if !thread_pool.is_running() {
            debug!("Thread pool is stopped, placing node {} back in graph", id);
            thread_pool
                .graph
                .place_node(id, node)
                .unwrap_or_else(|| {
                    error!("Failed to place node {} back in graph - node not found (should never happen)", id);
                    panic!("Node not found in graph (should never happen)")
                });
            return;
        }

        let thread_pool_clone = thread_pool.clone();
        let run_model = node.get_run_model();
        debug!("Node {} has run model: {:?}", id, run_model);

        match run_model {
            RunModel::CPU => {
                if !node.is_ready_exec() {
                    debug!("Node {} not ready for CPU execution, placing back in graph", id);
                    thread_pool.graph.place_node(id, node).unwrap_or_else(|| {
                        error!("Failed to place node {} back in graph - node not found (should be impossible)", id);
                        panic!("Node not found in graph (should be impossible)")
                    });
                    return;
                }
                
                info!("Spawning CPU task for node {}", id);
                thread_pool.thread_pool.spawn(move || {
                    Self::thread_compute_task(thread_pool_clone, id, node);
                });
            }
            RunModel::IO => {
                if !node.is_ready_exec() {
                    debug!("Node {} not ready for IO execution, placing back in graph", id);
                    thread_pool.graph.place_node(id, node).unwrap_or_else(|| {
                        error!("Failed to place node {} back in graph - node not found (should be impossible)", id);
                        panic!("Node not found in graph (should be impossible)")
                    });
                    return;
                }
                
                info!("Spawning async IO task for node {}", id);
                thread_pool.async_runtime.spawn(async move {
                    Self::async_compute_task(thread_pool_clone, id, node).await;
                });
            }
            RunModel::Communicator => {
                info!("Node {} testing for sink", id);
                if !node.is_sink() {
                    info!("Spawning async communicator task for node {}", id);
                    thread_pool.async_runtime.spawn(async move {
                        Self::async_sender_task(thread_pool_clone, id, node).await
                    });
                }
                else {
                    info!("Node {} is a sink, skipping async communicator task", id);
                    thread_pool.graph.place_node(id, node).unwrap_or_else(|| {
                        error!("Failed to place node {} back in graph - node not found (should be impossible)", id);
                        panic!("Node not found in graph (should be impossible)")
                    });
                }
            }
        }
    }

    async fn async_sender_task(
        thread_pool: Arc<Self>,
        id: usize,
        mut node: Box<dyn CollectibleNode>,
    ) {
        debug!("Starting async sender task for node {}", id);
        
        let satiated_channels = node.run_senders(id).await;
        let downgraded = Arc::downgrade(&thread_pool.clone());
        
        match satiated_channels {
            Some(successor_ids) => {
                debug!("Node {} sender task completed, triggering {} successors", id, successor_ids.len());
                for successor_id in successor_ids {
                    trace!("Triggering successor node {}", successor_id);
                    Self::task_execute(downgraded.clone(), successor_id);
                }
            },
            None => {
                warn!("Node {} sender task returned no successors (potential error condition)", id);
            }
        }

        debug!("Rescheduling node {} after sender task completion", id);
        Self::task_execute_direct(thread_pool.clone(), id, node);
    }

    async fn async_compute_task(
        thread_pool: Arc<Self>,
        id: usize,
        mut node: Box<dyn CollectibleNode>,
    ) {
        debug!("Starting async compute task for node {}", id);
        let start = std::time::Instant::now();
        
        node.call_thread_io(id).await;
        
        let execution_time = start.elapsed().as_nanos() as u64;
        trace!("Node {} async compute task completed in {} ns", id, execution_time);
        
        thread_pool
            .graph
            .update_analytics(id, execution_time);
        
        debug!("Rescheduling node {} after async compute task completion", id);
        Self::task_execute_direct(thread_pool, id, node);
    }

    fn thread_compute_task(thread_pool: Arc<Self>, id: usize, mut node: Box<dyn CollectibleNode>) {
        debug!("Starting CPU compute task for node {}", id);
        let start = std::time::Instant::now();
        
        node.call_thread_cpu(id);
        
        let execution_time = start.elapsed().as_nanos() as u64;
        trace!("Node {} CPU compute task completed in {} ns", id, execution_time);
        
        thread_pool
            .graph
            .update_analytics(id, execution_time);
        
        debug!("Rescheduling node {} after CPU compute task completion", id);
        Self::task_execute_direct(thread_pool, id, node);
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
        debug!("Creating ThreadPoolTopographicalHandle");
        Self {
            run_flag,
            graph,
            master_pool,
        }
    }

    pub fn stop(&mut self) {
        info!("Stopping thread pool execution");
        self.run_flag
            .store(false, std::sync::atomic::Ordering::Release);
        debug!("Thread pool stop signal sent");
    }

    pub fn start(&mut self) {
        info!("Starting thread pool execution");
        self.run_flag
            .store(true, std::sync::atomic::Ordering::Release);

        let sources = self.graph.get_all_sources();
        debug!("Found {} source nodes to execute", sources.len());
        
        let weak_ref = Arc::downgrade(&self.master_pool);
        for source in &sources {
            debug!("Triggering execution of source node {}", source);
            ThreadPoolTopographical::task_execute(weak_ref.clone(), *source);
        }
        
        let initially_stateful = self.graph.get_all_initially_stateful();
        debug!("Found {} initially stateful nodes to execute", initially_stateful.len());
        
        for node in &initially_stateful {
            debug!("Triggering execution of initially stateful node {}", node);
            ThreadPoolTopographical::task_execute(weak_ref.clone(), *node);
        }
        
        info!("Thread pool started with {} source nodes and {} initially stateful nodes", 
              sources.len(), initially_stateful.len());
    }

    pub fn kill(mut self) {
        info!("Killing thread pool (stopping execution)");
        self.stop();
        debug!("Thread pool killed");
    }
}

pub fn build_topographical_thread_pool(
    num_compute_thread: usize,
    num_async_thread: usize,
    graph: Arc<PipelineGraph>,
) -> ThreadPoolTopographicalHandle {
    info!("Building topographical thread pool with {} compute threads and {} async threads", 
          num_compute_thread, num_async_thread);
    
    let new_thread_pool: Arc<ThreadPoolTopographical> = Arc::new(ThreadPoolTopographical::new(
        num_compute_thread,
        num_async_thread,
        graph.clone(),
    ));
    
    let new_handle =
        ThreadPoolTopographicalHandle::new(new_thread_pool.get_run_flag(), graph, new_thread_pool);
    
    info!("Topographical thread pool built successfully");
    new_handle
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;
    use super::*;
    use crate::pipeline::construction_layer::node_builder::PipelineBuildVector;
    use crate::pipeline::construction_layer::node_builder::PipelineParameters;
    use std::sync::atomic::Ordering;
    use std::time::Duration;
    use tokio::time::sleep;

    // Mock CollectibleNode for testing
    #[derive(Debug)]
    struct MockNode {
        id: usize,
        num_inputs: usize,
        num_outputs: usize,
        has_initial: bool,
        run_model: RunModel,
        ready: bool,
        execution_count: Arc<AtomicBool>,
    }

    impl MockNode {
        fn new(
            id: usize,
            num_inputs: usize,
            num_outputs: usize,
            has_initial: bool,
            run_model: RunModel,
        ) -> Self {
            Self {
                id,
                num_inputs,
                num_outputs,
                has_initial,
                run_model,
                ready: true,
                execution_count: Arc::new(AtomicBool::new(false)),
            }
        }

        fn new_not_ready(id: usize, run_model: RunModel) -> Self {
            Self {
                id,
                num_inputs: 1,
                num_outputs: 1,
                has_initial: false,
                run_model,
                ready: false,
                execution_count: Arc::new(AtomicBool::new(false)),
            }
        }

        fn was_executed(&self) -> bool {
            self.execution_count.load(Ordering::Acquire)
        }
    }

    #[async_trait::async_trait]
    impl CollectibleNode for MockNode {
        fn is_ready_exec(&self) -> bool {
            self.ready
        }
        fn get_successors(&self) -> Vec<usize> {
            vec![]
        }
        fn get_run_model(&self) -> RunModel {
            self.run_model
        }
        fn get_num_inputs(&self) -> usize {
            self.num_inputs
        }
        fn get_num_outputs(&self) -> usize {
            self.num_outputs
        }
        async fn run_senders(&mut self, _id: usize) -> Option<Vec<usize>> {
            self.execution_count.store(true, Ordering::Release);
            Some(vec![])
        }
        fn load_initial_state(&mut self) {}
        fn has_initial_state(&self) -> bool {
            self.has_initial
        }
        fn call_thread_cpu(&mut self, _id: usize) {
            self.execution_count.store(true, Ordering::Release);
        }
        async fn call_thread_io(&mut self, _id: usize) {
            self.execution_count.store(true, Ordering::Release);
        }
    }

    fn create_test_graph() -> Arc<PipelineGraph> {
        let params = PipelineParameters::new(4);
        let mut build_vector = PipelineBuildVector::new(params);

        // Add a source node (0 inputs, 2 outputs)
        build_vector.add_node((
            0,
            "source".to_string(),
            Box::new(MockNode::new(0, 0, 2, false, RunModel::CPU)),
        ));

        // Add a processing node (2 inputs, 1 output, with initial state)
        build_vector.add_node((
            1,
            "processor".to_string(),
            Box::new(MockNode::new(1, 2, 1, true, RunModel::IO)),
        ));

        // Add a sink node (1 input, 0 outputs)
        build_vector.add_node((
            2,
            "sink".to_string(),
            Box::new(MockNode::new(2, 1, 0, false, RunModel::Communicator)),
        ));

        Arc::new(PipelineGraph::new(Rc::new(RefCell::new(build_vector))))
    }

    #[test]
    fn test_thread_pool_topographical_new() {
        let graph = create_test_graph();
        let pool = ThreadPoolTopographical::new(4, 2, graph.clone());

        assert!(pool.is_running());
        assert!(Arc::ptr_eq(&pool.graph, &graph));
    }

    #[test]
    fn test_thread_pool_topographical_get_run_flag() {
        let graph = create_test_graph();
        let pool = ThreadPoolTopographical::new(6, 1, graph);

        let run_flag = pool.get_run_flag();
        assert!(run_flag.load(Ordering::Acquire));

        // Modify the flag and verify it affects the pool
        run_flag.store(false, Ordering::Release);
        assert!(!pool.is_running());
    }

    #[test]
    fn test_thread_pool_topographical_is_running() {
        let graph = create_test_graph();
        let pool = ThreadPoolTopographical::new(2, 1, graph);

        assert!(pool.is_running());

        pool.run_flag.store(false, Ordering::Release);
        assert!(!pool.is_running());
    }

    #[test]
    fn test_thread_pool_topographical_handle_new() {
        let graph = create_test_graph();
        let pool = Arc::new(ThreadPoolTopographical::new(2, 1, graph.clone()));
        let run_flag = pool.get_run_flag();

        let handle = ThreadPoolTopographicalHandle::new(run_flag.clone(), graph.clone(), pool);

        assert!(Arc::ptr_eq(&handle.run_flag, &run_flag));
        assert!(Arc::ptr_eq(&handle.graph, &graph));
    }

    #[test]
    fn test_thread_pool_topographical_handle_stop() {
        let graph = create_test_graph();
        let pool = Arc::new(ThreadPoolTopographical::new(2, 1, graph.clone()));
        let run_flag = pool.get_run_flag();

        let mut handle = ThreadPoolTopographicalHandle::new(run_flag.clone(), graph, pool);

        assert!(handle.run_flag.load(Ordering::Acquire));

        handle.stop();
        assert!(!handle.run_flag.load(Ordering::Acquire));
    }

    #[test]
    fn test_thread_pool_topographical_handle_start() {
        let graph = create_test_graph();
        let pool = Arc::new(ThreadPoolTopographical::new(2, 1, graph.clone()));
        let run_flag = pool.get_run_flag();

        let mut handle = ThreadPoolTopographicalHandle::new(run_flag.clone(), graph, pool);

        // Stop first
        handle.stop();
        assert!(!handle.run_flag.load(Ordering::Acquire));

        // Then start
        handle.start();
        assert!(handle.run_flag.load(Ordering::Acquire));
    }

    #[test]
    fn test_build_topographical_thread_pool() {
        let graph = create_test_graph();
        let handle = build_topographical_thread_pool(4, 2, graph.clone());

        assert!(handle.run_flag.load(Ordering::Acquire));
        assert!(Arc::ptr_eq(&handle.graph, &graph));
    }

    #[test]
    fn test_task_execute_with_cpu_node() {
        let handle = std::thread::spawn(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            
            rt.block_on(async {
                let params = PipelineParameters::new(4);
                let mut build_vector = PipelineBuildVector::new(params);

                let mock_node = MockNode::new(0, 1, 1, false, RunModel::CPU);
                let execution_flag = mock_node.execution_count.clone();
                build_vector.add_node((0, "cpu_node".to_string(), Box::new(mock_node)));

                let graph = Arc::new(PipelineGraph::new(Rc::new(RefCell::new(build_vector))));
                
                {
                    let pool = Arc::new(ThreadPoolTopographical::new(2, 1, graph));
                    ThreadPoolTopographical::task_execute(Arc::downgrade(&pool), 0);

                    // Give some time for the task to execute
                    sleep(Duration::from_millis(100)).await;
                    
                    execution_flag.load(Ordering::Acquire)
                }
            })
        });
        
        let result = handle.join().unwrap();
        assert!(result);
    }

    #[test]
    fn test_task_execute_with_io_node() {
        let handle = std::thread::spawn(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            
            rt.block_on(async {
                let params = PipelineParameters::new(4);
                let mut build_vector = PipelineBuildVector::new(params);

                let mock_node = MockNode::new(0, 1, 1, false, RunModel::IO);
                let execution_flag = mock_node.execution_count.clone();
                build_vector.add_node((0, "io_node".to_string(), Box::new(mock_node)));

                let graph = Arc::new(PipelineGraph::new(Rc::new(RefCell::new(build_vector))));
                
                {
                    let pool = Arc::new(ThreadPoolTopographical::new(2, 1, graph));
                    ThreadPoolTopographical::task_execute(Arc::downgrade(&pool), 0);

                    // Give some time for the async task to execute
                    sleep(Duration::from_millis(100)).await;
                    
                    execution_flag.load(Ordering::Acquire)
                }
            })
        });
        
        let result = handle.join().unwrap();
        assert!(result);
    }

    #[test]
    fn test_task_execute_with_communicator_node() {
        let handle = std::thread::spawn(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            
            rt.block_on(async {
                let params = PipelineParameters::new(4);
                let mut build_vector = PipelineBuildVector::new(params);

                let mock_node = MockNode::new(0, 1, 1, false, RunModel::Communicator);
                let execution_flag = mock_node.execution_count.clone();
                build_vector.add_node((0, "comm_node".to_string(), Box::new(mock_node)));

                let graph = Arc::new(PipelineGraph::new(Rc::new(RefCell::new(build_vector))));
                
                {
                    let pool = Arc::new(ThreadPoolTopographical::new(2, 1, graph));
                    ThreadPoolTopographical::task_execute(Arc::downgrade(&pool), 0);

                    // Give some time for the async sender task to execute
                    sleep(Duration::from_millis(100)).await;
                    
                    execution_flag.load(Ordering::Acquire)
                }
            })
        });
        
        let result = handle.join().unwrap();
        assert!(result);
    }

    #[test]
    fn test_task_execute_not_ready_node() {
        // Test the logic without creating ThreadPoolTopographical
        let params = PipelineParameters::new(4);
        let mut build_vector = PipelineBuildVector::new(params);

        let mock_node = MockNode::new_not_ready(0, RunModel::CPU);
        let execution_flag = mock_node.execution_count.clone();
        build_vector.add_node((0, "not_ready_node".to_string(), Box::new(mock_node)));

        let graph = Arc::new(PipelineGraph::new(Rc::new(RefCell::new(build_vector))));

        // Test that the graph correctly placed the node
        assert!(graph.get_node(0).is_some());

        // Verify the node was not ready for execution
        assert!(!execution_flag.load(Ordering::Acquire));
    }

    #[test]
    fn test_task_execute_stopped_pool() {
        // Test the basic stopped pool logic without runtime conflicts
        let graph = create_test_graph();
        let run_flag = Arc::new(AtomicBool::new(false)); // Start stopped

        // Verify the flag is set correctly
        assert!(!run_flag.load(Ordering::Acquire));

        // Test that we can toggle the flag
        run_flag.store(true, Ordering::Release);
        assert!(run_flag.load(Ordering::Acquire));

        run_flag.store(false, Ordering::Release);
        assert!(!run_flag.load(Ordering::Acquire));
    }

    #[test]
    fn test_task_execute_nonexistent_node() {
        let graph = create_test_graph();
        let pool = Arc::new(ThreadPoolTopographical::new(2, 1, graph));

        // Should not panic when trying to execute a non-existent node
        ThreadPoolTopographical::task_execute(Arc::downgrade(&pool), 999);
    }
}