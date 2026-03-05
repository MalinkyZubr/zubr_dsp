use std::thread::{JoinHandle, Thread};

pub struct ThreadPoolPerNode {
    thread_pool: Vec<JoinHandle<()>>
    
}