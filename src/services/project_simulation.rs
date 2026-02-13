use std::collections::{BTreeSet, HashMap};

use rand::Rng;
use rand_distr::{Beta, Distribution};
use serde::Serialize;
use thiserror::Error;

use crate::domain::estimate::{Estimate, StoryPointEstimate, ThreePointEstimate};
use crate::domain::issue::{Issue, IssueStatus};
use crate::domain::project::Project;
use crate::services::project_yaml::{load_project_from_yaml_file, ProjectYamlError};

#[derive(Error, Debug)]
pub enum ProjectSimulationError {
    #[error("failed to read project yaml: {0}")]
    ReadProject(#[from] std::io::Error),
    #[error("failed to parse project yaml: {0}")]
    ParseProject(#[from] ProjectYamlError),
    #[error("iterations must be greater than zero")]
    InvalidIterations,
    #[error("project has no work packages")]
    EmptyProject,
    #[error("missing issue id")]
    MissingIssueId,
    #[error("missing estimate for issue {0}")]
    MissingEstimate(String),
    #[error("missing dates for velocity calculation")]
    MissingVelocityDates,
    #[error("no completed issues with story point estimates")]
    MissingVelocityData,
    #[error("invalid velocity duration")]
    InvalidVelocityDuration,
    #[error("invalid velocity value")]
    InvalidVelocityValue,
    #[error("dependency {dependency} not found for issue {issue}")]
    UnknownDependency { issue: String, dependency: String },
    #[error("dependency graph has a cycle")]
    CyclicDependencies,
    #[error("invalid estimate values for issue {0}")]
    InvalidEstimate(String),
}

#[derive(Serialize, Debug, Clone)]
pub struct SimulationPercentiles {
    pub p0: f32,
    pub p50: f32,
    pub p85: f32,
    pub p100: f32,
}

#[derive(Serialize, Debug, Clone)]
pub struct WorkPackageSimulationReport {
    pub id: String,
    pub percentiles: SimulationPercentiles,
}

#[derive(Serialize, Debug, Clone)]
pub struct ProjectSimulationReport {
    pub velocity: f32,
    pub iterations: usize,
    pub total: SimulationPercentiles,
    pub work_packages: Vec<WorkPackageSimulationReport>,
}

pub async fn simulate_project_from_yaml_file(
    path: &str,
    iterations: usize,
) -> Result<ProjectSimulationReport, ProjectSimulationError> {
    let project = load_project_from_yaml_file(path).await?;
    simulate_project(&project, iterations)
}

pub fn simulate_project(
    project: &Project,
    iterations: usize,
) -> Result<ProjectSimulationReport, ProjectSimulationError> {
    if iterations == 0 {
        return Err(ProjectSimulationError::InvalidIterations);
    }
    if project.work_packages.is_empty() {
        return Err(ProjectSimulationError::EmptyProject);
    }

    let velocity = calculate_project_velocity(project)?;
    let order = topological_sort(project)?;
    let mut rng = rand::thread_rng();
    run_simulation_with_rng(project, &order, velocity, iterations, &mut rng)
}

pub fn calculate_project_velocity(project: &Project) -> Result<f32, ProjectSimulationError> {
    let mut completed: Vec<&Issue> = project
        .work_packages
        .iter()
        .filter(|issue| issue.status == Some(IssueStatus::Done))
        .filter(|issue| story_point_value(issue).is_some())
        .filter(|issue| issue.start_date.is_some() && issue.done_date.is_some())
        .collect();

    if completed.is_empty() {
        return Err(ProjectSimulationError::MissingVelocityData);
    }

    completed.sort_by_key(|issue| issue.done_date);
    let selected = if completed.len() > 30 {
        &completed[completed.len() - 30..]
    } else {
        completed.as_slice()
    };

    let first = selected.first().ok_or(ProjectSimulationError::MissingVelocityData)?;
    let last = selected.last().ok_or(ProjectSimulationError::MissingVelocityData)?;
    let start_date = first
        .start_date
        .ok_or(ProjectSimulationError::MissingVelocityDates)?;
    let end_date = last
        .done_date
        .ok_or(ProjectSimulationError::MissingVelocityDates)?;

    let duration_days = end_date.signed_duration_since(start_date).num_days();
    if duration_days <= 0 {
        return Err(ProjectSimulationError::InvalidVelocityDuration);
    }

    let total_points: f32 = selected
        .iter()
        .filter_map(|issue| story_point_value(issue))
        .sum();
    let velocity = total_points / duration_days as f32;
    if velocity <= 0.0 {
        return Err(ProjectSimulationError::InvalidVelocityValue);
    }

    Ok(velocity)
}

fn run_simulation_with_rng<R: Rng + ?Sized>(
    project: &Project,
    order: &[String],
    velocity: f32,
    iterations: usize,
    rng: &mut R,
) -> Result<ProjectSimulationReport, ProjectSimulationError> {
    let mut nodes = build_simulation_nodes(project)?;
    let mut total_durations = Vec::with_capacity(iterations);

    for _ in 0..iterations {
        let mut earliest_finish: HashMap<String, f32> = HashMap::new();
        for id in order {
            let node = nodes
                .get_mut(id)
                .ok_or(ProjectSimulationError::MissingIssueId)?;
            let start = node
                .dependencies
                .iter()
                .filter_map(|dep| earliest_finish.get(dep))
                .fold(0.0_f32, |acc, value| acc.max(*value));
            let duration = sample_duration(&node.estimate, velocity, rng, &node.id)?;
            let end_time = start + duration;
            node.samples.push(end_time);
            earliest_finish.insert(node.id.clone(), end_time);
        }

        let project_duration = earliest_finish
            .values()
            .fold(0.0_f32, |acc, value| acc.max(*value));
        total_durations.push(project_duration);
    }

    let total = percentiles_from_values(&total_durations);
    let work_packages = nodes
        .values()
        .map(|node| WorkPackageSimulationReport {
            id: node.id.clone(),
            percentiles: percentiles_from_values(&node.samples),
        })
        .collect();

    Ok(ProjectSimulationReport {
        velocity,
        iterations,
        total,
        work_packages,
    })
}

fn build_simulation_nodes(
    project: &Project,
) -> Result<HashMap<String, SimulationNode>, ProjectSimulationError> {
    let mut nodes = HashMap::new();
    for issue in &project.work_packages {
        let id = issue
            .issue_id
            .as_ref()
            .map(|issue_id| issue_id.id.clone())
            .ok_or(ProjectSimulationError::MissingIssueId)?;
        let estimate = issue
            .estimate
            .clone()
            .ok_or_else(|| ProjectSimulationError::MissingEstimate(id.clone()))?;
        let dependencies = issue
            .dependencies
            .iter()
            .map(|dep| dep.id.clone())
            .collect();

        nodes.insert(
            id.clone(),
            SimulationNode {
                id,
                estimate,
                dependencies,
                samples: Vec::new(),
            },
        );
    }

    Ok(nodes)
}

fn topological_sort(project: &Project) -> Result<Vec<String>, ProjectSimulationError> {
    let mut dependencies: HashMap<String, Vec<String>> = HashMap::new();
    for issue in &project.work_packages {
        let id = issue
            .issue_id
            .as_ref()
            .map(|issue_id| issue_id.id.clone())
            .ok_or(ProjectSimulationError::MissingIssueId)?;
        let deps = issue
            .dependencies
            .iter()
            .map(|dep| dep.id.clone())
            .collect();
        dependencies.insert(id, deps);
    }

    for (issue_id, deps) in &dependencies {
        for dep in deps {
            if !dependencies.contains_key(dep) {
                return Err(ProjectSimulationError::UnknownDependency {
                    issue: issue_id.clone(),
                    dependency: dep.clone(),
                });
            }
        }
    }

    let mut indegree: HashMap<String, usize> = HashMap::new();
    let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();
    for issue_id in dependencies.keys() {
        indegree.insert(issue_id.clone(), 0);
    }

    for (issue_id, deps) in &dependencies {
        for dep in deps {
            *indegree.entry(issue_id.clone()).or_insert(0) += 1;
            adjacency.entry(dep.clone()).or_default().push(issue_id.clone());
        }
    }

    let mut queue: BTreeSet<String> = indegree
        .iter()
        .filter_map(|(id, degree)| if *degree == 0 { Some(id.clone()) } else { None })
        .collect();

    let mut order = Vec::with_capacity(indegree.len());
    while let Some(id) = queue.iter().next().cloned() {
        queue.remove(&id);
        order.push(id.clone());

        if let Some(children) = adjacency.get(&id) {
            for child in children {
                if let Some(value) = indegree.get_mut(child) {
                    *value = value.saturating_sub(1);
                    if *value == 0 {
                        queue.insert(child.clone());
                    }
                }
            }
        }
    }

    if order.len() != indegree.len() {
        return Err(ProjectSimulationError::CyclicDependencies);
    }

    Ok(order)
}

fn story_point_value(issue: &Issue) -> Option<f32> {
    match issue.estimate.as_ref()? {
        Estimate::StoryPoint(StoryPointEstimate { estimate }) => *estimate,
        Estimate::ThreePoint(_) => None,
    }
}

fn sample_duration<R: Rng + ?Sized>(
    estimate: &Estimate,
    velocity: f32,
    rng: &mut R,
    issue_id: &str,
) -> Result<f32, ProjectSimulationError> {
    if velocity <= 0.0 {
        return Err(ProjectSimulationError::InvalidVelocityValue);
    }

    let (optimistic, most_likely, pessimistic) = match estimate {
        Estimate::StoryPoint(StoryPointEstimate { estimate }) => {
            let value = estimate.ok_or_else(|| {
                ProjectSimulationError::InvalidEstimate(issue_id.to_string())
            })?;
            let (lower, upper) = fibonacci_bounds(value);
            (lower, value, upper)
        }
        Estimate::ThreePoint(ThreePointEstimate {
            optimistic,
            most_likely,
            pessimistic,
        }) => {
            let optimistic = optimistic.ok_or_else(|| {
                ProjectSimulationError::InvalidEstimate(issue_id.to_string())
            })?;
            let most_likely = most_likely.ok_or_else(|| {
                ProjectSimulationError::InvalidEstimate(issue_id.to_string())
            })?;
            let pessimistic = pessimistic.ok_or_else(|| {
                ProjectSimulationError::InvalidEstimate(issue_id.to_string())
            })?;
            (optimistic, most_likely, pessimistic)
        }
    };

    let sampled_points = beta_pert_sample(optimistic, most_likely, pessimistic, rng)
        .map_err(|_| ProjectSimulationError::InvalidEstimate(issue_id.to_string()))?;
    Ok(sampled_points / velocity)
}

fn beta_pert_sample<R: Rng + ?Sized>(
    optimistic: f32,
    most_likely: f32,
    pessimistic: f32,
    rng: &mut R,
) -> Result<f32, ()> {
    if pessimistic < optimistic {
        return Err(());
    }
    if (pessimistic - optimistic).abs() < f32::EPSILON {
        return Ok(optimistic);
    }
    if most_likely < optimistic || most_likely > pessimistic {
        return Err(());
    }

    let range = (pessimistic - optimistic) as f64;
    let alpha = 1.0 + 4.0 * ((most_likely - optimistic) as f64 / range);
    let beta = 1.0 + 4.0 * ((pessimistic - most_likely) as f64 / range);
    let beta_dist = Beta::new(alpha, beta).map_err(|_| ())?;
    let sample = beta_dist.sample(rng) as f32;
    Ok(optimistic + sample * (pessimistic - optimistic))
}

fn fibonacci_bounds(value: f32) -> (f32, f32) {
    let series = [
        0.0, 1.0, 2.0, 3.0, 5.0, 8.0, 13.0, 21.0, 34.0, 55.0, 89.0, 144.0, 233.0, 377.0,
        610.0, 987.0,
    ];

    if value <= series[0] {
        return (series[0], series[1]);
    }

    for window in series.windows(2) {
        let lower = window[0];
        let upper = window[1];
        if value <= upper {
            return (lower, upper);
        }
    }

    let last = *series.last().unwrap();
    (last, last)
}

fn percentiles_from_values(values: &[f32]) -> SimulationPercentiles {
    if values.is_empty() {
        return SimulationPercentiles {
            p0: 0.0,
            p50: 0.0,
            p85: 0.0,
            p100: 0.0,
        };
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    SimulationPercentiles {
        p0: percentile_value(&sorted, 0.0),
        p50: percentile_value(&sorted, 50.0),
        p85: percentile_value(&sorted, 85.0),
        p100: percentile_value(&sorted, 100.0),
    }
}

fn percentile_value(sorted_values: &[f32], percentile: f64) -> f32 {
    if sorted_values.is_empty() {
        return 0.0;
    }
    if percentile <= 0.0 {
        return sorted_values[0];
    }
    if percentile >= 100.0 {
        return sorted_values[sorted_values.len() - 1];
    }
    let position = (percentile / 100.0) * (sorted_values.len() as f64 - 1.0);
    let index = position.round() as usize;
    sorted_values[index]
}

#[derive(Debug, Clone)]
struct SimulationNode {
    id: String,
    estimate: Estimate,
    dependencies: Vec<String>,
    samples: Vec<f32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::issue::{IssueId, IssueStatus};
    use chrono::NaiveDate;

    fn build_done_issue(id: &str, points: f32, start: NaiveDate, done: NaiveDate) -> Issue {
        let mut issue = Issue::new();
        issue.issue_id = Some(IssueId { id: id.to_string() });
        issue.status = Some(IssueStatus::Done);
        issue.start_date = Some(start);
        issue.done_date = Some(done);
        issue.estimate = Some(Estimate::StoryPoint(StoryPointEstimate {
            estimate: Some(points),
        }));
        issue
    }

    #[test]
    fn calculate_velocity_from_done_story_points() {
        let base = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let mut issues = Vec::new();
        for idx in 0..30 {
            let start = base + chrono::Duration::days(idx);
            let done = start + chrono::Duration::days(1);
            issues.push(build_done_issue(&format!("ABC-{idx}"), 2.0, start, done));
        }
        let project = Project {
            name: "Demo".to_string(),
            work_packages: issues,
        };

        let velocity = calculate_project_velocity(&project).unwrap();
        assert!((velocity - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn calculate_velocity_uses_last_thirty_issues() {
        let base = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let mut issues = Vec::new();
        for idx in 0..31 {
            let start = base + chrono::Duration::days(idx);
            let done = start + chrono::Duration::days(1);
            issues.push(build_done_issue(&format!("ABC-{idx}"), 1.0, start, done));
        }
        let project = Project {
            name: "Demo".to_string(),
            work_packages: issues,
        };

        let velocity = calculate_project_velocity(&project).unwrap();
        let expected = 30.0 / 30.0;
        assert!((velocity - expected).abs() < f32::EPSILON);
    }

    #[test]
    fn simulate_rejects_cyclic_dependencies() {
        let base = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let mut issue_a = build_done_issue("A", 1.0, base, base + chrono::Duration::days(1));
        let mut issue_b = build_done_issue("B", 1.0, base, base + chrono::Duration::days(2));
        issue_a.dependencies.push(IssueId { id: "B".to_string() });
        issue_b.dependencies.push(IssueId { id: "A".to_string() });

        let project = Project {
            name: "Demo".to_string(),
            work_packages: vec![issue_a, issue_b],
        };

        let error = simulate_project(&project, 10).unwrap_err();
        assert!(matches!(error, ProjectSimulationError::CyclicDependencies));
    }
}
