#![feature(mpmc_channel)]

use std::sync::atomic::AtomicBool;
use rayon::{ThreadPoolBuilder, ThreadPool};
use std::sync::Arc;
use crate::pipeline::orchestration_layer::pipeline_graph::{PipelineGraph};

pub trait ThreadTaskTopographical {
    fn execute(&mut self) -> (Vec<Arc<dyn ThreadTaskTopographical>>, bool);
}

pub struct ThreadPoolTopographical {
    num_threads: usize,
    thread_pool: ThreadPool,
    run_flag: Arc<AtomicBool>,
}
impl ThreadPoolTopographical {
    pub fn new(num_thread: usize) -> Self {
        if num_thread < 1 {
            // panic here
        }
        let run_flag = Arc::new(AtomicBool::new(true));
        let mut threads = Vec::with_capacity(num_thread);

        for _ in 0..num_thread {
            let run_flag_clone = run_flag.clone();
            threads.push(thread::spawn(move || {
                while run_flag_clone.load(std::sync::atomic::Ordering::Acquire) {
                    let mut received_task: Arc<dyn ThreadTaskTopographical + Send> =
                        receiver_clone.recv().unwrap();
                    let (successor_tasks, is_source) = received_task.execute();

                    for successor_task in successor_tasks {
                        sender_clone.send(successor_task).unwrap();
                    }
                    if is_source {
                        sender_clone.send(received_task).unwrap();
                    }
                }
            }))
        }
        Self {
            num_threads: num_thread,
            thread_pool: ThreadPoolBuilder::new().
            run_flag,
            sender,
        }
    }

    pub fn acquire_submitter(&self) -> StaticThreadPoolSubmitter {
        StaticThreadPoolSubmitter {
            sender: self.sender.clone(),
        }
    }

    pub fn shutdown(&mut self) {
        self.run_flag
            .store(false, std::sync::atomic::Ordering::Release);
        for thread in self.threads.drain(..) {
            thread.join().unwrap();
        }
    }
    
    pub fn thread_task(task_id: usize, task_list: Arc<PipelineGraph>) {
        // need run flag check here
        let task = task_list.get_node(task_id);
        let mutable_task_object = task.thread_object.get_mut().unwrap(); // system guarantees that nothing else is trying to run this at the same time. No need to lock
        mutable_task_object.call_thread();
        
        
        for edge in task.successors.iter() {
            edge.num_executions_since_completion.fetch_add(1, std::sync::atomic::Ordering::Release);

            let successor_node = task_list.get_node(edge.destination_id);

            if !successor_node.is_running() && 
                successor_node.predecessors_responsibility_fulfilled() // && is state okay
            {
                let destination_id = edge.destination_id.clone();
                let task_list_clone = task_list.clone();
                rayon::spawn(move || Self::thread_task(destination_id, task_list_clone));
            }
        }

        if task.is_source() || task.predecessors_responsibility_fulfilled() {
            rayon::spawn(move || Self::thread_task(task_id, task_list))
        }
    }
}
