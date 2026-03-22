use crate::domain::project::Project;
use crate::services::project_simulation::beta_pert_sampler::ThreePointSampler;
use crate::services::project_simulation::sample_duration::SamplingError;
use crate::services::project_simulation::sample_duration::sample_duration_days;
use chrono::NaiveDate;
use petgraph::algo::toposort;
use petgraph::graph::DiGraph;
use thiserror::Error;
use std::collections::HashMap;

#[derive(Error, Debug)]
pub enum NetworkNodesError {
    #[error("missing issue ID for an issue")]
    MissingIssueId,
    #[error("missing estimate for issue {0}")]
    MissingEstimate(String),
    #[error("sampling error: {0}")]
    Sampling(#[from] SamplingError),
    #[error("duplicate node ID found: {0}")]
    DuplicateNodeId(String),
    #[error("missing dependency: {0}")]
    MissingDependency(String),
    #[error("cycle detected in dependencies")]
    CycleDetected,
}

#[derive(Debug)]
pub struct NetworkNode {
    pub id: String,
    pub is_milestone: bool,
    pub duration: f32,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub dependencies: Vec<String>,
}

pub struct SortedNetworkNodes(Vec<NetworkNode>);

impl SortedNetworkNodes {
    pub fn new(nodes: Vec<NetworkNode>) -> Result<Self , NetworkNodesError> {
        let sorted_nodes = topological_sort(nodes)?;
        Ok(SortedNetworkNodes(sorted_nodes))
    }

    pub fn take(self) -> Vec<NetworkNode> {
        self.0
    }
}

pub fn build_network_nodes<R: ThreePointSampler + ?Sized>(
    project: &Project,
    velocity: Option<f32>,
    sampler: &mut R,
) -> Result<Vec<NetworkNode>, NetworkNodesError> {
    let mut nodes = Vec::with_capacity(project.work_packages.len());

    for issue in project.work_packages.iter() {
        let id = issue
            .issue_id
            .as_ref()
            .map(|issue_id| issue_id.id.clone())
            .ok_or(NetworkNodesError::MissingIssueId)?;
        let start_date = issue.start_date;
        let end_date = issue.done_date;
        let estimate = issue
            .estimate
            .clone()
            .ok_or_else(|| NetworkNodesError::MissingEstimate(id.clone()))?;
        let dependencies = issue
            .dependencies
            .as_ref()
            .map(|deps| deps.iter().map(|dep| dep.id.clone()).collect())
            .unwrap_or_default();

        let duration = sample_duration_days(&estimate, velocity, sampler, &id)?;

        nodes.push(NetworkNode {
            id,
            is_milestone: issue.is_milestone(),
            duration,
            start_date,
            end_date,
            dependencies,
        });
    }

    Ok(nodes)
}

fn topological_sort(
    network: Vec<NetworkNode>,
) -> Result<Vec<NetworkNode>, NetworkNodesError> {
    let mut graph: DiGraph<String, ()> = DiGraph::new();
    let mut nodes_by_index = HashMap::new();
    let mut index_by_id = HashMap::new();

    // Add nodes to graph
    for node in network {
        let graph_node_index = graph.add_node(node.id.clone());

        if index_by_id.contains_key(&node.id) {
            return Err(NetworkNodesError::DuplicateNodeId(node.id.clone()));
        }

        index_by_id.insert(node.id.clone(), graph_node_index);
        nodes_by_index.insert(graph_node_index, node);
    }

    // Add edges to graph based on dependencies
    for (graph_node_index, node) in &nodes_by_index {
        for dependency in &node.dependencies {
            let dependency_index = index_by_id
                .get(dependency)
                .ok_or_else(|| NetworkNodesError::MissingDependency(dependency.clone()))?;
            graph.add_edge(*dependency_index, *graph_node_index, ());
        }
    }

    // Perform topological sort
    let sorted_indices =
        toposort(&graph, None).map_err(|_| NetworkNodesError::CycleDetected)?;

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

    fn build_network_node(id: &str, duration: f32, dependencies: &[&str]) -> NetworkNode {
        NetworkNode {
            id: id.to_string(),
            is_milestone: false,
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
        let result = SortedNetworkNodes::new(network);
        assert!(matches!(
            result,
            Err(NetworkNodesError::DuplicateNodeId(_))
        ));
    }

    #[test]
    fn missing_dependency_is_detected() {
        let network = vec![
            build_network_node("WP0", 1.0, &["WP1"]), // WP1 does not exist
        ];
        let result = SortedNetworkNodes::new(network);
        assert!(matches!(
            result,
            Err(NetworkNodesError::MissingDependency(_))
        ));
    }

    #[test]
    fn cycle_detection_works() {
        let network = vec![
            build_network_node("WP0", 1.0, &["WP1"]),
            build_network_node("WP1", 1.0, &["WP0"]),
        ];
        let result = SortedNetworkNodes::new(network);
        assert!(matches!(
            result,
            Err(NetworkNodesError::CycleDetected)
        ));
    }
}
