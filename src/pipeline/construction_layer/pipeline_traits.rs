use std::fmt::Debug;
use std::sync::mpmc::{RecvError, RecvTimeoutError, SendError};
use std::sync::mpsc::{Receiver, SyncSender};


pub trait HasDefault {
    fn default() -> Self;
}

pub trait Sharable = Send + Sync + Debug + Clone + HasDefault + 'static;

pub trait Source {}
pub trait Sink {}

pub trait Unit: Send + Clone {
    fn gen() -> Self;
}
impl Unit for () {
    fn gen() -> Self {
        ()
    }
}

pub trait HasID {
    fn get_id(&self) -> String;
    fn set_id(&mut self, id: &str);
}