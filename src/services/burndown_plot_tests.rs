use super::*;

use crate::domain::calendar::{Calendar, FreeDateRange, TeamCalendar};
use crate::domain::estimate::ThreePointEstimate;
use crate::domain::issue::IssueId;
use crate::services::simulation_types::{SimulationPercentile, WorkPackagePercentiles};

use assert_fs::prelude::*;
use chrono::NaiveDate;
use predicates::prelude::*;

fn on_date(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).unwrap()
}

fn percentile(end_date: NaiveDate) -> SimulationPercentile {
    SimulationPercentile {
        days: 0.0,
        end_date,
    }
}

fn simulation_for(
    id: &str,
    p15: NaiveDate,
    p50: NaiveDate,
    p85: NaiveDate,
) -> WorkPackageSimulation {
    WorkPackageSimulation {
        id: id.to_string(),
        is_milestone: false,
        percentiles: WorkPackagePercentiles {
            p0: percentile(p15),
            p15: percentile(p15),
            p50: percentile(p50),
            p85: percentile(p85),
            p100: percentile(p85),
        },
    }
}

fn base_report(work_packages: Option<Vec<WorkPackageSimulation>>) -> SimulationReport {
    SimulationReport {
        data_source: "unit".to_string(),
        start_date: on_date(2026, 3, 1),
        velocity: Some(1.0),
        iterations: 10,
        simulated_items: 2,
        p0: percentile(on_date(2026, 3, 2)),
        p15: percentile(on_date(2026, 3, 2)),
        p50: percentile(on_date(2026, 3, 3)),
        p85: percentile(on_date(2026, 3, 4)),
        p100: percentile(on_date(2026, 3, 5)),
        work_packages,
    }
}

#[test]
fn build_data_converts_none_estimate_to_one_story_point() {
    let mut done = Issue::new();
    done.issue_id = Some(IssueId {
        id: "DONE-1".to_string(),
    });
    done.status = Some(IssueStatus::Done);
    done.done_date = Some(on_date(2026, 3, 1));
    done.estimate = None;

    let mut todo = Issue::new();
    todo.issue_id = Some(IssueId {
        id: "TODO-1".to_string(),
    });
    todo.status = Some(IssueStatus::ToDo);
    todo.estimate = None;

    let project = Project {
        name: "Demo".to_string(),
        work_packages: vec![done, todo],
    };
    let report = base_report(Some(vec![simulation_for(
        "TODO-1",
        on_date(2026, 3, 2),
        on_date(2026, 3, 3),
        on_date(2026, 3, 4),
    )]));

    let data = build_burndown_data(&project, &report, None).unwrap();
    assert_eq!(data.total_points, 2.0);
    assert!(data.capacity_ranges.is_empty());
    assert_eq!(data.done_points[0].remaining, 1.0);
    assert_eq!(data.p50_points[0].remaining, 0.0);
}

#[test]
fn build_data_rejects_non_story_point_estimates() {
    let mut done = Issue::new();
    done.issue_id = Some(IssueId {
        id: "DONE-1".to_string(),
    });
    done.status = Some(IssueStatus::Done);
    done.done_date = Some(on_date(2026, 3, 1));
    done.estimate = Some(Estimate::ThreePoint(ThreePointEstimate {
        optimistic: Some(1.0),
        most_likely: Some(2.0),
        pessimistic: Some(3.0),
    }));

    let project = Project {
        name: "Demo".to_string(),
        work_packages: vec![done],
    };
    let report = base_report(Some(vec![]));

    let error = build_burndown_data(&project, &report, None).unwrap_err();
    assert!(matches!(
        error,
        BurndownPlotError::UnsupportedEstimateType { .. }
    ));
}

#[test]
fn build_data_requires_done_date_for_done_issues() {
    let mut done = Issue::new();
    done.issue_id = Some(IssueId {
        id: "DONE-1".to_string(),
    });
    done.status = Some(IssueStatus::Done);
    done.estimate = Some(Estimate::StoryPoint(StoryPointEstimate {
        estimate: Some(2.0),
    }));

    let mut todo = Issue::new();
    todo.issue_id = Some(IssueId {
        id: "TODO-1".to_string(),
    });
    todo.status = Some(IssueStatus::ToDo);
    todo.estimate = Some(Estimate::StoryPoint(StoryPointEstimate {
        estimate: Some(3.0),
    }));

    let project = Project {
        name: "Demo".to_string(),
        work_packages: vec![done, todo],
    };
    let report = base_report(Some(vec![simulation_for(
        "TODO-1",
        on_date(2026, 3, 2),
        on_date(2026, 3, 3),
        on_date(2026, 3, 4),
    )]));

    let error = build_burndown_data(&project, &report, None).unwrap_err();
    assert!(matches!(error, BurndownPlotError::MissingDoneDate { .. }));
}

#[test]
fn build_data_requires_at_least_one_done_issue() {
    let mut todo = Issue::new();
    todo.issue_id = Some(IssueId {
        id: "TODO-1".to_string(),
    });
    todo.status = Some(IssueStatus::ToDo);
    todo.estimate = Some(Estimate::StoryPoint(StoryPointEstimate {
        estimate: Some(3.0),
    }));

    let project = Project {
        name: "Demo".to_string(),
        work_packages: vec![todo],
    };
    let report = base_report(Some(vec![simulation_for(
        "TODO-1",
        on_date(2026, 3, 2),
        on_date(2026, 3, 3),
        on_date(2026, 3, 4),
    )]));

    let error = build_burndown_data(&project, &report, None).unwrap_err();
    assert!(matches!(error, BurndownPlotError::NoDoneIssues));
}

#[test]
fn build_data_requires_simulation_for_not_done_issue() {
    let mut done = Issue::new();
    done.issue_id = Some(IssueId {
        id: "DONE-1".to_string(),
    });
    done.status = Some(IssueStatus::Done);
    done.done_date = Some(on_date(2026, 3, 1));
    done.estimate = Some(Estimate::StoryPoint(StoryPointEstimate {
        estimate: Some(2.0),
    }));

    let mut todo = Issue::new();
    todo.issue_id = Some(IssueId {
        id: "TODO-1".to_string(),
    });
    todo.status = Some(IssueStatus::ToDo);
    todo.estimate = Some(Estimate::StoryPoint(StoryPointEstimate {
        estimate: Some(3.0),
    }));

    let project = Project {
        name: "Demo".to_string(),
        work_packages: vec![done, todo],
    };
    let report = base_report(Some(vec![]));

    let error = build_burndown_data(&project, &report, None).unwrap_err();
    assert!(matches!(
        error,
        BurndownPlotError::MissingSimulationForIssue { .. }
    ));
}

#[test]
fn build_data_collects_low_capacity_ranges_from_calendar() {
    let mut done = Issue::new();
    done.issue_id = Some(IssueId {
        id: "DONE-1".to_string(),
    });
    done.status = Some(IssueStatus::Done);
    done.done_date = Some(on_date(2026, 3, 1));
    done.estimate = Some(Estimate::StoryPoint(StoryPointEstimate {
        estimate: Some(2.0),
    }));

    let mut todo = Issue::new();
    todo.issue_id = Some(IssueId {
        id: "TODO-1".to_string(),
    });
    todo.status = Some(IssueStatus::ToDo);
    todo.estimate = Some(Estimate::StoryPoint(StoryPointEstimate {
        estimate: Some(3.0),
    }));

    let project = Project {
        name: "Demo".to_string(),
        work_packages: vec![done, todo],
    };
    let report = base_report(Some(vec![simulation_for(
        "TODO-1",
        on_date(2026, 3, 3),
        on_date(2026, 3, 4),
        on_date(2026, 3, 6),
    )]));

    let calendar = TeamCalendar {
        calendars: vec![
            Calendar {
                free_weekdays: vec![chrono::Weekday::Tue],
                free_date_ranges: vec![],
            },
            Calendar {
                free_weekdays: vec![],
                free_date_ranges: vec![FreeDateRange {
                    start_date: on_date(2026, 3, 5),
                    end_date: on_date(2026, 3, 6),
                }],
            },
        ],
    };

    let data = build_burndown_data(&project, &report, Some(&calendar)).unwrap();

    assert_eq!(
        data.capacity_ranges,
        vec![
            CapacityRange {
                start_date: on_date(2026, 3, 3),
                end_date: on_date(2026, 3, 3),
                capacity: 0.5,
            },
            CapacityRange {
                start_date: on_date(2026, 3, 5),
                end_date: on_date(2026, 3, 6),
                capacity: 0.5,
            },
        ]
    );
}

#[test]
fn plot_burndown_from_yaml_files_writes_png() {
    let project_yaml = r#"
name: Demo
work_packages:
  - id: DONE-1
    status: Done
    done_date: 2026-03-01
    estimate:
      type: story_points
      value: 3
  - id: TODO-1
    status: ToDo
"#;

    let report_yaml = r#"
data_source: unit
start_date: 2026-03-01
velocity: 2.0
iterations: 100
simulated_items: 2
p0:
  days: 1
  end_date: 2026-03-02
p15:
  days: 2
  end_date: 2026-03-03
p50:
  days: 3
  end_date: 2026-03-04
p85:
  days: 4
  end_date: 2026-03-05
p100:
  days: 5
  end_date: 2026-03-06
work_packages:
  - id: TODO-1
    is_milestone: false
    percentiles:
      p0:
        days: 1
        end_date: 2026-03-02
      p15:
        days: 2
        end_date: 2026-03-03
      p50:
        days: 3
        end_date: 2026-03-04
      p85:
        days: 4
        end_date: 2026-03-05
      p100:
        days: 5
        end_date: 2026-03-06
"#;

    let project_file = assert_fs::NamedTempFile::new("project.yaml").unwrap();
    project_file.write_str(project_yaml).unwrap();
    let report_file = assert_fs::NamedTempFile::new("result.yaml").unwrap();
    report_file.write_str(report_yaml).unwrap();
    let output_file = assert_fs::NamedTempFile::new("burndown.png").unwrap();

    plot_burndown_from_yaml_files(
        project_file.path().to_str().unwrap(),
        report_file.path().to_str().unwrap(),
        output_file.path().to_str().unwrap(),
        None,
    )
    .unwrap();

    output_file.assert(predicate::path::exists());
    let metadata = std::fs::metadata(output_file.path()).unwrap();
    assert!(metadata.len() > 0);
}
