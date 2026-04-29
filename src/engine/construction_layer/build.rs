use crate::engine::construction_layer::node_build_vector::PipelineBuildVector;
use crate::engine::construction_layer::unfinished_node_builder::PipelineParameters;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use log::warn;
use tokio::runtime::Runtime;
use crate::engine::construction_layer::topology_verify::verify_pipeline_topology;
use crate::engine::interface_layer::interface_hl::Interface;
use crate::engine::orchestration_layer::pipeline_graph::PipelineGraph;
use crate::engine::orchestration_layer::pipeline_hl::{Pipeline, PipelineScheduler};

pub type PipelineBuildRoutine = Box<dyn FnOnce(Rc<RefCell<PipelineBuildVector>>, PipelineParameters) -> ()>;


// ts so tuff
pub fn build_pipeline<Scheduler: PipelineScheduler>(
    build_routine: PipelineBuildRoutine,
    pipeline_parameters: PipelineParameters,
    io_op_runtime: Runtime,
    verify: bool
) -> Result<Pipeline<Scheduler>, String> { // should return some handle type on ok, not ()
    let mut build_vector = Rc::new(RefCell::new(PipelineBuildVector::new()));
    build_routine(build_vector.clone(), pipeline_parameters.clone());
    
    let mut prepared_nodes = build_vector.borrow_mut().submit_nodes();
    if verify {
        match verify_pipeline_topology(&mut prepared_nodes) {
            Ok(_) => (),
            Err(e) => return Err(e)
        }
    }
    else {
        warn!("Toplogy verification is disabled. Those checks are there for a reason!!!")
    }
    
    let graph = Arc::new(PipelineGraph::new(prepared_nodes));
    let scheduler: Scheduler = Scheduler::new(graph.clone(), pipeline_parameters, io_op_runtime);
    let pipeline = Pipeline::new(scheduler, graph, Interface {});
    
    Ok(pipeline)
}
