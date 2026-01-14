#![feature(mpmc_channel)]

use std::sync::atomic::AtomicBool;
use std::sync::mpmc::{channel, Sender};
use std::sync::Arc;
use std::thread;

pub trait StaticThreadTaskTopographical {
    fn execute(&mut self) -> (Vec<Arc<dyn StaticThreadTaskTopographical>>, bool);
    
    
}

pub struct StaticThreadPoolSubmitter {
    sender: Sender<Arc<dyn StaticThreadTaskTopographical + Send>>,
}
impl StaticThreadPoolSubmitter {
    pub fn submit(&self, task: Arc<dyn StaticThreadTaskTopographical + Send>) {
        self.sender.send(task).unwrap();
    }
}

pub struct StaticThreadPoolTopographical {
    num_threads: usize,
    threads: Vec<thread::JoinHandle<()>>,
    sender: Sender<Arc<dyn StaticThreadTaskTopographical + Send>>,
    run_flag: Arc<AtomicBool>,
}
impl StaticThreadPoolTopographical {
    pub fn new(num_thread: usize) -> Self {
        if num_thread < 1 {
            // panic here
        }
        let run_flag = Arc::new(AtomicBool::new(true));
        let (sender, receiver) = channel::<Arc<dyn StaticThreadTaskTopographical + Send>>();
        let mut threads = Vec::with_capacity(num_thread);

        for _ in 0..num_thread {
            let run_flag_clone = run_flag.clone();
            let receiver_clone = receiver.clone();
            let sender_clone = sender.clone();
            threads.push(thread::spawn(move || {
                while run_flag_clone.load(std::sync::atomic::Ordering::Acquire) {
                    let mut received_task: Arc<dyn StaticThreadTaskTopographical + Send> =
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
            threads,
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
}
