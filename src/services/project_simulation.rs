use std::collections::HashMap;

use thiserror::Error;

use crate::domain::calendar::TeamCalendar;
use crate::domain::estimate::{
    Estimate, ReferenceEstimate, StoryPointEstimate, ThreePointEstimate,
};

use crate::domain::project::Project;
use crate::services::beta_pert_sampler::BetaPertSampler;
use crate::services::beta_pert_sampler::ThreePointSampler;
use crate::services::histogram::HistogramError;
use crate::services::project_yaml::{ProjectYamlError, load_project_from_yaml_file};
use crate::services::simulation_types::{
    SimulationOutput, SimulationPercentile, SimulationReport, WorkPackagePercentiles,
    WorkPackageSimulation,
};
use crate::services::team_calendar_yaml::TeamCalendarYamlError;
use crate::services::team_calendar_yaml::load_team_calendar_from_yaml_dir;
use crate::services::velocity_calculation::VelocityCalculationError;
use crate::services::velocity_calculation::calculate_project_velocity;
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
    #[error("failed to calculate velocity: {0}")]
    VelocityCalculation(#[from] VelocityCalculationError),
    #[error("iterations must be greater than zero")]
    InvalidIterations,
    #[error("project has no work packages")]
    EmptyProject,
    #[error("missing issue id")]
    MissingIssueId,
    #[error("missing estimate for issue {0}")]
    MissingEstimate(String),
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
    let mut output = simulate_project(&project, iterations, start_date, calendar)?;
    output.report.data_source = data_source_name(path);
    Ok(output)
}

fn load_team_calendar_if_provided(
    calendar_path: Option<&str>,
) -> Result<TeamCalendar, ProjectSimulationError> {
    if let Some(path) = calendar_path {
        let calendar = load_team_calendar_from_yaml_dir(path)?;
        Ok(calendar)
    } else {
        Ok(TeamCalendar::new())
    }
}

pub fn simulate_project(
    project: &Project,
    iterations: usize,
    start_date: &str,
    calendar: TeamCalendar,
) -> Result<SimulationOutput, ProjectSimulationError> {
    if iterations == 0 {
        return Err(ProjectSimulationError::InvalidIterations);
    }
    if project.work_packages.is_empty() {
        return Err(ProjectSimulationError::EmptyProject);
    }

    let velocity = if project_has_story_points(project) {
        Some(calculate_project_velocity(project, &calendar)?)
    } else {
        None
    };
    let order = topological_sort(project)?;
    let start_date = chrono::NaiveDate::parse_from_str(start_date, "%Y-%m-%d")
        .map_err(|_| ProjectSimulationError::InvalidStartDate(start_date.to_string()))?;
    let mut rng = rand::thread_rng();
    let mut sampler = BetaPertSampler::new(&mut rng);
    let output = run_simulation(
        project,
        &order,
        velocity,
        iterations,
        start_date,
        &mut sampler,
        &calendar,
    )?;
    Ok(output)
}

fn run_simulation<R: ThreePointSampler + ?Sized>(
    project: &Project,
    order: &[String],
    velocity: Option<f32>,
    iterations: usize,
    start_date: chrono::NaiveDate,
    sampler: &mut R,
    calendar: &TeamCalendar,
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
            let duration = sample_duration(&node.estimate, velocity, sampler, &node.id)?;
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

fn sample_duration<R: ThreePointSampler + ?Sized>(
    estimate: &Estimate,
    velocity: Option<f32>,
    sampler: &mut R,
    issue_id: &str,
) -> Result<f32, ProjectSimulationError> {
    let (optimistic, most_likely, pessimistic, is_story_point_estimate) = match estimate {
        Estimate::StoryPoint(estimate ) => to_story_point_triplet(estimate, issue_id)?,
        Estimate::ThreePoint(estimate) => to_three_point_triplet(estimate)?,
        Estimate::Reference(estimate) => to_reference_triplet(estimate, issue_id)?,
    };

    let sampled = sampler
        .sample(optimistic, most_likely, pessimistic)
        .map_err(|_| ProjectSimulationError::InvalidEstimate(issue_id.to_string()))?;

    if is_story_point_estimate {
        let velocity = velocity.ok_or(ProjectSimulationError::MissingVelocity)?;
        if velocity <= 0.0 {
            return Err(ProjectSimulationError::InvalidVelocityValue);
        }
        Ok(sampled / velocity)
    } else {
        Ok(sampled)
    }
}



fn to_reference_triplet(
    reference: &ReferenceEstimate,
    issue_id: &str
) -> Result<(f32, f32, f32, bool), ProjectSimulationError> {
    let cached = reference
        .cached_estimate
        .as_ref()
        .ok_or_else(|| ProjectSimulationError::InvalidEstimate(issue_id.to_string()))?;
    to_three_point_triplet(cached)
}

fn to_story_point_triplet(
    story_points: &StoryPointEstimate,
    issue_id: &str,
) -> Result<(f32, f32, f32, bool), ProjectSimulationError> {
    let value = story_points
        .estimate
        .ok_or_else(|| ProjectSimulationError::InvalidEstimate(issue_id.to_string()))?;
    let (lower, upper) = fibonacci_bounds(value);
    let is_story_point_estimate = true;
    Ok((lower, value, upper, is_story_point_estimate))
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
    let is_story_point_estimate = false;
    Ok((optimistic, most_likely, pessimistic, is_story_point_estimate))
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
    use crate::domain::issue::IssueId;
    use crate::test_support::{
        build_done_issue, build_story_point_issue, build_three_point_issue,
        create_calendar_without_any_free_days, on_date,
    };
    use chrono::NaiveDate;
    use rand::SeedableRng;
    use rand::rngs::StdRng;
    use std::time::{SystemTime, UNIX_EPOCH};

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
        let calendar = create_calendar_without_any_free_days();

        let error = simulate_project(&project, 10, "2026-01-01", calendar).unwrap_err();
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
            let mut sampler = BetaPertSampler::new(&mut rng);
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
            let calendar = create_calendar_without_any_free_days();

            let output = run_simulation(
                &project,
                &topological_sort(&project).unwrap(),
                Some(calculate_project_velocity(&project, &calendar).unwrap()),
                25,
                base,
                &mut sampler,
                &calendar,
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
    fn project_simulation_takes_calendar_into_account() {
        use crate::test_support::MockSampler;

        let mut sampler = MockSampler;
        let project = Project {
            name: "Dependent Project".to_string(),
            work_packages: vec![
                build_done_issue("SP-0", 2.0, on_date(2026, 2, 13), on_date(2026, 2, 13)), // velocity of 2 points/day
                build_story_point_issue("SP-1", 2.0, &[]),
                build_story_point_issue("SP-2", 2.0, &["SP-1"]),
                build_story_point_issue("SP-3", 2.0, &["SP-2"]),
                build_story_point_issue("SP-4", 2.0, &["SP-3"]),
                build_story_point_issue("SP-5", 2.0, &["SP-4"]),
                build_story_point_issue("SP-6", 2.0, &["SP-5"]),
            ],
        };
        let calendar = TeamCalendar::new(); // default calendar which assumes weekends to be free

        let output = run_simulation(
            &project,
            &topological_sort(&project).unwrap(),
            Some(calculate_project_velocity(&project, &calendar).unwrap()),
            1,
            on_date(2026, 2, 16), // Start on a Monday
            &mut sampler,
            &calendar,
        )
        .unwrap();

        let velocity = output.report.velocity.unwrap();
        let p50_days = output.report.p50.days;

        assert_eq!(
            output.report.p0.days, output.report.p100.days,
            "With deterministic sampling, p0 and p100 should be the same"
        );

        assert_eq!(
            velocity, 2.0,
            "Expected velocity of 2 points/day based on the completed issue"
        );
        assert_eq!(
            p50_days, 8.0,
            "Expected 8 days to complete 12 story points with a velocity of 2 points/day, taking into account the weekend"
        );
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
            simulate_project_from_yaml_file(input_path.to_str().unwrap(), 5, "2026-01-01", None)
                .unwrap();

        assert_eq!(
            output.report.data_source,
            input_path.file_name().unwrap().to_str().unwrap()
        );
        assert_eq!(output.report.iterations, 5);
        assert_eq!(output.report.velocity, None);
    }
}
