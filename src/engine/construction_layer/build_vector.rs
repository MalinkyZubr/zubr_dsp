use log::info;
use crate::engine::structural::generic_pipeline_node::CollectibleNode;


#[derive(Clone)]
pub struct PipelineParameters {
    pub buff_size: usize,
}
impl PipelineParameters {
    pub fn new(buff_size: usize) -> Self {
        Self {
            buff_size: buff_size,
        }
    }
}


pub struct PreparedNode {
    pub node: Box<dyn CollectibleNode>,
    pub id: usize,
    pub name: String
}
impl PreparedNode {
    pub fn new(node: Box<dyn CollectibleNode>, id: usize, name: String) -> Self {
        PreparedNode {
            node,
            id,
            name
        }
    }
}


pub type NodeSubmissionClosure = Box<dyn FnOnce() -> PreparedNode>;


pub struct PipelineBuildVector {
    submission_closures: Vec<NodeSubmissionClosure>,
    id_counter: usize,
    pipeline_parameters: PipelineParameters,
}
impl PipelineBuildVector {
    pub fn new(pipeline_parameters: PipelineParameters) -> Self {
        PipelineBuildVector {
            submission_closures: Vec::new(),
            id_counter: 0,
            pipeline_parameters
        }
    }
    
    pub fn promise_node(&mut self, closure: NodeSubmissionClosure) {
        self.submission_closures.push(closure);
    }
    
    pub fn submit_nodes(&mut self) -> Vec<PreparedNode> {
        let mut submitted_nodes = Vec::new();
        
        for closure in self.submission_closures.drain(..) {
            let node = closure();
            submitted_nodes.push(node);
        }
        
        submitted_nodes
    }

    pub fn get_new_id(&mut self) -> usize {
        let out = self.id_counter;
        self.id_counter += 1;
        out
    }
    
    pub fn get_pipeline_parameters(&self) -> &PipelineParameters {
        &self.pipeline_parameters
    }
}