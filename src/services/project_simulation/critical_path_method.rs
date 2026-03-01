use std::collections::HashMap;

use chrono::NaiveDate;
use petgraph::algo::toposort;
use petgraph::graph::DiGraph;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CriticalPathMethodError {
    #[error("The network contains duplicate node IDs: {0}")]
    DuplicateNodeId(String),
    #[error("The network contains a cycle, which is not allowed in a project schedule.")]
    CycleDetected,
    #[error("A node has a dependency on a non-existent node: {0}")]
    MissingDependency(String),
}

#[derive(Debug, Clone)]
pub struct NetworkNode {
    id: String,
    duration: u32,
    start_date: Option<NaiveDate>,
    end_date: Option<NaiveDate>,
    dependencies: Vec<String>,
}

pub struct ResultNode {
    id: String,
    duration: u32,
    earliest_start: NaiveDate,
    lastest_start: NaiveDate,
    earliest_finish: NaiveDate,
    latest_finish: NaiveDate,
    free_float: u32,
    total_float: u32,
    drag: u32,
}

pub fn critical_path_method(
    network: Vec<NetworkNode>,
    project_start: NaiveDate,
) -> Result<Vec<ResultNode>, CriticalPathMethodError> {
    if network.is_empty() {
        return Ok(vec![]);
    }

    let mut graph: DiGraph<String, ()> = DiGraph::new();
    let mut nodes_by_index = HashMap::new();
    let mut index_by_id = HashMap::new();

    // Add nodes to graph
    for node in &network {
        let graph_node_index = graph.add_node(node.id.clone());

        if index_by_id.contains_key(&node.id) {
            return Err(CriticalPathMethodError::DuplicateNodeId(node.id.clone()));
        }

        index_by_id.insert(node.id.clone(), graph_node_index);
        nodes_by_index.insert(graph_node_index, node);
    }

    for (graph_node_index, node) in &nodes_by_index {
        for dependency in &node.dependencies {
            let dependency_index = index_by_id
                .get(dependency)
                .ok_or_else(|| CriticalPathMethodError::MissingDependency(dependency.clone()))?;
            graph.add_edge(*dependency_index, *graph_node_index, ());
        }
    }

    let sorted_indices =
        toposort(&graph, None).map_err(|_| CriticalPathMethodError::CycleDetected)?;

    let result_vector = sorted_indices
        .into_iter()
        .map(|index| {
            let node = nodes_by_index.get(&index).unwrap();
            ResultNode {
                id: node.id.clone(),
                duration: node.duration,
                earliest_start: project_start, // Placeholder, should be calculated based on dependencies
                lastest_start: project_start, // Placeholder, should be calculated based on dependencies
                earliest_finish: project_start, // Placeholder, should be calculated based on dependencies
                latest_finish: project_start, // Placeholder, should be calculated based on dependencies
                free_float: 0, // Placeholder, should be calculated based on dependencies
                total_float: 0, // Placeholder, should be calculated based on dependencies
                drag: 0,       // Placeholder, should be calculated based on dependencies
            }
        })
        .collect();

    Ok(result_vector)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn build_network_node(id: &str, duration: u32, dependencies: &[&str]) -> NetworkNode {
        NetworkNode {
            id: id.to_string(),
            duration,
            start_date: None,
            end_date: None,
            dependencies: dependencies.into_iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn duplicate_node_ids_are_detected() {
        let network = vec![
            build_network_node("WP1", 1, &[]),
            build_network_node("WP1", 1, &[]), // Duplicate ID
        ];
        let project_start = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let result = critical_path_method(network, project_start);
        assert!(matches!(result, Err(CriticalPathMethodError::DuplicateNodeId(_))));
    }

    #[test]
    fn empty_input_leads_to_empty_output() {
        let network = vec![];
        let project_start = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let result = critical_path_method(network, project_start).unwrap();
        assert!(result.is_empty());
    }

    // The dependency graph for the test is:
    //
    //    WP0      WP1
    //     |        |
    //     |        |
    //     |    +---+----+
    //     |    |        |
    //     +---WP2      WP3
    //     |    |        |
    //     +----+--+-----+
    //            |
    //           FIN
    #[test]
    fn a_topologically_sorted_vec_of_result_nodes_is_returned() {
        let network = vec![
            build_network_node("WP3", 1, &["WP1"]),
            build_network_node("FIN", 1, &["WP0", "WP2", "WP3"]),
            build_network_node("WP1", 1, &[]),
            build_network_node("WP2", 1, &["WP0", "WP1"]),
            build_network_node("WP0", 1, &[]),
        ];
        let project_start = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let result = critical_path_method(network, project_start).unwrap();
        let expected_order = vec!["WP0", "WP1", "WP2", "WP3", "FIN"];
        let result_order: Vec<String> = result.iter().map(|node| node.id.clone()).collect();
        assert_eq!(result_order, expected_order);
    }
}
