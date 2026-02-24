use chrono::NaiveDate;
use thiserror::Error;

use crate::domain::project::Project;
use crate::services::simulation_types::SimulationOutput;

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
        let id = issue
            .issue_id
            .as_ref()
            .map(|id| id.id.clone())
            .unwrap_or_default();
        let name = issue.summary.as_deref().unwrap_or(&id).to_string();
        let wp = map
            .get(&id)
            .ok_or_else(|| GanttDiagramError::MissingWorkPackage(id.clone()))?;

        let start_date_wp = wp.percentiles.p85.start_date;
        let end_date_wp = wp.percentiles.p85.end_date;

        if issue.has_zero_duration().unwrap_or(false) {
            lines.push(make_milestone_line(&id, &name, end_date_wp));
        } else {
            lines.push(make_work_package_line(
                &id,
                &name,
                start_date_wp,
                end_date_wp,
            ));
        }
    }
    lines.push("```".to_string());

    Ok(lines.join("\n"))
}

fn make_work_package_line(
    issue: &str,
    name: &str,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> String {
    format!(
        "    {issue} {name} :{issue}, {}, {}",
        start_date.format("%d-%m-%Y"),
        end_date.format("%d-%m-%Y")
    )
}

fn make_milestone_line(issue: &str, name: &str, date: NaiveDate) -> String {
    format!(
        "    {issue} {name} :milestone, {}, 0",
        date.format("%d-%m-%Y"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::issue::{Issue, IssueId};
    use crate::domain::issue_status::IssueStatus;
    use crate::services::simulation_types::{
        SimulationOutput, SimulationPercentile, SimulationReport, WorkPackagePercentiles,
        WorkPackageSimulation,
    };
    use crate::test_support::on_date;

    fn wp_percentile(start_date: NaiveDate, days: f32) -> SimulationPercentile {
        SimulationPercentile {
            days,
            start_date,
            end_date: add_days(start_date, days),
        }
    }

    fn add_days(start_date: NaiveDate, days: f32) -> NaiveDate {
        let days = days.ceil().max(0.0) as i64;
        start_date + chrono::Duration::days(days)
    }

    fn wp_percentile_from_dates(
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> SimulationPercentile {
        let days = (end_date - start_date).num_days() as f32;
        SimulationPercentile {
            days,
            start_date,
            end_date,
        }
    }

    fn build_issue(id: &str, deps: &[&str]) -> Issue {
        let mut issue = Issue::new();
        issue.issue_id = Some(IssueId { id: id.to_string() });
        issue.summary = Some(format!("Name {id}"));
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

    fn build_basic_simulation_output() -> SimulationOutput {
        let start_date = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();

        SimulationOutput {
            report: SimulationReport {
                data_source: "unit-test".to_string(),
                start_date,
                velocity: None,
                iterations: 1,
                simulated_items: 2,
                p0: SimulationPercentile {
                    days: 0.0,
                    start_date,
                    end_date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                },
                p50: SimulationPercentile {
                    days: 0.0,
                    start_date,
                    end_date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                },
                p85: SimulationPercentile {
                    days: 0.0,
                    start_date,
                    end_date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                },
                p100: SimulationPercentile {
                    days: 0.0,
                    start_date,
                    end_date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                },
            },
            results: vec![1.0],
            work_packages: Some(Vec::new()),
        }
    }

    fn add_work_package(
        simulation: &mut SimulationOutput,
        id: &str,
        status: IssueStatus,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) {
        let wp = WorkPackageSimulation {
            id: id.to_string(),
            status,
            percentiles: WorkPackagePercentiles {
                p0: wp_percentile_from_dates(start_date, end_date),
                p50: wp_percentile_from_dates(start_date, end_date),
                p85: wp_percentile_from_dates(start_date, end_date),
                p100: wp_percentile_from_dates(start_date, end_date),
            },
        };
        if let Some(wps) = simulation.work_packages.as_mut() {
            wps.push(wp);
        }
    }

    fn build_simulation_output() -> SimulationOutput {
        let start_date = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();

        SimulationOutput {
            report: SimulationReport {
                data_source: "unit-test".to_string(),
                start_date,
                velocity: None,
                iterations: 1,
                simulated_items: 2,
                p0: SimulationPercentile {
                    days: 0.0,
                    start_date,
                    end_date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                },
                p50: SimulationPercentile {
                    days: 0.0,
                    start_date,
                    end_date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                },
                p85: SimulationPercentile {
                    days: 0.0,
                    start_date,
                    end_date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                },
                p100: SimulationPercentile {
                    days: 0.0,
                    start_date,
                    end_date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                },
            },
            results: vec![1.0],
            work_packages: Some(vec![
                WorkPackageSimulation {
                    id: "A".to_string(),
                    status: IssueStatus::ToDo,
                    percentiles: WorkPackagePercentiles {
                        p0: wp_percentile(start_date, 1.0),
                        p50: wp_percentile(start_date, 1.0),
                        p85: wp_percentile(start_date, 1.0),
                        p100: wp_percentile(start_date, 1.0),
                    },
                },
                WorkPackageSimulation {
                    id: "B".to_string(),
                    status: IssueStatus::ToDo,
                    percentiles: WorkPackagePercentiles {
                        p0: wp_percentile(start_date, 3.0),
                        p50: wp_percentile(start_date, 3.0),
                        p85: wp_percentile(start_date, 3.0),
                        p100: wp_percentile(start_date, 3.0),
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

    #[test]
    fn generate_gantt_diagram_shows_status_and_milestones_correctly() {
        let project = Project {
            name: "Demo Project".to_string(),
            work_packages: vec![
                build_issue("WP-1", &[]),
                build_issue("WP-2", &["WP-1"]),
                build_issue("WP-3", &["WP-2"]),
            ],
        };

        // ToDo:
        // - Add a milestone
        // - Assert that status is given

        let mut simulation_output = build_basic_simulation_output();
        add_work_package(
            &mut simulation_output,
            "WP-1",
            IssueStatus::Done,
            on_date(2026, 1, 1),
            on_date(2026, 1, 5),
        );
        add_work_package(
            &mut simulation_output,
            "WP-2",
            IssueStatus::InProgress,
            on_date(2026, 1, 6),
            on_date(2026, 1, 9),
        );
        add_work_package(
            &mut simulation_output,
            "WP-3",
            IssueStatus::ToDo,
            on_date(2026, 1, 19),
            on_date(2026, 1, 21),
        );

        let dummy_start_date = on_date(2026, 1, 1);
        let diagram =
            generate_gantt_diagram(&project, &simulation_output, dummy_start_date, 85.0).unwrap();

        assert_eq!(
            diagram,
            r#"
# Demo Project Timeline
```mermaid
gantt
    dateFormat  DD-MM-YYYY
    WP-1 Name WP-1 :WP-1, 01-01-2026, 05-01-2026
    WP-2 Name WP-2 :WP-2, 06-01-2026, 09-01-2026
    WP-3 Name WP-3 :WP-3, 19-01-2026, 21-01-2026
```"#
        )
    }
}
