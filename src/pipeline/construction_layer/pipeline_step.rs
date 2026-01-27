use crate::pipeline::communication_layer::formats::ODFormat;

// how can I make multiple input and output types more convenient?
/*
1. every pipeline step has a separate trait method for each input type, with separate signature. By defualt it will return an error saying its unimplemented
2. user can return whatever data scheme they want from each separate handler for the node to do with what it pleases
3. at the beginning of runtime, depending on the receiver type assigned to the node, a different handler (node method) is chosen to receive, so no additional match is needed
 */
pub trait PipelineStep<I: Sharable, O: Sharable> : Send + 'static {
    // There is a single input, and a single output
    fn run_SISO(&mut self, input: I) -> Result<ODFormat<O>, ()> { panic!("Single in Single Out Not Implemented") }
    // You are receiving a vector of values reassembled from a series branch, and are outputting to a single output
    fn run_REASO(&mut self, input: Vec<I>) -> Result<ODFormat<O>, ()> { panic!("Series In Single Out Not Implemented") }
    // you are receiving a vector of values, each one representing the output of a distinct branch. Outputting to a single output
    fn run_MISO(&mut self, input: Vec<I>) -> Result<ODFormat<O>, ()> { panic!("Multiple In Single Out Not Implemented") }
    // you are receiving a single value and outputting to multiple distinct pipeline branches
    fn run_SIMO(&mut self, input: I) -> Result<ODFormat<O>, ()> { panic!("Single In Multiple Out Not Implemented") }
    // you are receiving a vector of values, representing a series receive, and outputing to multiple outputs
    fn run_REAMO(&mut self, input: Vec<I>) -> Result<ODFormat<O>, ()> { panic!("Series In Multiple Out Not Implemented") }
    // receiving a vector of outputs from distinct pipeline branches and outputting to multiple distinct pipeline branches
    fn run_MIMO(&mut self, input: Vec<I>) -> Result<ODFormat<O>, ()> { panic!("Multiple In Multiple Out Not Implemented") }
    // intended for source nodes. When the source receiver is 'Dummy'. This is a PRODUCER startpoint. Generates its own input
    fn run_DISO(&mut self) -> Result<ODFormat<O>, ()> {
        panic!("Dummy in Not Implemented")
    }
    // intended for sink nodes. When the sink sender is 'Dummy'. This is a CONSUMER endpoint. Must put the output wherever it needs to go on its own
    fn run_SIDO(&mut self, input: I) -> Result<ODFormat<O>, ()> { panic!("Dummy out Not Implemented") }
    // optional method to be run whenever a pause signal is received
    fn pause_behavior(&mut self) { () }
    // optional method to be run whenever a start signal is received
    fn start_behavior(&mut self) { () }
    // optional method to be run whenevr a kill signal is received
    fn kill_behavior(&mut self) { () }
}