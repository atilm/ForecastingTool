use std::collections::HashMap;

use thiserror::Error;

use crate::domain::calendar::TeamCalendar;

use crate::services::project_simulation::network_nodes::build_network_nodes;

use chrono::NaiveDate;

use crate::domain::project::Project;
use crate::services::histogram::HistogramError;
use crate::services::percentiles;
use crate::services::project_simulation::beta_pert_sampler::BetaPertSampler;
use crate::services::project_simulation::beta_pert_sampler::ThreePointSampler;
use crate::services::project_simulation::sample_duration::SamplingError;
use crate::services::project_simulation::velocity_calculation::VelocityCalculationError;
use crate::services::project_simulation::velocity_calculation::calculate_project_velocity;
use crate::services::project_yaml::{ProjectYamlError, load_project_from_yaml_file};
use crate::services::simulation_types::{
    SimulationOutput, SimulationPercentile, SimulationReport, WorkPackagePercentiles,
    WorkPackageSimulation,
};
use crate::services::team_calendar_yaml::TeamCalendarYamlError;
use crate::services::team_calendar_yaml::load_team_calendar_if_provided;
use crate::services::util::dates::data_source_name;

use crate::services::project_simulation::critical_path_method::CriticalPathMethodError;
use crate::services::project_simulation::critical_path_method::critical_path_method;
use crate::services::project_simulation::network_nodes::NetworkNodesError;

#[derive(Debug, Clone, Copy)]
struct WorkItemSample {
    end_date: chrono::NaiveDate,
}

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
    #[error("failed to render histogram: {0}")]
    Histogram(#[from] HistogramError),
    #[error("failed to calculate percentiles: {0}")]
    SamplingError(#[from] SamplingError),
    #[error("failed to perform critical path method analysis: {0}")]
    CriticalPathMethod(#[from] CriticalPathMethodError),
    #[error("failed to build network nodes: {0}")]
    NetworkNodes(#[from] NetworkNodesError),
}

pub fn simulate_project_from_yaml_file(
    path: &str,
    iterations: usize,
    start_date: NaiveDate,
    calendar_path: Option<&str>,
) -> Result<SimulationOutput, ProjectSimulationError> {
    let project = load_project_from_yaml_file(path)?;
    let calendar = load_team_calendar_if_provided(calendar_path)?;
    let mut output = simulate_project(&project, iterations, start_date, calendar)?;
    output.report.data_source = data_source_name(path);
    Ok(output)
}

pub fn simulate_project(
    project: &Project,
    iterations: usize,
    start_date: NaiveDate,
    calendar: TeamCalendar,
) -> Result<SimulationOutput, ProjectSimulationError> {
    if iterations == 0 {
        return Err(ProjectSimulationError::InvalidIterations);
    }
    if project.work_packages.is_empty() {
        return Err(ProjectSimulationError::EmptyProject);
    }

    let velocity = calculate_project_velocity(project, &calendar)?;

    let mut rng = rand::thread_rng();
    let mut sampler = BetaPertSampler::new(&mut rng);
    let output = run_simulation(
        project,
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
    velocity: Option<f32>,
    iterations: usize,
    start_date: chrono::NaiveDate,
    sampler: &mut R,
    calendar: &TeamCalendar,
) -> Result<SimulationOutput, ProjectSimulationError> {
    let mut samples_by_id: HashMap<String, Vec<WorkItemSample>> = HashMap::new();
    let mut project_end_dates = Vec::with_capacity(iterations);
    let calendar_option = if project.has_story_points() {
        Some(calendar)
    } else {
        None
    };

    for _ in 0..iterations {
        let network_nodes = build_network_nodes(&project, velocity, sampler)?;

        let result_nodes = critical_path_method(network_nodes, start_date, calendar_option)?;

        let project_end_date = result_nodes
            .iter()
            .map(|node| node.earliest_finish)
            .max()
            .unwrap_or(start_date);
        project_end_dates.push(project_end_date);

        for result_node in result_nodes {
            samples_by_id
                .entry(result_node.id.clone())
                .or_insert_with(|| Vec::with_capacity(iterations))
                .push(WorkItemSample {
                    end_date: result_node.earliest_finish,
                });
        }
    }

    let work_packages = project
        .work_packages
        .iter()
        .map(|issue| {
            let id = issue
                .issue_id
                .as_ref()
                .map(|issue_id| issue_id.id.clone())
                .unwrap_or_default();
            WorkPackageSimulation {
                id,
                is_milestone: issue.is_milestone(),
                percentiles: percentiles_from_samples(
                    samples_by_id
                        .get(issue.issue_id.as_ref().unwrap().id.as_str())
                        .map(Vec::as_slice)
                        .unwrap_or(&[]),
                    start_date,
                ),
            }
        })
        .collect();

    project_end_dates.sort();
    let report = SimulationReport {
        data_source: String::new(),
        start_date,
        velocity,
        iterations,
        simulated_items: project.work_packages.len(),
        p0: to_simulation_percentile(&project_end_dates, 0.0, start_date),
        p15: to_simulation_percentile(&project_end_dates, 15.0, start_date),
        p50: to_simulation_percentile(&project_end_dates, 50.0, start_date),
        p85: to_simulation_percentile(&project_end_dates, 85.0, start_date),
        p100: to_simulation_percentile(&project_end_dates, 100.0, start_date),
        work_packages: Some(work_packages),
    };

    let results = project_end_dates
        .iter()
        .map(|date| calculate_days(start_date, *date))
        .collect();

    Ok(SimulationOutput { report, results })
}

fn percentiles_from_samples(
    samples: &[WorkItemSample],
    start_date: chrono::NaiveDate,
) -> WorkPackagePercentiles {
    let mut sorted_end_dates = samples.iter().map(|s| s.end_date).collect::<Vec<_>>();
    sorted_end_dates.sort();

    WorkPackagePercentiles {
        p0: to_simulation_percentile(&sorted_end_dates, 0.0, start_date),
        p15: to_simulation_percentile(&sorted_end_dates, 15.0, start_date),
        p50: to_simulation_percentile(&sorted_end_dates, 50.0, start_date),
        p85: to_simulation_percentile(&sorted_end_dates, 85.0, start_date),
        p100: to_simulation_percentile(&sorted_end_dates, 100.0, start_date),
    }
}

fn to_simulation_percentile(
    sorted_end_dates: &[chrono::NaiveDate],
    percentile: f64,
    start_date: chrono::NaiveDate,
) -> SimulationPercentile {
    let end_date =
        percentiles::get_percentile_value(sorted_end_dates, percentile).unwrap_or(start_date);
    let days = calculate_days(start_date, end_date);
    SimulationPercentile { days, end_date }
}

fn calculate_days(start_date: chrono::NaiveDate, end_date: chrono::NaiveDate) -> f32 {
    (end_date - start_date).num_days().max(0) as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::issue::IssueId;
    use crate::test_support::{MockSampler, build_in_progress_story_point_issue};
    use crate::test_support::{
        build_constant_three_point_issue, build_done_issue, build_done_issue_with_deps,
        build_story_point_issue, build_story_point_issue_with_start_date,
        create_calendar_without_any_free_days, on_date,
    };
    use chrono::NaiveDate;
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

        let error = simulate_project(&project, 10, on_date(2026, 1, 1), calendar).unwrap_err();
        assert!(matches!(
            error,
            ProjectSimulationError::CriticalPathMethod(CriticalPathMethodError::CycleDetected)
        ));
    }

    #[test]
    fn simulation_handles_different_status_values_correctly() {
        let simulation_start = on_date(2026, 2, 16); // Monday

        let mut sampler = MockSampler;
        let project = Project {
            name: "Dependent Project".to_string(),
            work_packages: vec![
                build_done_issue_with_deps(
                    "SP-0",
                    None,
                    8.0,
                    on_date(2026, 1, 5),
                    on_date(2026, 1, 8),
                ), // velocity of 8 points/4 days
                build_done_issue_with_deps(
                    "SP-1",
                    Some(&["SP-0"]),
                    8.0,
                    on_date(2026, 1, 9),
                    on_date(2026, 1, 14),
                ),
                build_done_issue_with_deps(
                    "SP-2",
                    Some(&["SP-1"]),
                    8.0,
                    on_date(2026, 1, 15),
                    on_date(2026, 1, 20),
                ),
                build_story_point_issue_with_start_date("SP-3", 8.0, simulation_start, &["SP-2"]),
                build_story_point_issue("SP-4", 8.0, &["SP-3"]),
                build_story_point_issue("SP-5", 8.0, &["SP-4"]),
            ],
        };
        let calendar = TeamCalendar::new(); // default calendar which assumes weekends to be free

        let ignored_simulation_start = on_date(2000, 1, 1);
        let output = run_simulation(
            &project,
            calculate_project_velocity(&project, &calendar).unwrap(),
            1,
            ignored_simulation_start,
            &mut sampler,
            &calendar,
        )
        .unwrap();

        let velocity = output.report.velocity.unwrap();
        assert_eq!(velocity, 2.0); // 8 points / 4 days = 2 points/day

        let work_packages = output.report.work_packages.unwrap();
        let wp0 = work_packages.iter().find(|wp| wp.id == "SP-0").unwrap();
        let wp1 = work_packages.iter().find(|wp| wp.id == "SP-1").unwrap();
        let wp2 = work_packages.iter().find(|wp| wp.id == "SP-2").unwrap();
        let wp3 = work_packages.iter().find(|wp| wp.id == "SP-3").unwrap();
        let wp4 = work_packages.iter().find(|wp| wp.id == "SP-4").unwrap();
        let wp5 = work_packages.iter().find(|wp| wp.id == "SP-5").unwrap();

        assert_eq!(wp0.percentiles.p0.end_date, on_date(2026, 1, 8));
        assert_eq!(wp1.percentiles.p0.end_date, on_date(2026, 1, 14));
        assert_eq!(wp2.percentiles.p0.end_date, on_date(2026, 1, 20));
        assert_eq!(wp3.percentiles.p0.end_date, on_date(2026, 2, 20));
        assert_eq!(wp4.percentiles.p0.end_date, on_date(2026, 2, 26));
        assert_eq!(wp5.percentiles.p0.end_date, on_date(2026, 3, 4));
    }

    #[test]
    fn simulation_handles_in_progress_status_correctly() {
        let feb_sixteen = on_date(2026, 2, 16); // Monday

        let mut sampler = MockSampler;
        let project = Project {
            name: "Dependent Project".to_string(),
            work_packages: vec![
                build_done_issue_with_deps(
                    "SP-0",
                    None,
                    8.0,
                    on_date(2026, 1, 5),
                    on_date(2026, 1, 8),
                ),
                build_in_progress_story_point_issue("SP-1", 8.0, feb_sixteen, &["SP-0"]),
                build_story_point_issue("SP-2", 8.0, &["SP-1"]),
            ],
        };
        let calendar = TeamCalendar::new(); // default calendar which assumes weekends to be free

        let ignored_simulation_start = on_date(2000, 1, 1);
        let output = run_simulation(
            &project,
            calculate_project_velocity(&project, &calendar).unwrap(),
            1,
            ignored_simulation_start,
            &mut sampler,
            &calendar,
        )
        .unwrap();

        let velocity = output.report.velocity.unwrap();
        assert_eq!(velocity, 2.0); // 8 points / 4 days = 2 points/day

        let work_packages = output.report.work_packages.unwrap();
        let wp0 = work_packages.iter().find(|wp| wp.id == "SP-0").unwrap();
        let wp1 = work_packages.iter().find(|wp| wp.id == "SP-1").unwrap();
        let wp2 = work_packages.iter().find(|wp| wp.id == "SP-2").unwrap();

        assert_eq!(wp0.percentiles.p0.end_date, on_date(2026, 1, 8));
        assert_eq!(wp1.percentiles.p0.end_date, on_date(2026, 2, 20));
        assert_eq!(wp2.percentiles.p0.end_date, on_date(2026, 2, 26));
    }

    #[test]
    fn simulate_project_with_dependencies_matches_critical_path() {
        let base = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let done_issue = build_done_issue("DONE-1", 100.0, base, base + chrono::Duration::days(1));

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
        //    WP0      WP1        SP-1
        //     |        |           |
        //     |        |           |
        //     |    +---+----+    SP-2
        //     |    |        |
        //     +---WP2      WP3
        //     |    |        |
        //     +----+--+-----+
        //            |
        //           FIN
        for (_idx, (wp0, wp1, wp2, wp3, expected)) in test_cases.into_iter().enumerate() {
            let mut sampler = MockSampler;
            let project = Project {
                name: "Dependent Project".to_string(),
                work_packages: vec![
                    done_issue.clone(),
                    build_story_point_issue("SP-1", 1.0, &[]),
                    build_story_point_issue("SP-2", 1.0, &["SP-1"]),
                    build_constant_three_point_issue("WP0", wp0, &[]),
                    build_constant_three_point_issue("WP1", wp1, &[]),
                    build_constant_three_point_issue("WP2", wp2, &["WP0", "WP1"]),
                    build_constant_three_point_issue("WP3", wp3, &["WP1"]),
                    build_constant_three_point_issue("FIN", 0.0, &["WP0", "WP2", "WP3"]),
                ],
            };
            let calendar = create_calendar_without_any_free_days();

            let output = run_simulation(
                &project,
                calculate_project_velocity(&project, &calendar).unwrap(),
                25,
                base,
                &mut sampler,
                &calendar,
            )
            .unwrap();

            assert_eq!(output.report.p85.days, expected);
            assert_eq!(output.report.iterations, 25);
            assert!(output.report.velocity.is_some());
        }
    }

    #[test]
    fn work_package_percentiles_contain_end_dates() {
        let mut sampler = MockSampler;
        let project = Project {
            name: "Dependent Project".to_string(),
            work_packages: vec![
                build_constant_three_point_issue("WP0", 2.0, &[]),
                build_constant_three_point_issue("WP1", 4.0, &[]),
                build_constant_three_point_issue("WP2", 3.0, &["WP0", "WP1"]),
                build_constant_three_point_issue("FIN", 0.0, &["WP2"]),
            ],
        };
        let calendar = create_calendar_without_any_free_days();

        let project_start_date = on_date(2026, 1, 1);

        let output = run_simulation(
            &project,
            None,
            25,
            project_start_date,
            &mut sampler,
            &calendar,
        );

        let work_packages = output.unwrap().report.work_packages.unwrap();

        let wp0 = work_packages.iter().find(|wp| wp.id == "WP0").unwrap();
        let wp1 = work_packages.iter().find(|wp| wp.id == "WP1").unwrap();
        let wp2 = work_packages.iter().find(|wp| wp.id == "WP2").unwrap();
        let fin = work_packages.iter().find(|wp| wp.id == "FIN").unwrap();

        assert_eq!(wp0.percentiles.p0.days, 2.0);
        assert_eq!(
            wp0.percentiles.p0.end_date,
            project_start_date + chrono::Duration::days(2)
        );

        let wp1_end_date = project_start_date + chrono::Duration::days(4);

        assert_eq!(wp1.percentiles.p0.days, 4.0);
        assert_eq!(wp1.percentiles.p0.end_date, wp1_end_date);

        let wp2_end_date = wp1_end_date + chrono::Duration::days(3);

        assert_eq!(wp2.percentiles.p0.days, 7.0);
        assert_eq!(wp2.percentiles.p0.end_date, wp2_end_date);

        assert_eq!(fin.percentiles.p0.days, 7.0);
        assert_eq!(fin.percentiles.p0.end_date, wp2_end_date);
    }

    #[test]
    fn project_simulation_takes_calendar_into_account() {
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
            calculate_project_velocity(&project, &calendar).unwrap(),
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

        let output = simulate_project_from_yaml_file(
            input_path.to_str().unwrap(),
            5,
            on_date(2026, 1, 1),
            None,
        )
        .unwrap();

        assert_eq!(
            output.report.data_source,
            input_path.file_name().unwrap().to_str().unwrap()
        );
        assert_eq!(output.report.iterations, 5);
        assert_eq!(output.report.velocity, None);
    }
}
