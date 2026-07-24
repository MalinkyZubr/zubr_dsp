use crate::engine::data_plane::construction::node_build_vector::PipelineBuildVector;
use crate::engine::data_plane::construction::unfinished_node_builder::{PipelineInterfaceConfiguration, PipelineParameters};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use log::warn;
use tokio::runtime::Runtime;
use crate::engine::control_plane::node_state_manager::{ExternalStopSource, InternodeStopAuthorityFactory};
use crate::engine::control_plane::node_wrapper::{wrap_prepared_nodes, NodeWrapper};
//use crate::engine::data_plane::construction::topology_verify::verify_pipeline_topology;
use crate::engine::interface_layer::interface_hl::Interface;
use crate::engine::control_plane::pipeline_graph::PipelineGraph;
use crate::engine::control_plane::pipeline_hl::{Pipeline, PipelineScheduler};
use crate::engine::control_plane::pipeline_analytics::PipelineAnalyticsSink;


pub type PipelineBuildRoutine = Box<dyn FnOnce(Rc<RefCell<PipelineBuildVector>>, PipelineParameters) -> ()>;


// ts so tuff
pub fn build_pipeline<Scheduler: PipelineScheduler>(
    build_routine: PipelineBuildRoutine,
    pipeline_parameters: PipelineParameters,
    io_op_runtime: Arc<Runtime>,
) -> Result<Pipeline<Scheduler>, String> { // should return some handle type on ok, not ()
    let build_vector = Rc::new(RefCell::new(PipelineBuildVector::new()));
    build_routine(build_vector.clone(), pipeline_parameters.clone());
    
    let prepared_nodes = build_vector.borrow_mut().submit_nodes();
    if pipeline_parameters.verify_topology {
        // match verify_pipeline_topology(&mut prepared_nodes) {
        //     Ok(_) => (),
        //     Err(e) => return Err(e)
        // }
    }
    else {
        warn!("Toplogy verification is disabled. Those checks are there for a reason!!!")
    }
    
    let analytics_sink = match pipeline_parameters.pipeline_configuration {
        PipelineInterfaceConfiguration::GUI | PipelineInterfaceConfiguration::TermFull => {
            Some(Arc::new(PipelineAnalyticsSink::new(pipeline_parameters.analytics_sink_buffer_size)))
        },
        _ => None
    };
    
    let (wrapped_nodes, external_stop_source) = wrap_prepared_nodes(prepared_nodes, &analytics_sink, pipeline_parameters.stop_broadcast_buffer_size);
    
    let graph = Arc::new(PipelineGraph::new(wrapped_nodes));
    let scheduler: Scheduler = Scheduler::new(graph.clone(), pipeline_parameters, io_op_runtime);
    let pipeline = Pipeline::new(scheduler, graph, external_stop_source, analytics_sink);
    
    Ok(pipeline)
}
