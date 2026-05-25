use crate::engine::control_plane::node_state_manager::{
    generate_node_state_manager, ExternalStopSource, InternodeStopAuthorityFactory,
    NodeStateManager,
};
use crate::engine::control_plane::pipeline_analytics::{PipelineAnalyticsSink, PipelineAnalyticsSource};
use crate::engine::data_plane::construction::node_build_vector::PreparedNode;
use crate::engine::data_plane::structural::generic_pipeline_node::{GenericNode, NodeState};
use std::future::Future;
use std::sync::Arc;
use tokio::sync::watch::Receiver as WatchReceiver;

pub fn wrap_prepared_nodes(
    mut prepared_nodes: Vec<PreparedNode>,
    analytics_sink: &Option<Arc<PipelineAnalyticsSink>>,
    stop_broadcast_buffer_size: usize,
) -> (Vec<NodeWrapper>, ExternalStopSource) {
    let mut external_stop_source = ExternalStopSource::new();
    let internode_stopping_authority_factory =
        InternodeStopAuthorityFactory::new(stop_broadcast_buffer_size);

    let wrapped_nodes = prepared_nodes
        .drain(..)
        .map(|node| {
            let id = node.id;
            if node.node.get_num_outputs() == 0 || node.node.get_num_inputs() == 0 {
                let mut wrapper = NodeWrapper::new_endpoint_node(
                    node,
                    &internode_stopping_authority_factory,
                    external_stop_source.register_endpoint(id),
                );
                match analytics_sink {
                    Some(sink) => wrapper.attach_analytics_source(sink.generate_source(id)),
                    _ => ()
                }
                wrapper
            } else {
                let mut wrapper = NodeWrapper::new_general_node(node, &internode_stopping_authority_factory);
                match analytics_sink {
                    Some(sink) => wrapper.attach_analytics_source(sink.generate_source(id)),
                    _ => ()
                }
                wrapper
            }
        })
        .collect::<Vec<NodeWrapper>>();

    (wrapped_nodes, external_stop_source)
}


pub type CPUTask = Box<dyn FnOnce() -> NodeWrapper + Send + 'static>;
pub type IOTask = Box<dyn Future<Output=NodeWrapper> + Send + 'static>;
pub type CommTask = Box<dyn Future<Output=(NodeWrapper, Option<usize>)> + Send + 'static>;
pub enum RunType {
    CPU(CPUTask),
    IO(IOTask),
    COMM(CommTask),
}

pub struct NodeWrapper {
    id: usize,
    name: String,
    node: Box<dyn GenericNode>,
    analytic_source: Option<PipelineAnalyticsSource>,
    state_manager: Box<dyn NodeStateManager>,
}
impl NodeWrapper {
    pub fn new_endpoint_node(
        prepared_node: PreparedNode,
        internode_stop_authority_factory: &InternodeStopAuthorityFactory,
        stop_receiver: WatchReceiver<bool>,
    ) -> Self {
        let state_manager = generate_node_state_manager(
            &prepared_node,
            internode_stop_authority_factory,
            Some(stop_receiver),
        );
        Self {
            id: prepared_node.id,
            name: prepared_node.name,
            node: prepared_node.node,
            analytic_source: None,
            state_manager,
        }
    }

    pub fn new_general_node(
        prepared_node: PreparedNode,
        internode_stop_authority_factory: &InternodeStopAuthorityFactory,
    ) -> Self {
        let state_manager =
            generate_node_state_manager(&prepared_node, internode_stop_authority_factory, None);

        Self {
            id: prepared_node.id,
            name: prepared_node.name,
            node: prepared_node.node,
            analytic_source: None,
            state_manager,
        }
    }

    fn wrapped_run_exec_cpu(mut wrapper: Self) -> CPUTask {
        Box::new(move || {
            wrapper.state_manager.update_state(&wrapper.node);
            let result;
            match wrapper.analytic_source.as_mut() {
                Some(mut analytic_source) => {
                    analytic_source.enter_execution();
                    result = wrapper.node.call_thread_cpu();
                    analytic_source.exit_execution();
                }
                None => result = wrapper.node.call_thread_cpu(),
            }
            match result { //error handling later to be immplemment
                Ok(()) => (),
                Err(()) => (),
            };
            wrapper
        })
    }

    fn wrapped_run_exec_io(mut wrapper: Self) -> IOTask {
        Box::new(async move {
            wrapper.state_manager.update_state(&wrapper.node);
            let result;
            match wrapper.analytic_source.as_mut() {
                Some(mut analytic_source) => {
                    analytic_source.enter_execution();
                    result = wrapper.node.call_thread_io().await;
                    analytic_source.exit_execution();
                }
                None => result = wrapper.node.call_thread_io().await,
            }
            wrapper
        })
    }

    fn wrapped_run_exec_comm(
        mut wrapper: Self,
    ) -> CommTask {
        Box::new(async move {
            wrapper.state_manager.update_state(&wrapper.node);
            let mut successors;
            match wrapper.analytic_source.as_mut() {
                Some(mut analytic_source) => {
                    analytic_source.enter_execution();
                    successors = wrapper.node.run_senders().await;
                    analytic_source.exit_execution();
                }
                None => successors = wrapper.node.run_senders().await,
            }
            (wrapper, successors)
        })
    }

    fn wrapped_run_comm(
        mut wrapper: Self,
    ) -> CommTask {
        Box::new(async move {
            let successors = wrapper.node.run_senders().await;
            (wrapper, successors)
        })
    }

    pub fn generate_run(mut self) -> Result<RunType, Self> {
        if self.node.is_ready_exec(self.state_manager.get_state()) {
            return Err(self);
        }
        match self.state_manager.get_state() {
            NodeState::ExecCpu => Ok(RunType::CPU(Self::wrapped_run_exec_cpu(self))),
            NodeState::ExecIo => Ok(RunType::IO(Self::wrapped_run_exec_io(self))),
            NodeState::ExecCommunicate => Ok(RunType::COMM(Self::wrapped_run_exec_comm(self))),
            NodeState::Communicate => Ok(RunType::COMM(Self::wrapped_run_comm(self))),
            NodeState::Stop => Err(self),
        }
    }

    pub fn attach_analytics_source(&mut self, source: PipelineAnalyticsSource) {
        self.analytic_source = Some(source);
    }

    pub fn get_state(&self) -> NodeState {
        self.state_manager.get_state()
    }

    pub fn get_id(&self) -> usize {
        self.id
    }

    async fn run_senders(&mut self) {
        self.node.run_senders().await;
    }

    pub fn get_successors(&self) -> Vec<usize> {
        self.node.get_successors()
    }

    pub fn get_predecessors(&self) -> Vec<usize> {
        self.node.get_predecessors()
    }

    pub fn is_ready_exec(&self) -> bool {
        self.node.is_ready_exec(self.state_manager.get_state())
    }

    pub fn is_source(&self) -> bool {
        self.node.get_num_inputs() == 0
    }

    pub fn is_sink(&self) -> bool {
        self.node.get_num_outputs() == 0
    }

    pub fn has_initial_state(&self) -> bool {
        self.node.has_initial_value()
    }
    
    pub fn get_name(&self) -> String {
        self.name.clone()
    }
    
    pub fn get_satiated_edges(&self, num_satiated: usize) -> &[usize] {
        self.node.get_satiated_edges(num_satiated)
    }
}
