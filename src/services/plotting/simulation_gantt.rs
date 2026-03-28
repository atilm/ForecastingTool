use std::collections::HashMap;
use std::io;

use chrono::NaiveDate;
use thiserror::Error;

use crate::domain::project::Project;
use crate::services::parsing::project_yaml::{ProjectYamlError, load_project_from_yaml_file};
use crate::services::parsing::simulation_report_yaml::{ReportParseError, load_simulation_report_from_file};
use crate::services::simulation_types::{SimulationReport, WorkPackageSimulation};

#[derive(Error, Debug)]
pub enum SimulationGanttError {
    #[error("failed to load project: {0}")]
    ProjectLoad(#[from] ProjectYamlError),
    #[error("failed to load simulation report: {0}")]
    ReportLoad(#[from] ReportParseError),
    #[error("failed to write output: {0}")]
    Io(#[from] io::Error),
}

/// Loads a project YAML and a simulation report YAML, then writes a Mermaid
/// Gantt diagram to the output file showing p85 scheduled work packages.
pub fn write_simulation_gantt_markdown(
    project_path: &str,
    report_path: &str,
    output_path: &str,
) -> Result<(), SimulationGanttError> {
    let project = load_project_from_yaml_file(project_path)?;
    let report = load_simulation_report_from_file(report_path)?;
    let markdown = generate_simulation_gantt_markdown(&project, &report);
    std::fs::write(output_path, markdown)?;
    Ok(())
}

/// Generates a Mermaid Gantt diagram from a project and a simulation report.
///
/// Each `WorkPackageSimulation` in the report becomes a task. The start date
/// is the latest p85 end_date among the work package's dependencies (or the
/// report's start_date if there are none), and the end date is the work
/// package's own p85 end_date. Milestones are rendered as Mermaid milestones.
pub fn generate_simulation_gantt_markdown(project: &Project, report: &SimulationReport) -> String {
    let wp_sim_by_id: HashMap<&str, &WorkPackageSimulation> = report
        .work_packages
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .map(|wp| (wp.id.as_str(), wp))
        .collect();

    let dep_start_date =
        |wp_sim: &WorkPackageSimulation| -> NaiveDate { compute_start_date(wp_sim, project, &wp_sim_by_id, report.start_date) };

    let work_packages = report.work_packages.as_deref().unwrap_or(&[]);

    let summary_by_id: HashMap<&str, &str> = project
        .work_packages
        .iter()
        .filter_map(|wp| {
            let id = wp.issue_id.as_ref()?.id.as_str();
            let summary = wp.summary.as_deref().unwrap_or(id);
            Some((id, summary))
        })
        .collect();

    let mut lines = Vec::new();
    lines.push(format!("# {} Simulation Gantt Diagram", project.name));
    lines.push("```mermaid".to_string());
    lines.push("gantt".to_string());
    lines.push("    dateFormat YYYY-MM-DD".to_string());

    for wp_sim in work_packages {
        let id = wp_sim.id.as_str();
        let summary = summary_by_id.get(id).copied().unwrap_or(id);
        let label = format!("{} {}", id, summary);
        let start = dep_start_date(wp_sim);
        let end = wp_sim.percentiles.p85.end_date;

        if wp_sim.is_milestone {
            lines.push(format!(
                "    {label} :milestone, {id}, {end}, 0d",
                end = end.format("%Y-%m-%d"),
            ));
        } else {
            lines.push(format!(
                "    {label} :{id}, {start}, {end}",
                start = start.format("%Y-%m-%d"),
                end = end.format("%Y-%m-%d"),
            ));
        }
    }

    lines.push("```".to_string());
    lines.join("\n")
}

fn compute_start_date(
    wp_sim: &WorkPackageSimulation,
    project: &Project,
    wp_sim_by_id: &HashMap<&str, &WorkPackageSimulation>,
    default_date: NaiveDate,
) -> NaiveDate {
    let deps = project
        .work_packages
        .iter()
        .find(|wp| wp.issue_id.as_ref().map(|id| id.id.as_str()) == Some(wp_sim.id.as_str()))
        .and_then(|wp| wp.dependencies.as_deref());

    let Some(deps) = deps else {
        return default_date;
    };

    deps.iter()
        .filter_map(|dep_id| wp_sim_by_id.get(dep_id.id.as_str()))
        .map(|dep_wp| dep_wp.percentiles.p85.end_date)
        .max()
        .unwrap_or(default_date)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::issue::{Issue, IssueId};
    use crate::services::simulation_types::{
        SimulationPercentile, WorkPackagePercentiles, WorkPackageSimulation,
    };
    use chrono::NaiveDate;

    fn date(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    fn percentile(end: &str) -> SimulationPercentile {
        SimulationPercentile {
            days: 0.0,
            end_date: date(end),
        }
    }

    fn wp_percentiles(end: &str) -> WorkPackagePercentiles {
        WorkPackagePercentiles {
            p0: percentile(end),
            p15: percentile(end),
            p50: percentile(end),
            p85: percentile(end),
            p100: percentile(end),
        }
    }

    fn build_report(
        start: &str,
        work_packages: Vec<WorkPackageSimulation>,
    ) -> SimulationReport {
        SimulationReport {
            data_source: "test".to_string(),
            start_date: date(start),
            velocity: None,
            iterations: 100,
            simulated_items: work_packages.len(),
            p0: percentile(start),
            p15: percentile(start),
            p50: percentile(start),
            p85: percentile(start),
            p100: percentile(start),
            work_packages: Some(work_packages),
        }
    }

    fn build_issue(id: &str, summary: &str, deps: Option<Vec<&str>>) -> Issue {
        let mut issue = Issue::new();
        issue.issue_id = Some(IssueId { id: id.to_string() });
        issue.summary = Some(summary.to_string());
        issue.dependencies = deps.map(|ds| {
            ds.into_iter()
                .map(|d| IssueId { id: d.to_string() })
                .collect()
        });
        issue
    }

    fn build_project(name: &str, issues: Vec<Issue>) -> Project {
        Project {
            name: name.to_string(),
            work_packages: issues,
        }
    }

    #[test]
    fn work_package_without_deps_uses_report_start_date() {
        let report = build_report(
            "2026-01-01",
            vec![WorkPackageSimulation {
                id: "WP1".to_string(),
                is_milestone: false,
                percentiles: wp_percentiles("2026-01-10"),
            }],
        );
        let project = build_project(
            "Demo",
            vec![build_issue("WP1", "Design", None)],
        );

        let md = generate_simulation_gantt_markdown(&project, &report);

        assert!(md.contains("WP1 Design"));
        assert!(md.contains(":WP1, 2026-01-01, 2026-01-10"));
    }

    #[test]
    fn work_package_start_is_latest_dep_p85_end_date() {
        let report = build_report(
            "2026-01-01",
            vec![
                WorkPackageSimulation {
                    id: "WP1".to_string(),
                    is_milestone: false,
                    percentiles: wp_percentiles("2026-01-10"),
                },
                WorkPackageSimulation {
                    id: "WP2".to_string(),
                    is_milestone: false,
                    percentiles: wp_percentiles("2026-01-20"),
                },
            ],
        );
        let project = build_project(
            "Demo",
            vec![
                build_issue("WP1", "Design", None),
                build_issue("WP2", "Build", Some(vec!["WP1"])),
            ],
        );

        let md = generate_simulation_gantt_markdown(&project, &report);

        // WP2 should start at WP1's p85 end_date (2026-01-10)
        assert!(md.contains(":WP2, 2026-01-10, 2026-01-20"));
    }

    #[test]
    fn start_date_is_max_of_multiple_dependencies() {
        let report = build_report(
            "2026-01-01",
            vec![
                WorkPackageSimulation {
                    id: "WP1".to_string(),
                    is_milestone: false,
                    percentiles: wp_percentiles("2026-01-05"),
                },
                WorkPackageSimulation {
                    id: "WP2".to_string(),
                    is_milestone: false,
                    percentiles: wp_percentiles("2026-01-15"),
                },
                WorkPackageSimulation {
                    id: "WP3".to_string(),
                    is_milestone: false,
                    percentiles: wp_percentiles("2026-01-25"),
                },
            ],
        );
        let project = build_project(
            "Demo",
            vec![
                build_issue("WP1", "Task A", None),
                build_issue("WP2", "Task B", None),
                build_issue("WP3", "Task C", Some(vec!["WP1", "WP2"])),
            ],
        );

        let md = generate_simulation_gantt_markdown(&project, &report);

        // WP3 depends on WP1 (ends 2026-01-05) and WP2 (ends 2026-01-15)
        // Start should be max = 2026-01-15
        assert!(md.contains(":WP3, 2026-01-15, 2026-01-25"));
    }

    #[test]
    fn milestone_rendered_as_mermaid_milestone() {
        let report = build_report(
            "2026-01-01",
            vec![
                WorkPackageSimulation {
                    id: "WP1".to_string(),
                    is_milestone: false,
                    percentiles: wp_percentiles("2026-01-10"),
                },
                WorkPackageSimulation {
                    id: "MS1".to_string(),
                    is_milestone: true,
                    percentiles: wp_percentiles("2026-01-10"),
                },
            ],
        );
        let project = build_project(
            "Demo",
            vec![
                build_issue("WP1", "Design", None),
                build_issue("MS1", "Release", Some(vec!["WP1"])),
            ],
        );

        let md = generate_simulation_gantt_markdown(&project, &report);

        assert!(md.contains(":milestone, MS1, 2026-01-10, 0d"));
        assert!(!md.contains(":MS1, "));
    }

    #[test]
    fn label_uses_id_and_summary_from_project() {
        let report = build_report(
            "2026-01-01",
            vec![WorkPackageSimulation {
                id: "WP1".to_string(),
                is_milestone: false,
                percentiles: wp_percentiles("2026-01-10"),
            }],
        );
        let project = build_project(
            "MyProject",
            vec![build_issue("WP1", "My Summary", None)],
        );

        let md = generate_simulation_gantt_markdown(&project, &report);

        assert!(md.contains("WP1 My Summary"));
        assert!(md.contains("# MyProject Simulation Gantt Diagram"));
    }

    #[test]
    fn dependency_not_in_report_falls_back_to_report_start() {
        let report = build_report(
            "2026-01-01",
            vec![WorkPackageSimulation {
                id: "WP2".to_string(),
                is_milestone: false,
                percentiles: wp_percentiles("2026-01-20"),
            }],
        );
        let project = build_project(
            "Demo",
            vec![
                build_issue("WP1", "External", None),
                build_issue("WP2", "Build", Some(vec!["WP1"])),
            ],
        );

        let md = generate_simulation_gantt_markdown(&project, &report);

        // WP1 not in report, so start falls back to report start_date
        assert!(md.contains(":WP2, 2026-01-01, 2026-01-20"));
    }
}
