use chrono::NaiveDate;
use thiserror::Error;

use crate::domain::project::Project;
use crate::services::simulation_types::{SimulationOutput, WorkPackageSimulation};

#[derive(Error, Debug)]
pub enum GanttDiagramError {
    #[error("missing work package results")]
    MissingWorkPackages,
    #[error("missing work package result for {0}")]
    MissingWorkPackage(String),
}

pub fn generate_gantt_diagram(
    project: &Project,
    simulation: &SimulationOutput,
    start_date: NaiveDate,
    percentile: f32,
) -> Result<String, GanttDiagramError> {
    let work_packages = simulation
        .work_packages
        .as_ref()
        .ok_or(GanttDiagramError::MissingWorkPackages)?;

    let mut map = std::collections::HashMap::new();
    for item in work_packages {
        map.insert(item.id.clone(), item.clone());
    }

    let mut lines = Vec::new();
    lines.push("".to_string());
    lines.push(format!("# {} Timeline", project.name));
    lines.push("```mermaid".to_string());
    lines.push("gantt".to_string());
    lines.push("    dateFormat  DD-MM-YYYY".to_string());

    for issue in &project.work_packages {
        let id = issue.issue_id.as_ref().map(|id| id.id.clone()).unwrap_or_default();
        let name = issue.summary.as_deref().unwrap_or(&id).to_string();
        let wp = map
            .get(&id)
            .ok_or_else(|| GanttDiagramError::MissingWorkPackage(id.clone()))?;
        let end_time = percentile_value(wp, percentile);

        let mut start_time = 0.0_f32;
        if let Some(deps) = issue.dependencies.as_ref() {
            if !deps.is_empty() {
                let mut dep_end_times = Vec::new();
                for dep in deps {
                    if let Some(dep_wp) = map.get(&dep.id) {
                        dep_end_times.push(percentile_value(dep_wp, percentile));
                    }
                }
                if let Some(value) = dep_end_times
                    .into_iter()
                    .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                {
                    start_time = value;
                }
            }
        }

        let start_date_wp = add_days(start_date, start_time);
        let end_date_wp = add_days(start_date, end_time);
        lines.push(format!(
            "    {id} {name} :{id}, {}, {}",
            start_date_wp.format("%d-%m-%Y"),
            end_date_wp.format("%d-%m-%Y")
        ));
    }
    lines.push("```".to_string());

    Ok(lines.join("\n"))
}

fn percentile_value(work_package: &WorkPackageSimulation, percentile: f32) -> f32 {
    if percentile <= 0.0 {
        return work_package.percentiles.p0;
    }
    if percentile <= 50.0 {
        return work_package.percentiles.p50;
    }
    if percentile <= 85.0 {
        return work_package.percentiles.p85;
    }
    work_package.percentiles.p100
}

fn add_days(start_date: NaiveDate, days: f32) -> NaiveDate {
    let days = days.ceil().max(0.0) as i64;
    start_date + chrono::Duration::days(days)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::issue::{Issue, IssueId};
    use crate::services::simulation_types::{
        SimulationOutput,
        SimulationPercentile,
        SimulationReport,
        WorkPackagePercentiles,
        WorkPackageSimulation,
    };

    fn build_issue(id: &str, deps: &[&str]) -> Issue {
        let mut issue = Issue::new();
        issue.issue_id = Some(IssueId { id: id.to_string() });
        issue.summary = Some(format!("Name {id}"));
        issue.dependencies = if deps.is_empty() {
            None
        } else {
            Some(
                deps.iter()
                    .map(|dep| IssueId { id: (*dep).to_string() })
                    .collect(),
            )
        };
        issue
    }

    fn build_simulation_output() -> SimulationOutput {
        SimulationOutput {
            report: SimulationReport {
                start_date: "2026-01-01".to_string(),
                simulated_items: 2,
                p0: SimulationPercentile {
                    days: 0.0,
                    date: "2026-01-01".to_string(),
                },
                p50: SimulationPercentile {
                    days: 0.0,
                    date: "2026-01-01".to_string(),
                },
                p85: SimulationPercentile {
                    days: 0.0,
                    date: "2026-01-01".to_string(),
                },
                p100: SimulationPercentile {
                    days: 0.0,
                    date: "2026-01-01".to_string(),
                },
            },
            results: vec![1.0],
            work_packages: Some(vec![
                WorkPackageSimulation {
                    id: "A".to_string(),
                    percentiles: WorkPackagePercentiles {
                        p0: 1.0,
                        p50: 1.0,
                        p85: 1.0,
                        p100: 1.0,
                    },
                },
                WorkPackageSimulation {
                    id: "B".to_string(),
                    percentiles: WorkPackagePercentiles {
                        p0: 3.0,
                        p50: 3.0,
                        p85: 3.0,
                        p100: 3.0,
                    },
                },
            ]),
        }
    }

    #[test]
    fn generate_gantt_diagram_uses_dependencies() {
        let project = Project {
            name: "Demo".to_string(),
            work_packages: vec![build_issue("A", &[]), build_issue("B", &["A"])],
        };
        let simulation = build_simulation_output();
        let start_date = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();

        let diagram = generate_gantt_diagram(&project, &simulation, start_date, 85.0).unwrap();
        assert!(diagram.contains("# Demo Timeline"));
        assert!(diagram.contains("gantt"));
        assert!(diagram.contains("A Name A"));
        assert!(diagram.contains("B Name B"));
        assert!(diagram.contains("01-01-2026"));
    }
}
