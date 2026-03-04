use std::io;

use thiserror::Error;

use crate::domain::project::Project;
use crate::services::project_simulation::beta_pert_sampler::PertExpectedValueSampler;
use crate::services::project_simulation::critical_path_method::CriticalPathMethodError;
use crate::services::project_simulation::critical_path_method::ResultNode;
use crate::services::project_simulation::critical_path_method::critical_path_method;
use crate::services::project_simulation::network_nodes::NetworkNodesError;
use crate::services::project_simulation::network_nodes::build_network_nodes;
use crate::services::project_simulation::velocity_calculation::VelocityCalculationError;
use crate::services::project_simulation::velocity_calculation::calculate_project_velocity;
use crate::services::project_yaml::{ProjectYamlError, load_project_from_yaml_file};
use crate::services::team_calendar_yaml::{TeamCalendarYamlError, load_team_calendar_if_provided};
use chrono::NaiveDate;

#[derive(Error, Debug)]
pub enum EstimateGanttError {
    #[error("failed to load project: {0}")]
    ProjectLoad(#[from] ProjectYamlError),
    #[error("failed to write output: {0}")]
    Io(#[from] io::Error),
    #[error("failed to calculate velocity: {0}")]
    Velocity(#[from] VelocityCalculationError),
    #[error("failed to build network nodes: {0}")]
    NetworkNodes(#[from] NetworkNodesError),
    #[error("failed to perform critical path method analysis: {0}")]
    CriticalPathMethod(#[from] CriticalPathMethodError),
    #[error("failed to load team calendar: {0}")]
    TeamCalendar(#[from] TeamCalendarYamlError),
}

/// Loads a project YAML, computes expected durations, and writes
/// the Mermaid Gantt diagram to the output file.
pub fn write_pert_gantt_markdown(
    input_path: &str,
    output_path: &str,
    start_date: NaiveDate,
    calendar_path: Option<&str>,
) -> Result<(), EstimateGanttError> {
    let project = load_project_from_yaml_file(input_path)?;
    let calendar = load_team_calendar_if_provided(calendar_path)?;

    let velocity = calculate_project_velocity(&project, &calendar)?;

    let mut expected_value_sampler = PertExpectedValueSampler;

    let network_nodes = build_network_nodes(&project, velocity, &mut expected_value_sampler)?;
    let result_nodes = critical_path_method(network_nodes, start_date, Some(&calendar))?;

    let markdown = generate_gantt_markdown(&result_nodes, &project);

    std::fs::write(output_path, markdown)?;
    Ok(())
}

/// Generates a Mermaid Gantt diagram from CPM result nodes and a project.
///
/// Work packages are named by combining their id and summary from the project.
/// Uses earliest_start and earliest_finish dates for scheduling. Marks critical
/// path nodes with `crit` and zero-duration nodes as milestones.
pub fn generate_gantt_markdown(result_nodes: &[ResultNode], project: &Project) -> String {
    let name_map: std::collections::HashMap<&str, &str> = project
        .work_packages
        .iter()
        .filter_map(|wp| {
            let id = wp.issue_id.as_ref()?.id.as_str();
            let summary = wp.summary.as_deref().unwrap_or(id);
            Some((id, summary))
        })
        .collect();

    let mut lines = Vec::new();
    lines.push(format!("# {} Gantt Diagram", project.name));
    lines.push("```mermaid".to_string());
    lines.push("gantt".to_string());
    lines.push("    dateFormat YYYY-MM-DD".to_string());

    for node in result_nodes {
        let id_str = node.id.as_str();
        let summary = name_map.get(id_str).unwrap_or(&id_str);
        let label = format!("{} {}", node.id, summary);
        let start_str = node.earliest_start.format("%Y-%m-%d");
        let end_str = node.earliest_finish.format("%Y-%m-%d");

        let crit_str = if node.is_critical() { "crit, " } else { "" };

        if node.is_milestone() {
            lines.push(format!(
                "    {label} :{crit_str}milestone, {id}, {start_str}, 0d",
                id = node.id,
            ));
        } else {
            lines.push(format!(
                "    {label} :{crit_str}{id}, {start_str}, {end_str}",
                id = node.id,
            ));
        }
    }

    lines.push("```".to_string());
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::issue::{Issue, IssueId};
    use crate::domain::project::Project;
    use crate::services::project_simulation::critical_path_method::ResultNode;
    use crate::test_support::on_date;

    fn build_issue(id: &str, summary: &str) -> Issue {
        let mut issue = Issue::new();
        issue.issue_id = Some(IssueId {
            id: id.to_string(),
        });
        issue.summary = Some(summary.to_string());
        issue
    }

    fn build_result_node(
        id: &str,
        earliest_start: chrono::NaiveDate,
        earliest_finish: chrono::NaiveDate,
        total_float: f32,
    ) -> ResultNode {
        ResultNode {
            id: id.to_string(),
            earliest_start,
            latest_start: earliest_start,
            earliest_finish,
            latest_finish: earliest_finish,
            free_float: 0.0,
            total_float,
        }
    }

    #[test]
    fn generates_gantt_with_single_task() {
        let project = Project {
            name: "TestProject".to_string(),
            work_packages: vec![build_issue("WP1", "Design")],
        };
        let nodes = vec![build_result_node("WP1", on_date(2026, 1, 1), on_date(2026, 1, 6), 0.0)];

        let result = generate_gantt_markdown(&nodes, &project);

        assert!(result.contains("# TestProject Gantt Diagram"));
        assert!(result.contains("```mermaid"));
        assert!(result.contains("gantt"));
        assert!(result.contains("WP1 Design :crit, WP1, 2026-01-01, 2026-01-06"));
    }

    #[test]
    fn generates_milestone_for_zero_duration() {
        let project = Project {
            name: "Test".to_string(),
            work_packages: vec![build_issue("M", "Milestone")],
        };
        let nodes = vec![build_result_node("M", on_date(2026, 1, 5), on_date(2026, 1, 5), 0.0)];

        let result = generate_gantt_markdown(&nodes, &project);

        assert!(result.contains("M Milestone :crit, milestone, M, 2026-01-05, 0d"));
    }

    #[test]
    fn marks_critical_path_nodes() {
        let project = Project {
            name: "Test".to_string(),
            work_packages: vec![
                build_issue("A", "Task A"),
                build_issue("B", "Task B"),
            ],
        };
        let nodes = vec![
            build_result_node("A", on_date(2026, 1, 1), on_date(2026, 1, 4), 0.0),
            build_result_node("B", on_date(2026, 1, 1), on_date(2026, 1, 3), 2.0),
        ];

        let result = generate_gantt_markdown(&nodes, &project);

        assert!(result.contains("Task A :crit, A, 2026-01-01, 2026-01-04"));
        assert!(result.contains("Task B :B, 2026-01-01, 2026-01-03"));
        assert!(!result.contains("Task B :crit"));
    }

    #[test]
    fn non_critical_milestone_is_not_marked_crit() {
        let project = Project {
            name: "Test".to_string(),
            work_packages: vec![build_issue("M", "Gate")],
        };
        let nodes = vec![build_result_node("M", on_date(2026, 1, 5), on_date(2026, 1, 5), 3.0)];

        let result = generate_gantt_markdown(&nodes, &project);

        assert!(result.contains("M Gate :milestone, M, 2026-01-05, 0d"));
        assert!(!result.contains("crit"));
    }

    #[test]
    fn uses_id_as_label_when_summary_missing() {
        let mut issue = Issue::new();
        issue.issue_id = Some(IssueId {
            id: "X1".to_string(),
        });
        // No summary set
        let project = Project {
            name: "Test".to_string(),
            work_packages: vec![issue],
        };
        let nodes = vec![build_result_node("X1", on_date(2026, 1, 1), on_date(2026, 1, 3), 0.0)];

        let result = generate_gantt_markdown(&nodes, &project);

        assert!(result.contains("X1 X1 :crit, X1, 2026-01-01, 2026-01-03"));
    }

    #[test]
    fn multiple_tasks_with_mixed_criticality() {
        let project = Project {
            name: "Mixed".to_string(),
            work_packages: vec![
                build_issue("A", "Alpha"),
                build_issue("B", "Beta"),
                build_issue("C", "Gamma"),
                build_issue("FIN", "Finish"),
            ],
        };
        let nodes = vec![
            build_result_node("A", on_date(2026, 1, 1), on_date(2026, 1, 6), 0.0),
            build_result_node("B", on_date(2026, 1, 1), on_date(2026, 1, 3), 3.0),
            build_result_node("C", on_date(2026, 1, 6), on_date(2026, 1, 10), 0.0),
            build_result_node("FIN", on_date(2026, 1, 10), on_date(2026, 1, 10), 0.0),
        ];

        let result = generate_gantt_markdown(&nodes, &project);

        assert!(result.contains("A Alpha :crit, A, 2026-01-01, 2026-01-06"));
        assert!(result.contains("B Beta :B, 2026-01-01, 2026-01-03"));
        assert!(result.contains("C Gamma :crit, C, 2026-01-06, 2026-01-10"));
        assert!(result.contains("FIN Finish :crit, milestone, FIN, 2026-01-10, 0d"));
    }
}
