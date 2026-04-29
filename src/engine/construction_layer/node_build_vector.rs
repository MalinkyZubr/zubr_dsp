use log::info;
use crate::engine::structural::generic_pipeline_node::GenericNode;


pub struct PreparedNode {
    pub node: Box<dyn GenericNode>,
    pub id: usize,
    pub name: String
}
impl PreparedNode {
    pub fn new(node: Box<dyn GenericNode>, id: usize, name: String) -> Self {
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
}
impl PipelineBuildVector {
    pub fn new() -> Self {
        PipelineBuildVector {
            submission_closures: Vec::new(),
            id_counter: 0,
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
}