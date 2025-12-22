use strum::Display;
use crate::pipeline::api::Sharable;


#[derive(Debug, Clone, Display)]
pub enum ReceiveType<T: Sharable> {
    /* 
    This is a very important enum!
    Single: Only a single value was received
    Reassembled: Multiple values were received and they were reassembled from a series sender somewhere previously
    Multichannel: This data is the aggregation of data received from multiple channels simultaneously
    Dummy: this channel is not configured
    
    The pipeline step must handle how the internal behavior responds to each of these types!
     */
    Single(T),
    Reassembled(Vec<T>),
    Multichannel(Vec<T>),
    Dummy
}
impl<T: Sharable> ReceiveType<T> {
    pub fn to_string(&self) -> &str {
        match self {
            Self::Single(_) => "Single",
            Self::Reassembled(_) => "Reassembled",
            Self::Multichannel(_) => "Multichannel",
            Self::Dummy => "Dummy"
        }
    }
}


#[derive(Debug, Clone)]
pub enum ODFormat<T: Sharable> { // Output Data Format
    /*
    This is a very important enum!
    When you return data from a pipeline step, this defines how the pipeline treats that data. What do you want the pipeline to do with this data?
    Decompose:
        Single Out Behavior: Errors, decomposition only supported for multi out
        Multi Out Behavior: Gives a vector to the multi sender, each element of the vector is sent to a separate channel
            Example: 2 channel audio data is de-Decompose, separated into 2 vectors, and processed by different branches of the pipeline
    Series:
        Single Out Behavior: Iterates over the vector from start to finish sending elements to a single consumer in sequence
            Example: I am performing an overlap add convolution on some data, and need to break it into chunks. I chunk it into a vector of vectors,
            and return it so the sender sends each chunk separately in sequence
        Multiple Out Behavior: Replicates above behavior but round robin on multiple channels (eg, element 0 goes to channel 0, then channel 1, then element 1 goes to channel 0 etc)
    Repeat:
        Resends the same data multiple times (dont know why you would want this but I put it here all the same)
    Standard:
        Single Out Behavior: Just sends the data as is once
        Multiple Out Behavior: Just sends the data as is once but to many channels

    In general: multiplexer holds exact same behavior as the single out
    */
    Decompose(Vec<T>),
    Series(Vec<T>),
    Repeat(T, usize),
    Standard(T)
}

impl<T: Sharable> ODFormat<T> {
    pub fn unwrap_standard(self) -> T {
        match self {
            ODFormat::Standard(x) => x,
            _ => panic!()
        }
    }
}


// how can I make multiple input and output types more convenient?
/*
1. every pipeline step has a separate trait method for each input type, with separate signature. By defualt it will return an error saying its unimplemented
2. user can return whatever data scheme they want from each separate handler for the node to do with what it pleases
3. at the beginning of runtime, depending on the receiver type assigned to the node, a different handler (node method) is chosen to receive, so no additional match is needed
 */
pub trait PipelineStep<I: Sharable, O: Sharable> : Send + 'static {
    // There is a single input, and a single output
    fn run_SISO(&mut self, input: I) -> Result<ODFormat<O>, String> { panic!("Single in Single Out Not Implemented") }
    // You are receiving a vector of values reassembled from a series branch, and are outputting to a single output
    fn run_REASO(&mut self, input: Vec<I>) -> Result<ODFormat<O>, String> { panic!("Series In Single Out Not Implemented") }
    // you are receiving a vector of values, each one representing the output of a distinct branch. Outputting to a single output
    fn run_MISO(&mut self, input: Vec<I>) -> Result<ODFormat<O>, String> { panic!("Multiple In Single Out Not Implemented") }
    // you are receiving a single value and outputting to multiple distinct pipeline branches
    fn run_SIMO(&mut self, input: I) -> Result<ODFormat<O>, String> { panic!("Single In Multiple Out Not Implemented") }
    // you are receiving a vector of values, representing a series receive, and outputing to multiple outputs
    fn run_REAMO(&mut self, input: Vec<I>) -> Result<ODFormat<O>, String> { panic!("Series In Multiple Out Not Implemented") }
    // receiving a vector of outputs from distinct pipeline branches and outputting to multiple distinct pipeline branches
    fn run_MIMO(&mut self, input: Vec<I>) -> Result<ODFormat<O>, String> { panic!("Multiple In Multiple Out Not Implemented") }
    // intended for source nodes. When the source receiver is 'Dummy'. This is a PRODUCER startpoint. Generates its own input
    fn run_DISO(&mut self) -> Result<ODFormat<O>, String> {
        panic!("Dummy in Not Implemented")
    }
    // intended for sink nodes. When the sink sender is 'Dummy'. This is a CONSUMER endpoint. Must put the output wherever it needs to go on its own
    fn run_SIDO(&mut self, input: I) -> Result<ODFormat<O>, String> { panic!("Dummy out Not Implemented") }
    // optional method to be run whenever a pause signal is received
    fn pause_behavior(&mut self) { () }
    // optional method to be run whenever a start signal is received
    fn start_behavior(&mut self) { () }
    // optional method to be run whenevr a kill signal is received
    fn kill_behavior(&mut self) { () }
}