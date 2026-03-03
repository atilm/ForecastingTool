use std::io;

use thiserror::Error;

use crate::domain::calendar::TeamCalendar;
use crate::domain::project::Project;
use crate::services::estimate_duration::{
    EstimateDurationError, WorkPackageDuration, compute_expected_durations,
};
use crate::services::project_simulation::beta_pert_sampler::PertExpectedValueSampler;
use crate::services::project_simulation::critical_path_method::CriticalPathMethodError;
use crate::services::project_simulation::critical_path_method::ResultNode;
use crate::services::project_simulation::critical_path_method::critical_path_method;
use crate::services::project_simulation::network_nodes::NetworkNodesError;
use crate::services::project_simulation::network_nodes::build_network_nodes;
use crate::services::project_simulation::velocity_calculation::VelocityCalculationError;
use crate::services::project_simulation::velocity_calculation::calculate_project_velocity;
use crate::services::project_yaml::{ProjectYamlError, load_project_from_yaml_file};

#[derive(Error, Debug)]
pub enum EstimateGanttError {
    #[error("failed to load project: {0}")]
    ProjectLoad(#[from] ProjectYamlError),
    #[error("failed to compute durations: {0}")]
    Duration(#[from] EstimateDurationError),
    #[error("failed to write output: {0}")]
    Io(#[from] io::Error),
    #[error("failed to calculate velocity: {0}")]
    Velocity(#[from] VelocityCalculationError),
    #[error("failed to build network nodes: {0}")]
    NetworkNodes(#[from] NetworkNodesError),
    #[error("failed to perform critical path method analysis: {0}")]
    CriticalPathMethod(#[from] CriticalPathMethodError),
}

/// Loads a project YAML, computes expected durations, and writes
/// the Mermaid Gantt diagram to the output file.
pub fn write_pert_gantt_markdown(
    input_path: &str,
    output_path: &str,
) -> Result<(), EstimateGanttError> {
    let project = load_project_from_yaml_file(input_path)?;

    // Todo: parse team calender from passed path
    let calendar = TeamCalendar::new();
    // instantiate start_date to today
    let start_date = chrono::Utc::now().date_naive();

    let velocity = if project.has_story_points() {
        Some(calculate_project_velocity(&project, &calendar)?)
    } else {
        None
    };

    let mut expected_value_sampler = PertExpectedValueSampler;

    let network_nodes = build_network_nodes(&project, velocity, &mut expected_value_sampler)?;
    let result_nodes = critical_path_method(network_nodes, start_date, Some(&calendar))?;

    let markdown = generate_gantt_markdown(&result_nodes, &project);

    // let durations = compute_expected_durations(&project)?;
    // let markdown = generate_estimate_gantt(&project, &durations);
    std::fs::write(output_path, markdown)?;
    Ok(())
}

pub fn generate_gantt_markdown(result_nodes: &[ResultNode], project: &Project) -> String {
    "".to_string() // Placeholder implementation
}

/// Generates a Mermaid Gantt diagram from a project and its expected durations.
pub fn generate_estimate_gantt(project: &Project, durations: &[WorkPackageDuration]) -> String {
    let mut duration_map = std::collections::HashMap::new();
    for d in durations {
        duration_map.insert(d.id.as_str(), d);
    }

    let mut lines = Vec::new();
    lines.push(format!("# {} Gantt Diagram", project.name));
    lines.push("```mermaid".to_string());
    lines.push("gantt".to_string());
    lines.push("    dateFormat YYYY-MM-DD".to_string());

    for issue in &project.work_packages {
        let id = issue.issue_id.as_ref().map(|i| i.id.as_str()).unwrap_or("");
        let name = issue.summary.as_deref().unwrap_or(id);

        let wp_duration = duration_map.get(id);
        let days = wp_duration.map(|d| d.expected_days).unwrap_or(0.0);
        let days_rounded = days.ceil().max(0.0) as u32;

        let status_str = match issue.status.as_ref() {
            Some(crate::domain::issue_status::IssueStatus::Done) => "done, ",
            Some(crate::domain::issue_status::IssueStatus::InProgress) => "active, ",
            _ => "",
        };

        if days_rounded == 0 {
            let after_str = make_after_clause(issue);
            lines.push(format!(
                "    {name} :{status_str}milestone, {id}, {after_str}0d"
            ));
        } else {
            let after_str = make_after_clause(issue);
            lines.push(format!(
                "    {name} :{status_str}{id}, {after_str}{days_rounded}d"
            ));
        }
    }

    lines.push("```".to_string());
    lines.join("\n")
}

fn make_after_clause(issue: &crate::domain::issue::Issue) -> String {
    match &issue.dependencies {
        Some(deps) if !deps.is_empty() => {
            let dep_ids: Vec<&str> = deps.iter().map(|d| d.id.as_str()).collect();
            format!("after {}, ", dep_ids.join(" "))
        }
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::estimate::{Estimate, ThreePointEstimate};
    use crate::domain::issue::{Issue, IssueId};
    use crate::domain::issue_status::IssueStatus;
    use crate::domain::project::Project;
    use crate::services::estimate_duration::WorkPackageDuration;

    fn build_issue(id: &str, summary: &str, deps: &[&str]) -> Issue {
        let mut issue = Issue::new();
        issue.issue_id = Some(IssueId { id: id.to_string() });
        issue.summary = Some(summary.to_string());
        issue.estimate = Some(Estimate::ThreePoint(ThreePointEstimate {
            optimistic: Some(1.0),
            most_likely: Some(3.0),
            pessimistic: Some(5.0),
        }));
        issue.dependencies = if deps.is_empty() {
            None
        } else {
            Some(deps.iter().map(|d| IssueId { id: d.to_string() }).collect())
        };
        issue
    }

    fn build_duration(id: &str, days: f32) -> WorkPackageDuration {
        WorkPackageDuration {
            id: id.to_string(),
            summary: Some(format!("Task {id}")),
            expected_days: days,
        }
    }

    #[test]
    fn generates_gantt_with_single_task() {
        let project = Project {
            name: "TestProject".to_string(),
            work_packages: vec![build_issue("WP-1", "Design", &[])],
        };
        let durations = vec![build_duration("WP-1", 5.0)];

        let result = generate_estimate_gantt(&project, &durations);

        assert!(result.contains("# TestProject Gantt Diagram"));
        assert!(result.contains("```mermaid"));
        assert!(result.contains("gantt"));
        assert!(result.contains("Design :WP-1, 5d"));
    }

    #[test]
    fn generates_gantt_with_dependencies() {
        let project = Project {
            name: "Test".to_string(),
            work_packages: vec![
                build_issue("A", "Task A", &[]),
                build_issue("B", "Task B", &["A"]),
            ],
        };
        let durations = vec![build_duration("A", 3.0), build_duration("B", 4.0)];

        let result = generate_estimate_gantt(&project, &durations);

        assert!(result.contains("Task A :A, 3d"));
        assert!(result.contains("Task B :B, after A, 4d"));
    }

    #[test]
    fn generates_milestone_for_zero_duration() {
        let project = Project {
            name: "Test".to_string(),
            work_packages: vec![build_issue("M", "Milestone", &["A"])],
        };
        let durations = vec![build_duration("M", 0.0)];

        let result = generate_estimate_gantt(&project, &durations);

        assert!(result.contains("Milestone :milestone, M, after A, 0d"));
    }

    #[test]
    fn generates_gantt_with_status() {
        let mut issue = build_issue("WP-1", "Done Work", &[]);
        issue.status = Some(IssueStatus::Done);
        let mut issue2 = build_issue("WP-2", "Active Work", &["WP-1"]);
        issue2.status = Some(IssueStatus::InProgress);
        let project = Project {
            name: "StatusTest".to_string(),
            work_packages: vec![issue, issue2],
        };
        let durations = vec![build_duration("WP-1", 2.0), build_duration("WP-2", 3.0)];

        let result = generate_estimate_gantt(&project, &durations);

        assert!(result.contains("Done Work :done, WP-1, 2d"));
        assert!(result.contains("Active Work :active, WP-2, after WP-1, 3d"));
    }

    #[test]
    fn generates_gantt_with_multiple_dependencies() {
        let project = Project {
            name: "Test".to_string(),
            work_packages: vec![
                build_issue("A", "Task A", &[]),
                build_issue("B", "Task B", &[]),
                build_issue("C", "Task C", &["A", "B"]),
            ],
        };
        let durations = vec![
            build_duration("A", 2.0),
            build_duration("B", 3.0),
            build_duration("C", 4.0),
        ];

        let result = generate_estimate_gantt(&project, &durations);

        assert!(result.contains("Task C :C, after A B, 4d"));
    }

    #[test]
    fn rounds_fractional_days_up() {
        let project = Project {
            name: "Test".to_string(),
            work_packages: vec![build_issue("A", "Fractional", &[])],
        };
        let durations = vec![build_duration("A", 3.2)];

        let result = generate_estimate_gantt(&project, &durations);

        assert!(result.contains("Fractional :A, 4d"));
    }
}
