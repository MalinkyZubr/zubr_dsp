use crate::pipeline::api::HasDefault;
use crate::pipeline::pipeline_traits::Sharable;
use std::sync::mpmc::RecvTimeoutError;
use std::sync::mpsc::Receiver;
use std::time::Duration;

#[derive(Debug)]
pub struct WrappedReceiver<T: Sharable> {
    receiver: Receiver<T>,
    feedback_startup_flag: bool,
}
impl<T: Sharable + HasDefault> WrappedReceiver<T> {
    pub fn new(receiver: Receiver<T>) -> Self {
        WrappedReceiver {
            receiver,
            feedback_startup_flag: false,
        }
    }
    pub fn set_startup_flag(mut self) -> Self {
        self.feedback_startup_flag = true;
        self
    }
    fn result_handler(
        &self,
        result: &mut Result<T, RecvTimeoutError>,
        retry_num: &mut usize,
        retries: usize,
    ) -> bool {
        match result {
            Err(err) => {
                match err {
                    RecvTimeoutError::Timeout => *retry_num += 1,
                    _ => *retry_num = retries,
                };
                false
            }
            Ok(_) => true,
        }
    }
    pub fn recv(&mut self, timeout: u64, retries: usize) -> Result<T, RecvTimeoutError> {
        let mut retry_num = 0;

        let mut result = Err(RecvTimeoutError::Timeout);
        let mut success_flag = false;

        while retry_num < retries && !success_flag {
            result = self.receiver.recv_timeout(Duration::from_millis(timeout));

            success_flag = self.result_handler(&mut result, &mut retry_num, retries);
        }
        if self.feedback_startup_flag {
            self.feedback_startup_flag = false;
            Ok(T::default())
        } else {
            result
        }
    }
}
