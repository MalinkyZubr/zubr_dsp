use super::node_build_vector::PreparedNode;


fn verify_connectedness(prepared_nodes: &Vec<PreparedNode>) -> Result<(), String> {
    let mut node_statuses: Vec<bool> = Vec::with_capacity(prepared_nodes.len());
    let mut current_node_id: usize = 0;
    
    let mut next_node_id_stack = vec![current_node_id];
    
    while !next_node_id_stack.is_empty() {
        current_node_id = next_node_id_stack.pop().unwrap();
        let current_node = &prepared_nodes[current_node_id];
        
        for predecessor in current_node.node.get_predecessors() {
            if !node_statuses[predecessor] {
                next_node_id_stack.push(predecessor);
                node_statuses[predecessor] = true;
            }
        }
        
        for successor in current_node.node.get_successors() {
            if !node_statuses[successor] {
                next_node_id_stack.push(successor);
                node_statuses[successor] = true;
            }
        }
    }
    
    for (idx, node_status) in node_statuses.iter().enumerate() {
        if !*node_status {
            return Err(format!("Node {} {} is unreachable in graph. Connected test failure", node_status, prepared_nodes[idx].name));
        }
    }
    
    Ok(())
}


fn dfs_find_terminal(is_sink: bool, node_id: usize, prepared_nodes: &Vec<PreparedNode>) -> bool {
    let mut node_statuses: Vec<bool> = Vec::with_capacity(prepared_nodes.len());
    let mut current_node_id: usize = node_id;
    let mut terminal_found = false;

    let mut next_node_id_stack = vec![current_node_id];
    
    while !next_node_id_stack.is_empty() && !terminal_found {
        current_node_id = next_node_id_stack.pop().unwrap();
        let current_node = &prepared_nodes[current_node_id];
        if is_sink {
            if prepared_nodes[current_node_id].node.is_source() {
                terminal_found = true;
            }
            for predecessor in current_node.node.get_predecessors() {
                next_node_id_stack.push(predecessor);
                node_statuses[predecessor] = true;
            }
        }
        else {
            if prepared_nodes[current_node_id].node.is_sink() {
                terminal_found = true;
            }
            for successor in current_node.node.get_successors() {
                next_node_id_stack.push(successor);
                node_statuses[successor] = true;
            }
        }
    }
    
    terminal_found
}


fn verify_terminal_matching(prepared_nodes: &Vec<PreparedNode>) -> Result<(), String> {
    let mut sources: Vec<usize> = Vec::new();
    let mut sinks: Vec<usize> = Vec::new();
    
    for node in prepared_nodes.iter() {
        if node.node.is_sink() {
            sinks.push(node.id);
        }
        if node.node.is_source() {
            sources.push(node.id);
        }
    }
    
    for node_id in sources.iter() {
        if !dfs_find_terminal(false, *node_id, prepared_nodes) {
            return Err(format!("Source node {} {} is not connected to any sink node", node_id, prepared_nodes[*node_id].name));
        }
    }
    for node_id in sinks.iter() {
        if !dfs_find_terminal(true, *node_id, prepared_nodes) {
            return Err(format!("Sink node {} {} is not connected to any source node", node_id, prepared_nodes[*node_id].name));
        }
    }
    
    Ok(())
}


const VERIFICATION_STEPS: [fn(&Vec<PreparedNode>) -> Result<(), String>; 2] = [verify_connectedness, verify_terminal_matching];

pub fn verify_pipeline_topology(prepared_nodes: &mut Vec<PreparedNode>) -> Result<(), String> {
    prepared_nodes.sort_by(|a, b| a.id.cmp(&b.id));
    for step in VERIFICATION_STEPS.iter() {
        let step_result = step(prepared_nodes);
        if step_result.is_err() {
            return step_result;
        }
    }

    Ok(())
}
