use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::{select, spawn};
use tokio::runtime::Runtime;
use tokio::sync::watch::{channel, Receiver, Sender};


pub struct BackgroundTask {
    task: Pin<Box<dyn Future<Output = ()> + Send + 'static>>,
    kill_switch: Receiver<()>
}
impl BackgroundTask {
    pub fn new(kill_switch: Receiver<()>, task: Pin<Box<dyn Future<Output = ()> + Send + 'static>>) -> BackgroundTask {
        BackgroundTask {
            task, kill_switch
        }
    }
    pub async fn run(mut self) {
        let mut kill_flag = false;
        
        while !kill_flag {
            select! {
                _ = &mut self.task => {},
                _ = self.kill_switch.changed() => {
                    kill_flag = true
                }
            }
        }
    }
}


pub struct BackgroundTaskManager {
    registered_tasks: HashMap<String, tokio::task::JoinHandle<()>>,
    task_kill_switches: HashMap<String, Sender<()>>,
    runtime: Arc<Runtime>,
}
impl BackgroundTaskManager {
    pub fn new(runtime: Arc<Runtime>) -> BackgroundTaskManager {
        BackgroundTaskManager {
            registered_tasks: HashMap::new(),
            task_kill_switches: HashMap::new(),
            runtime
        }
    }
    
    pub fn add_task(&mut self, task_name: String, task: Pin<Box<dyn Future<Output = ()> + Send + 'static>>) {
        let (sender, receiver) = channel(());
        let task_obj = BackgroundTask::new(receiver, task);
        self.registered_tasks.insert(task_name.clone(), self.runtime.spawn(task_obj.run()));
        self.task_kill_switches.insert(task_name, sender);
    }
    
    pub fn remove_task(&mut self, task_name: &str) {
        let kill_switch = self.task_kill_switches.remove(task_name).unwrap();
        kill_switch.send(()).unwrap();
        self.registered_tasks.remove(task_name);
    }
}