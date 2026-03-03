use crate::domain::project::Project;
use crate::services::project_simulation::beta_pert_sampler::ThreePointSampler;
use crate::services::project_simulation::critical_path_method::NetworkNode;
use crate::services::project_simulation::sample_duration::SamplingError;
use crate::services::project_simulation::sample_duration::sample_duration_days;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum NetworkNodesError {
    #[error("missing issue ID for an issue")]
    MissingIssueId,
    #[error("missing estimate for issue {0}")]
    MissingEstimate(String),
    #[error("sampling error: {0}")]
    Sampling(#[from] SamplingError),
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
            duration,
            start_date,
            end_date,
            dependencies,
        });
    }

    Ok(nodes)
}
