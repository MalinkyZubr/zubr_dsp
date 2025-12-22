use crossbeam::queue::SegQueue;
use std::{
    sync::{Arc, Condvar, RwLock},
    thread,
};

trait ComputeBounds = Send + Sync + Clone + 'static;

#[derive(Clone)]
pub struct ComputeModule<
    InputParam: ComputeBounds,
    PostcomputeParam: ComputeBounds,
    TaskReturn: ComputeBounds,
> {
    computation_task: fn(InputParam) -> TaskReturn,
    postcompute_task: Option<fn(TaskReturn, &mut PostcomputeParam)>,
    postcompute_param: Option<PostcomputeParam>,
    executor_func: fn(&mut Self, Arc<SegQueue<InputParam>>, Arc<RwLock<bool>>, Arc<RwLock<bool>>),
}

impl<InputParam: ComputeBounds, PostcomputeParam: ComputeBounds, TaskReturn: ComputeBounds>
    ComputeModule<InputParam, PostcomputeParam, TaskReturn>
{
    pub fn new(
        computation_task: fn(InputParam) -> TaskReturn,
        postcompute_task: Option<fn(TaskReturn, &mut PostcomputeParam)>,
        postcompute_param: Option<PostcomputeParam>,
    ) -> Self {
        assert!(
            (postcompute_task.is_some() && postcompute_param.is_some())
                || (postcompute_task.is_none() && postcompute_param.is_none())
        );

        let mut executor_func: fn(
            &mut ComputeModule<InputParam, PostcomputeParam, TaskReturn>,
            Arc<SegQueue<InputParam>>,
            Arc<RwLock<bool>>,
            Arc<RwLock<bool>>,
        ) = Self::executor_nwp;
        if postcompute_task.is_some() && postcompute_param.is_some() {
            executor_func = Self::executor_wp;
        }

        ComputeModule {
            computation_task,
            postcompute_task,
            postcompute_param,
            executor_func,
        }
    }

    fn executor_wp(
        compute_module: &mut Self,
        task_queue: Arc<SegQueue<InputParam>>,
        running: Arc<RwLock<bool>>,
        is_empty: Arc<RwLock<bool>>,
    ) {
        while *running.read().unwrap() {
            match task_queue.pop() {
                None => {
                    *is_empty.write().unwrap() = true;
                }
                Some(value) => {
                    *is_empty.write().unwrap() = false;
                    let result = (compute_module.computation_task)(value);
                    (compute_module.postcompute_task.unwrap())(
                        result,
                        &mut compute_module.postcompute_param.as_mut().unwrap(),
                    );
                }
            }
        }
    }

    pub fn executor_nwp(
        compute_module: &mut Self,
        task_queue: Arc<SegQueue<InputParam>>,
        running: Arc<RwLock<bool>>,
        is_empty: Arc<RwLock<bool>>,
    ) {
        while *running.read().unwrap() {
            match task_queue.pop() {
                None => {
                    *is_empty.write().unwrap() = true;
                }
                Some(value) => {
                    *is_empty.write().unwrap() = false;
                    let _ = (compute_module.computation_task)(value);
                }
            }
        }
    }

    pub fn executor(
        &mut self,
        task_queue: Arc<SegQueue<InputParam>>,
        running: Arc<RwLock<bool>>,
        is_empty: Arc<RwLock<bool>>,
    ) {
        (self.executor_func)(self, task_queue, running, is_empty)
    }
}

pub struct ParallelComputation<
    InputParam: ComputeBounds,
    PostcomputeParam: ComputeBounds,
    TaskReturn: ComputeBounds,
> {
    thread_pool: Vec<thread::JoinHandle<()>>,
    num_threads: usize,
    running: Arc<RwLock<bool>>,
    task_queue: Arc<SegQueue<InputParam>>,
    compute_module: ComputeModule<InputParam, PostcomputeParam, TaskReturn>,
    is_empty_vec: Arc<Vec<Arc<RwLock<bool>>>>,
    pub is_empty: Arc<Condvar>,
}

impl<InputParam: ComputeBounds, PostcomputeParam: ComputeBounds, TaskReturn: ComputeBounds>
    ParallelComputation<InputParam, PostcomputeParam, TaskReturn>
{
    pub fn new(
        num_threads: usize,
        compute_module: ComputeModule<InputParam, PostcomputeParam, TaskReturn>,
    ) -> Self {
        ParallelComputation {
            thread_pool: Vec::new(),
            num_threads,
            running: Arc::new(RwLock::new(false)),
            task_queue: Arc::new(SegQueue::new()),
            compute_module,
            is_empty: Arc::new(Condvar::new()),
            is_empty_vec: Arc::new(vec![Arc::new(RwLock::new(false)); num_threads]),
        }
    }

    pub fn add_task(&mut self, task: InputParam) {
        self.task_queue.push(task)
    }

    pub fn start(&mut self) {
        *self.running.write().unwrap() = true;

        for thread_num in 0..self.num_threads {
            let queue = self.task_queue.clone();
            let running = self.running.clone();
            let mut compute_module = self.compute_module.clone();

            let empty_flag_clone = self.is_empty_vec[thread_num].clone();

            self.thread_pool.push(thread::spawn(move || {
                compute_module.executor(queue, running, empty_flag_clone);
            }));
        }

        let running = self.running.clone();
        let cloned_empty_vec = self.is_empty_vec.clone();
        let is_empty = self.is_empty.clone();

        self.thread_pool.push(thread::spawn(move || {
            while *running.read().unwrap() {
                thread::sleep(std::time::Duration::from_millis(5));
                if cloned_empty_vec.iter().all(|flag| *flag.read().unwrap()) {
                    is_empty.notify_one();
                }
            }
        }))
    }

    pub fn stop(self) {
        *self.running.write().unwrap() = false;

        for thread in self.thread_pool {
            let _ = thread.join();
        }
    }

    pub fn wait_until_complete(&mut self) {
        let lock = std::sync::Mutex::new(false);
        let mut _em = lock.lock().unwrap();

        _em = self.is_empty.wait(_em).unwrap();
    }
}
