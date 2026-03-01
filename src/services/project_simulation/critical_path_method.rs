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

#[derive(Debug)]
pub struct NetworkNode {
    id: String,
    duration: f32,
    start_date: Option<NaiveDate>,
    end_date: Option<NaiveDate>,
    dependencies: Vec<String>,
}

pub struct ResultNode {
    id: String,
    duration: f32,
    earliest_start: NaiveDate,
    lastest_start: NaiveDate,
    earliest_finish: NaiveDate,
    latest_finish: NaiveDate,
    free_float: f32,
    total_float: f32,
    drag: f32,
}

pub fn critical_path_method(
    network: Vec<NetworkNode>,
    project_start: NaiveDate,
) -> Result<Vec<ResultNode>, CriticalPathMethodError> {
    let nodes_count = network.len();
    let sorted_nodes = topological_sort(network)?;

    let mut earliest_finish: HashMap<String, chrono::NaiveDate> =
        HashMap::with_capacity(nodes_count);
    let mut result_nodes: HashMap<String, ResultNode> = HashMap::with_capacity(nodes_count);

    for node in &sorted_nodes {
        let earliest_start = node
            .dependencies
            .iter()
            .filter_map(|dep| earliest_finish.get(dep))
            .max()
            .cloned()
            .unwrap_or(project_start);

        let earliest_finish_date = earliest_start + chrono::Duration::days(node.duration as i64);
        earliest_finish.insert(node.id.clone(), earliest_finish_date);

        result_nodes.insert(
            node.id.clone(),
            ResultNode {
                id: node.id.clone(),
                duration: node.duration,
                earliest_start,
                lastest_start: project_start, // Placeholder, should be calculated based on dependencies
                earliest_finish: earliest_finish_date,
                latest_finish: project_start, // Placeholder, should be calculated based on dependencies
                free_float: 0.0, // Placeholder, should be calculated based on dependencies
                total_float: 0.0, // Placeholder, should be calculated based on dependencies
                drag: 0.0,       // Placeholder, should be calculated based on dependencies
            },
        );
    }

    let result_vector: Vec<ResultNode> = sorted_nodes
        .into_iter()
        .map(|node| result_nodes.remove(&node.id).unwrap())
        .collect();

    Ok(result_vector)
}

fn topological_sort(
    network: Vec<NetworkNode>,
) -> Result<Vec<NetworkNode>, CriticalPathMethodError> {
    let mut graph: DiGraph<String, ()> = DiGraph::new();
    let mut nodes_by_index = HashMap::new();
    let mut index_by_id = HashMap::new();

    // Add nodes to graph
    for node in network {
        let graph_node_index = graph.add_node(node.id.clone());

        if index_by_id.contains_key(&node.id) {
            return Err(CriticalPathMethodError::DuplicateNodeId(node.id.clone()));
        }

        index_by_id.insert(node.id.clone(), graph_node_index);
        nodes_by_index.insert(graph_node_index, node);
    }

    // Add edges to graph based on dependencies
    for (graph_node_index, node) in &nodes_by_index {
        for dependency in &node.dependencies {
            let dependency_index = index_by_id
                .get(dependency)
                .ok_or_else(|| CriticalPathMethodError::MissingDependency(dependency.clone()))?;
            graph.add_edge(*dependency_index, *graph_node_index, ());
        }
    }

    // Perform topological sort
    let sorted_indices =
        toposort(&graph, None).map_err(|_| CriticalPathMethodError::CycleDetected)?;

    // Create sorted vector of nodes based on sorted indices
    let sorted_nodes: Vec<NetworkNode> = sorted_indices
        .iter()
        .map(|index| nodes_by_index.remove(index).unwrap())
        .collect();

    Ok(sorted_nodes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn build_network_node(id: &str, duration: f32, dependencies: &[&str]) -> NetworkNode {
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
            build_network_node("WP1", 1.0, &[]),
            build_network_node("WP1", 1.0, &[]), // Duplicate ID
        ];
        let project_start = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let result = critical_path_method(network, project_start);
        assert!(matches!(
            result,
            Err(CriticalPathMethodError::DuplicateNodeId(_))
        ));
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
            build_network_node("WP3", 1.0, &["WP1"]),
            build_network_node("FIN", 0.0, &["WP0", "WP2", "WP3"]),
            build_network_node("WP1", 1.0, &[]),
            build_network_node("WP2", 1.0, &["WP0", "WP1"]),
            build_network_node("WP0", 1.0, &[]),
        ];
        let project_start = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let result = critical_path_method(network, project_start).unwrap();
        let expected_order = vec!["WP0", "WP1", "WP2", "WP3", "FIN"];
        let result_order: Vec<String> = result.iter().map(|node| node.id.clone()).collect();
        assert_eq!(result_order, expected_order);
    }

    // struct WorkPackageTestCase {
    //     id: &'static str,
    //     duration: u32,
    //     dependencies: Vec<&'static str>,
    //     expected_earliest_start_day: NaiveDate,
    //     expected_earliest_finish_day: NaiveDate,
    //     expected_latest_start_day: NaiveDate,
    //     expected_latest_finish_day: NaiveDate,
    // }

    #[test]
    fn earliest_start_and_eraliest_finish_are_calculated_correctly() {
        let base = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();

        // WP0, WP1, WP2, WP3, expected duration
        let test_cases: Vec<(f32, f32, f32, f32, f32)> = vec![
            (1.0, 1.0, 1.0, 1.0, 2.0), // Crit path: WP0 -> WP2 -> FIN
            (6.0, 1.0, 0.0, 1.0, 6.0), // Crit path: WP0 -> FIN
            (2.0, 1.0, 4.0, 1.0, 6.0), // Crit path: WP0 -> WP2 -> FIN
            (1.0, 5.0, 2.0, 1.0, 7.0), // Crit path: WP1 -> WP2 -> FIN
            (1.0, 5.0, 1.0, 4.0, 9.0), // Crit path: WP1 -> WP3 -> FIN
        ];

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
        for (_idx, (wp0, wp1, wp2, wp3, expected_duration)) in test_cases.into_iter().enumerate() {
            let network = vec![
                build_network_node("WP0", wp0, &[]),
                build_network_node("WP1", wp1, &[]),
                build_network_node("WP2", wp2, &["WP0", "WP1"]),
                build_network_node("WP3", wp3, &["WP1"]),
                build_network_node("FIN", 0.0, &["WP0", "WP2", "WP3"]),
            ];

            let result = critical_path_method(network, base).unwrap();
            let fin_node = result.iter().find(|node| node.id == "FIN").unwrap();

            assert_eq!(
                fin_node.earliest_finish,
                base + chrono::Duration::days(expected_duration as i64)
            );
        }
    }
}
