#![feature(mpmc_channel)]

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::thread;
use std::sync::mpmc::{channel, Sender, Receiver};


pub struct StaticThreadPoolSubmitter {
    sender: Sender<Box<dyn Fn(String)>>
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
    runflag: Arc<AtomicBool>
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
            sender
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