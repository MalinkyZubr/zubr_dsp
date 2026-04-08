// goals
// I want a macro to generate a PipelineStep that internally calls the methods of several other PipelineSteps
// with intermediary buffers and everything.
// general flowgraph: input -> mut ref to first step -> mut ref to first internal intermediate buffer -> mut ref to input of second step ... -> mut ref to outputs


macro_rules! composite_single_thread_step {
    (
        $name: ident,
        
    )
}