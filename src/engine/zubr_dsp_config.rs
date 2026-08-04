use crate::engine::data_plane::construction::unfinished_node_builder::PipelineInterfaceConfiguration;


#[derive(Clone)]
pub struct PipelineAnalyticsParameters {
    pub analytics_sink_buffer_size: usize,
    pub analytics_interval: usize,
}
#[derive(Clone)]
pub struct PipelineParameters {
    pub max_in_flight: usize,
    pub num_compute_threads: usize,
    pub stop_broadcast_buffer_size: usize,
    pub verify_topology: bool,
    pub analytics_parameters: Option<PipelineAnalyticsParameters>,
    pub proxied: bool
}

impl PipelineParameters {
    pub fn new(
        max_in_flight: usize,
        num_compute_threads: usize,
        verify_topology: bool,
        stop_broadcast_buffer_size: usize,
        analytics_parameters: Option<PipelineAnalyticsParameters>,
    ) -> Self {
        Self {
            max_in_flight,
            num_compute_threads,
            verify_topology,
            stop_broadcast_buffer_size,
            analytics_parameters,
            proxied: false
        }
    }

    pub fn standard() -> Self {
        Self {
            max_in_flight: 64,
            num_compute_threads: 8,
            verify_topology: false,
            stop_broadcast_buffer_size: 32,
            analytics_parameters: Some(
                PipelineAnalyticsParameters {
                    analytics_interval: 1024,
                    analytics_sink_buffer_size: 1024
                }
            ),
            proxied: false
        }
    }

    pub fn standard_no_analytics() -> Self {
        Self {
            max_in_flight: 64,
            num_compute_threads: 8,
            verify_topology: false,
            stop_broadcast_buffer_size: 32,
            analytics_parameters: None,
            proxied: false
        }
    }
}