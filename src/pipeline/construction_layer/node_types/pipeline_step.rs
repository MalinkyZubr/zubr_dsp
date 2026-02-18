use async_trait::async_trait;
use crate::pipeline::construction_layer::pipeline_traits::Sharable;

// how can I make multiple input and output types more convenient?
/*
1. every pipeline step has a separate trait method for each input type, with separate signature. By defualt it will return an error saying its unimplemented
2. user can return whatever data scheme they want from each separate handler for the node to do with what it pleases
3. at the beginning of runtime, depending on the receiver type assigned to the node, a different handler (node method) is chosen to receive, so no additional match is needed
 */
#[async_trait]
pub trait PipelineStep<I: Sharable, O: Sharable, const NI: usize> : Send + 'static {
    fn run_cpu(&mut self, input: [I; NI]) -> Result<O, ()> { panic!("run not implemented!") }
    async fn run_io(&mut self, input: [I; NI]) -> Result<O, ()> { panic!("run not implemented!") }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug)]
    struct S;

    // Intentionally does not override `run_cpu` / `run_io`.
    struct DefaultPanicsStep;

    #[async_trait::async_trait]
    impl PipelineStep<S, S, 1> for DefaultPanicsStep {}

    #[test]
    fn default_run_cpu_panics() {
        let result = std::panic::catch_unwind(|| {
            let mut step = DefaultPanicsStep;
            let _ = step.run_cpu([S]);
        });

        assert!(result.is_err());
    }
}