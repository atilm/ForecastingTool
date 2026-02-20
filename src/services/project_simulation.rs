use std::collections::HashMap;

use rand::Rng;
use rand_distr::{Beta, Distribution};
use thiserror::Error;

use crate::domain::calendar::TeamCalendar;
use crate::domain::estimate::{
    Estimate, ReferenceEstimate, StoryPointEstimate, ThreePointEstimate,
};
use crate::domain::issue::{Issue, IssueStatus};
use crate::domain::project::Project;
use crate::services::histogram::HistogramError;
use crate::services::project_yaml::{ProjectYamlError, load_project_from_yaml_file};
use crate::services::simulation_types::{
    SimulationOutput, SimulationPercentile, SimulationReport, WorkPackagePercentiles,
    WorkPackageSimulation,
};
use crate::services::team_calendar_yaml::load_team_calendar_from_yaml_dir;
use crate::services::team_calendar_yaml::TeamCalendarYamlError;
use petgraph::algo::toposort;
use petgraph::graph::DiGraph;
use petgraph::graph::NodeIndex;

#[derive(Error, Debug)]
pub enum ProjectSimulationError {
    #[error("failed to read project yaml: {0}")]
    ReadProject(#[from] std::io::Error),
    #[error("failed to parse project yaml: {0}")]
    ParseProject(#[from] ProjectYamlError),
    #[error("failed to read team calendar yaml: {0}")]
    ReadCalendar(#[from] TeamCalendarYamlError),
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
    #[error("invalid start date: {0}")]
    InvalidStartDate(String),
    #[error("missing velocity for story point estimates")]
    MissingVelocity,
    #[error("dependency {dependency} not found for issue {issue}")]
    UnknownDependency { issue: String, dependency: String },
    #[error("dependency graph has a cycle")]
    CyclicDependencies,
    #[error("invalid estimate values for issue {0}")]
    InvalidEstimate(String),
    #[error("failed to render histogram: {0}")]
    Histogram(#[from] HistogramError),
}

pub fn simulate_project_from_yaml_file(
    path: &str,
    iterations: usize,
    start_date: &str,
    calendar_path: Option<&str>,
) -> Result<SimulationOutput, ProjectSimulationError> {
    let project = load_project_from_yaml_file(path)?;
    let calendar = load_team_calendar_if_provided(calendar_path)?;
    let mut output = simulate_project(&project, iterations, start_date)?;
    output.report.data_source = data_source_name(path);
    Ok(output)
}

fn load_team_calendar_if_provided(
    calendar_path: Option<&str>,
) -> Result<Option<TeamCalendar>, ProjectSimulationError> {
    if let Some(path) = calendar_path {
        let calendar = load_team_calendar_from_yaml_dir(path)?;
        Ok(Some(calendar))
    } else {
        Ok(Some(TeamCalendar::new()))
    }
}

pub fn simulate_project(
    project: &Project,
    iterations: usize,
    start_date: &str,
) -> Result<SimulationOutput, ProjectSimulationError> {
    if iterations == 0 {
        return Err(ProjectSimulationError::InvalidIterations);
    }
    if project.work_packages.is_empty() {
        return Err(ProjectSimulationError::EmptyProject);
    }

    let velocity = if project_has_story_points(project) {
        Some(calculate_project_velocity(project)?)
    } else {
        None
    };
    let order = topological_sort(project)?;
    let start_date = chrono::NaiveDate::parse_from_str(start_date, "%Y-%m-%d")
        .map_err(|_| ProjectSimulationError::InvalidStartDate(start_date.to_string()))?;
    let mut rng = rand::thread_rng();
    let output =
        run_simulation_with_rng(project, &order, velocity, iterations, start_date, &mut rng)?;
    Ok(output)
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

    let first = selected
        .first()
        .ok_or(ProjectSimulationError::MissingVelocityData)?;
    let last = selected
        .last()
        .ok_or(ProjectSimulationError::MissingVelocityData)?;
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
    velocity: Option<f32>,
    iterations: usize,
    start_date: chrono::NaiveDate,
    rng: &mut R,
) -> Result<SimulationOutput, ProjectSimulationError> {
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

    total_durations.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let report = SimulationReport {
        data_source: String::new(),
        start_date: start_date.format("%Y-%m-%d").to_string(),
        velocity,
        iterations,
        simulated_items: project.work_packages.len(),
        p0: SimulationPercentile {
            days: percentile_value(&total_durations, 0.0),
            date: end_date_from_days(start_date, percentile_value(&total_durations, 0.0))
                .format("%Y-%m-%d")
                .to_string(),
        },
        p50: SimulationPercentile {
            days: percentile_value(&total_durations, 50.0),
            date: end_date_from_days(start_date, percentile_value(&total_durations, 50.0))
                .format("%Y-%m-%d")
                .to_string(),
        },
        p85: SimulationPercentile {
            days: percentile_value(&total_durations, 85.0),
            date: end_date_from_days(start_date, percentile_value(&total_durations, 85.0))
                .format("%Y-%m-%d")
                .to_string(),
        },
        p100: SimulationPercentile {
            days: percentile_value(&total_durations, 100.0),
            date: end_date_from_days(start_date, percentile_value(&total_durations, 100.0))
                .format("%Y-%m-%d")
                .to_string(),
        },
    };

    let work_packages = nodes
        .values()
        .map(|node| WorkPackageSimulation {
            id: node.id.clone(),
            percentiles: percentiles_from_values(&node.samples),
        })
        .collect();

    let output = SimulationOutput {
        report,
        results: total_durations,
        work_packages: Some(work_packages),
    };
    Ok(output)
}

fn data_source_name(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(path)
        .to_string()
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
            .as_ref()
            .map(|deps| deps.iter().map(|dep| dep.id.clone()).collect())
            .unwrap_or_default();

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
    let mut graph: DiGraph<String, ()> = DiGraph::new();
    let mut indices: HashMap<String, NodeIndex> = HashMap::new();

    for issue in &project.work_packages {
        let id = issue
            .issue_id
            .as_ref()
            .map(|issue_id| issue_id.id.clone())
            .ok_or(ProjectSimulationError::MissingIssueId)?;
        indices
            .entry(id.clone())
            .or_insert_with(|| graph.add_node(id));
    }

    for issue in &project.work_packages {
        let id = issue
            .issue_id
            .as_ref()
            .map(|issue_id| issue_id.id.clone())
            .ok_or(ProjectSimulationError::MissingIssueId)?;
        let issue_idx = *indices
            .get(&id)
            .ok_or(ProjectSimulationError::MissingIssueId)?;
        if let Some(deps) = issue.dependencies.as_ref() {
            for dep in deps {
                let dep_idx = match indices.get(&dep.id) {
                    Some(idx) => *idx,
                    None => {
                        return Err(ProjectSimulationError::UnknownDependency {
                            issue: id.clone(),
                            dependency: dep.id.clone(),
                        });
                    }
                };
                graph.add_edge(dep_idx, issue_idx, ());
            }
        }
    }

    let sorted = toposort(&graph, None).map_err(|_| ProjectSimulationError::CyclicDependencies)?;
    let mut id_by_index = HashMap::new();
    for (id, idx) in indices {
        id_by_index.insert(idx, id);
    }

    let mut ordered = Vec::with_capacity(sorted.len());
    for idx in sorted {
        if let Some(id) = id_by_index.get(&idx) {
            ordered.push(id.clone());
        }
    }
    Ok(ordered)
}

fn story_point_value(issue: &Issue) -> Option<f32> {
    match issue.estimate.as_ref()? {
        Estimate::StoryPoint(StoryPointEstimate { estimate }) => *estimate,
        Estimate::ThreePoint(_) => None,
        Estimate::Reference(_) => None,
    }
}

fn sample_duration<R: Rng + ?Sized>(
    estimate: &Estimate,
    velocity: Option<f32>,
    rng: &mut R,
    issue_id: &str,
) -> Result<f32, ProjectSimulationError> {
    let (optimistic, most_likely, pessimistic, apply_velocity) = match estimate {
        Estimate::StoryPoint(StoryPointEstimate { estimate }) => {
            let value = estimate
                .ok_or_else(|| ProjectSimulationError::InvalidEstimate(issue_id.to_string()))?;
            let (lower, upper) = fibonacci_bounds(value);
            (lower, value, upper, true)
        }
        Estimate::ThreePoint(estimate) => to_three_point_triplet(estimate)?,
        Estimate::Reference(ReferenceEstimate {
            report_file_path: _,
            cached_estimate,
        }) => {
            let estimate = cached_estimate
                .as_ref()
                .ok_or_else(|| ProjectSimulationError::InvalidEstimate(issue_id.to_string()))?;
            to_three_point_triplet(estimate)?
        }
    };

    let sampled = beta_pert_sample(optimistic, most_likely, pessimistic, rng)
        .map_err(|_| ProjectSimulationError::InvalidEstimate(issue_id.to_string()))?;

    if apply_velocity {
        let velocity = velocity.ok_or(ProjectSimulationError::MissingVelocity)?;
        if velocity <= 0.0 {
            return Err(ProjectSimulationError::InvalidVelocityValue);
        }
        Ok(sampled / velocity)
    } else {
        Ok(sampled)
    }
}

fn to_three_point_triplet(
    estimate: &ThreePointEstimate,
) -> Result<(f32, f32, f32, bool), ProjectSimulationError> {
    let optimistic = estimate.optimistic.ok_or_else(|| {
        ProjectSimulationError::InvalidEstimate("missing optimistic value".to_string())
    })?;
    let most_likely = estimate.most_likely.ok_or_else(|| {
        ProjectSimulationError::InvalidEstimate("missing most likely value".to_string())
    })?;
    let pessimistic = estimate.pessimistic.ok_or_else(|| {
        ProjectSimulationError::InvalidEstimate("missing pessimistic value".to_string())
    })?;
    let apply_velocity = false;
    Ok((optimistic, most_likely, pessimistic, apply_velocity))
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
        0.0, 1.0, 2.0, 3.0, 5.0, 8.0, 13.0, 21.0, 34.0, 55.0, 89.0, 144.0, 233.0, 377.0, 610.0,
        987.0,
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

fn percentiles_from_values(values: &[f32]) -> WorkPackagePercentiles {
    if values.is_empty() {
        return WorkPackagePercentiles {
            p0: 0.0,
            p50: 0.0,
            p85: 0.0,
            p100: 0.0,
        };
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    WorkPackagePercentiles {
        p0: percentile_value(&sorted, 0.0),
        p50: percentile_value(&sorted, 50.0),
        p85: percentile_value(&sorted, 85.0),
        p100: percentile_value(&sorted, 100.0),
    }
}

#[derive(Debug, Clone)]
struct SimulationNode {
    id: String,
    estimate: Estimate,
    dependencies: Vec<String>,
    samples: Vec<f32>,
}

fn project_has_story_points(project: &Project) -> bool {
    project.work_packages.iter().any(|issue| {
        matches!(
            issue.estimate,
            Some(Estimate::StoryPoint(StoryPointEstimate {
                estimate: Some(_)
            }))
        )
    })
}

fn end_date_from_days(start_date: chrono::NaiveDate, days: f32) -> chrono::NaiveDate {
    let days = days.ceil().max(0.0) as i64;
    start_date + chrono::Duration::days(days)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::issue::{IssueId, IssueStatus};
    use chrono::NaiveDate;
    use rand::SeedableRng;
    use rand::rngs::StdRng;
    use std::time::{SystemTime, UNIX_EPOCH};

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

    fn build_three_point_issue(id: &str, days: f32, deps: &[&str]) -> Issue {
        let mut issue = Issue::new();
        issue.issue_id = Some(IssueId { id: id.to_string() });
        issue.estimate = Some(Estimate::ThreePoint(ThreePointEstimate {
            optimistic: Some(days),
            most_likely: Some(days),
            pessimistic: Some(days),
        }));
        issue.dependencies = if deps.is_empty() {
            None
        } else {
            Some(
                deps.iter()
                    .map(|dep| IssueId {
                        id: (*dep).to_string(),
                    })
                    .collect(),
            )
        };
        issue
    }

    fn build_story_point_issue(id: &str, points: f32, deps: &[&str]) -> Issue {
        let mut issue = Issue::new();
        issue.issue_id = Some(IssueId { id: id.to_string() });
        issue.status = Some(IssueStatus::ToDo);
        issue.estimate = Some(Estimate::StoryPoint(StoryPointEstimate {
            estimate: Some(points),
        }));
        issue.dependencies = if deps.is_empty() {
            None
        } else {
            Some(
                deps.iter()
                    .map(|dep| IssueId {
                        id: (*dep).to_string(),
                    })
                    .collect(),
            )
        };
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
        issue_a
            .dependencies
            .get_or_insert_with(Vec::new)
            .push(IssueId {
                id: "B".to_string(),
            });
        issue_b
            .dependencies
            .get_or_insert_with(Vec::new)
            .push(IssueId {
                id: "A".to_string(),
            });

        let project = Project {
            name: "Demo".to_string(),
            work_packages: vec![issue_a, issue_b],
        };

        let error = simulate_project(&project, 10, "2026-01-01").unwrap_err();
        assert!(matches!(error, ProjectSimulationError::CyclicDependencies));
    }

    #[test]
    fn simulate_project_with_dependencies_matches_critical_path() {
        let base = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let done_issue = build_done_issue("DONE-1", 100.0, base, base + chrono::Duration::days(1));

        // WP0, WP1, WP2, WP3, expected duration
        let test_cases = vec![
            (1.0, 1.0, 1.0, 1.0, 2.0), // Crit path: WP0 -> WP2 -> FIN
            (6.0, 1.0, 0.0, 1.0, 6.0), // Crit path: WP0 -> FIN
            (2.0, 1.0, 4.0, 1.0, 6.0), // Crit path: WP0 -> WP2 -> FIN
            (1.0, 5.0, 2.0, 1.0, 7.0), // Crit path: WP1 -> WP2 -> FIN
            (1.0, 5.0, 1.0, 4.0, 9.0), // Crit path: WP1 -> WP3 -> FIN
        ];

        // The dependency graph for the test is:
        //
        //    WP0      WP1        SP-1   SP-2
        //     |        |
        //     |        |
        //     |    +---+----+
        //     |    |        |
        //     +---WP2      WP3
        //     |    |        |
        //     +----+--+-----+
        //            |
        //           FIN
        for (idx, (wp0, wp1, wp2, wp3, expected)) in test_cases.into_iter().enumerate() {
            let mut rng = StdRng::seed_from_u64(42 + idx as u64);
            let project = Project {
                name: "Dependent Project".to_string(),
                work_packages: vec![
                    done_issue.clone(),
                    build_story_point_issue("SP-1", 1.0, &[]),
                    build_story_point_issue("SP-2", 1.0, &[]),
                    build_three_point_issue("WP0", wp0, &[]),
                    build_three_point_issue("WP1", wp1, &[]),
                    build_three_point_issue("WP2", wp2, &["WP0", "WP1"]),
                    build_three_point_issue("WP3", wp3, &["WP1"]),
                    build_three_point_issue("FIN", 0.0, &["WP0", "WP2", "WP3"]),
                ],
            };

            let output = run_simulation_with_rng(
                &project,
                &topological_sort(&project).unwrap(),
                Some(calculate_project_velocity(&project).unwrap()),
                25,
                base,
                &mut rng,
            )
            .unwrap();

            let p50 = output.report.p50.days;
            assert!(
                p50 >= expected && p50 <= expected + 0.25,
                "expected ~{expected} days, got {p50}"
            );
            assert_eq!(output.report.iterations, 25);
            assert!(output.report.velocity.is_some());
        }
    }

    #[test]
    fn simulate_project_from_yaml_file_sets_report_fields() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir();
        let input_path = dir.join(format!("project-{nanos}.yaml"));
        let yaml = "name: Demo\nwork_packages:\n  - id: WP-1\n    estimate:\n      type: three_point\n      optimistic: 1\n      most_likely: 2\n      pessimistic: 3\n";
        std::fs::write(&input_path, yaml).unwrap();

        let output =
            simulate_project_from_yaml_file(input_path.to_str().unwrap(), 5, "2026-01-01", None).unwrap();

        assert_eq!(
            output.report.data_source,
            input_path.file_name().unwrap().to_str().unwrap()
        );
        assert_eq!(output.report.iterations, 5);
        assert_eq!(output.report.velocity, None);
    }
}
