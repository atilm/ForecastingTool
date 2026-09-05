use super::*;

use crate::services::project_simulation::simulation_types::{
    SimulationPercentile, WorkPackagePercentiles,
};
use assert_fs::prelude::*;
use chrono::NaiveDate;
use predicates::prelude::*;

fn date(value: &str) -> NaiveDate {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").unwrap()
}

fn percentile(end_date: NaiveDate) -> SimulationPercentile {
    SimulationPercentile {
        days: 0.0,
        end_date,
    }
}

fn work_package(
    id: &str,
    work_package_type: WorkPackageType,
    estimate: Option<Estimate>,
    done_date: Option<NaiveDate>,
    completion_date: NaiveDate,
) -> WorkPackageSimulation {
    WorkPackageSimulation {
        id: id.to_string(),
        work_package_type,
        estimate,
        done_date,
        percentiles: WorkPackagePercentiles {
            p0: percentile(completion_date),
            p15: percentile(completion_date),
            p50: percentile(completion_date),
            p85: percentile(completion_date),
            p100: percentile(completion_date),
        },
    }
}

fn story_points(points: f32) -> Option<Estimate> {
    Some(Estimate::StoryPoint(StoryPointEstimate {
        estimate: Some(points),
    }))
}

fn base_report(work_packages: Option<Vec<WorkPackageSimulation>>) -> SimulationReport {
    let start = date("2026-03-01");
    SimulationReport {
        data_source: "unit".to_string(),
        start_date: start,
        velocity: Some(1.0),
        iterations: 10,
        simulated_items: 2,
        p0: percentile(start),
        p15: percentile(start),
        p50: percentile(start),
        p85: percentile(start),
        p100: percentile(start),
        work_packages,
    }
}

#[test]
fn build_data_includes_dynamic_work_at_chart_start_and_never_rises() {
    let report = base_report(Some(vec![
        work_package(
            "DONE-1",
            WorkPackageType::Done,
            story_points(2.0),
            Some(date("2026-03-01")),
            date("2026-03-01"),
        ),
        work_package(
            "TODO-1",
            WorkPackageType::ToDo,
            story_points(3.0),
            None,
            date("2026-03-02"),
        ),
        work_package(
            "GENERATED-1",
            WorkPackageType::DynamicToDo,
            story_points(5.0),
            None,
            date("2026-03-03"),
        ),
        work_package(
            "IN-PROGRESS-1",
            WorkPackageType::InProgress,
            story_points(2.0),
            None,
            date("2026-03-04"),
        ),
        work_package(
            "MILESTONE-1",
            WorkPackageType::Milestone,
            Some(Estimate::Milestone),
            None,
            date("2026-03-05"),
        ),
    ]));

    let data = build_burndown_data(&report, None).unwrap();

    assert_eq!(data.total_points, 12.0);
    assert_eq!(data.done_points[0].remaining, 10.0);
    assert_eq!(
        data.forecast_p50_points.last().unwrap().point.remaining,
        0.0
    );
    assert!(
        data.forecast_p50_points
            .windows(2)
            .all(|points| points[1].point.remaining <= points[0].point.remaining)
    );
    assert!(
        data.forecast_p50_points
            .iter()
            .any(|point| point.work_package_type == WorkPackageType::DynamicToDo)
    );
}

#[test]
fn build_data_defaults_missing_estimate_to_one_story_point() {
    let report = base_report(Some(vec![
        work_package(
            "DONE-1",
            WorkPackageType::Done,
            None,
            Some(date("2026-03-01")),
            date("2026-03-01"),
        ),
        work_package(
            "TODO-1",
            WorkPackageType::ToDo,
            None,
            None,
            date("2026-03-02"),
        ),
    ]));
    assert_eq!(
        build_burndown_data(&report, None).unwrap().total_points,
        2.0
    );
}

#[test]
fn build_data_rejects_missing_done_date_with_item_id() {
    let report = base_report(Some(vec![
        work_package(
            "DONE-1",
            WorkPackageType::Done,
            story_points(2.0),
            None,
            date("2026-03-01"),
        ),
        work_package(
            "TODO-1",
            WorkPackageType::ToDo,
            story_points(2.0),
            None,
            date("2026-03-02"),
        ),
    ]));
    assert!(
        matches!(build_burndown_data(&report, None), Err(BurndownPlotError::MissingDoneDate { id }) if id == "DONE-1")
    );
}

#[test]
fn build_data_rejects_unsupported_estimate() {
    let report = base_report(Some(vec![
        work_package(
            "DONE-1",
            WorkPackageType::Done,
            story_points(2.0),
            Some(date("2026-03-01")),
            date("2026-03-01"),
        ),
        work_package(
            "TODO-1",
            WorkPackageType::ToDo,
            Some(Estimate::ThreePoint(
                crate::domain::estimate::ThreePointEstimate {
                    optimistic: Some(1.0),
                    most_likely: Some(2.0),
                    pessimistic: Some(3.0),
                },
            )),
            None,
            date("2026-03-02"),
        ),
    ]));
    assert!(
        matches!(build_burndown_data(&report, None), Err(BurndownPlotError::UnsupportedEstimateType { id }) if id == "TODO-1")
    );
}

#[test]
fn build_data_requires_work_package_data_and_both_work_categories() {
    assert!(matches!(
        build_burndown_data(&base_report(None), None),
        Err(BurndownPlotError::MissingSimulationWorkPackages)
    ));
    assert!(matches!(
        build_burndown_data(
            &base_report(Some(vec![work_package(
                "DONE-1",
                WorkPackageType::Done,
                story_points(1.0),
                Some(date("2026-03-01")),
                date("2026-03-01")
            )])),
            None
        ),
        Err(BurndownPlotError::NoForecastIssues)
    ));
    assert!(matches!(
        build_burndown_data(
            &base_report(Some(vec![work_package(
                "TODO-1",
                WorkPackageType::ToDo,
                story_points(1.0),
                None,
                date("2026-03-02")
            )])),
            None
        ),
        Err(BurndownPlotError::NoDoneIssues)
    ));
}

#[test]
fn plot_burndown_from_report_writes_png() {
    let report_yaml = r#"
data_source: unit
start_date: 2026-03-01
velocity: 2.0
iterations: 100
simulated_items: 2
p0: { days: 1, end_date: 2026-03-02 }
p15: { days: 1, end_date: 2026-03-02 }
p50: { days: 1, end_date: 2026-03-02 }
p85: { days: 1, end_date: 2026-03-02 }
p100: { days: 1, end_date: 2026-03-02 }
work_packages:
  - id: DONE-1
    type: Done
    estimate: { type: story_point, estimate: 3.0 }
    done_date: 2026-03-01
    percentiles: { p0: { days: 0, end_date: 2026-03-01 }, p15: { days: 0, end_date: 2026-03-01 }, p50: { days: 0, end_date: 2026-03-01 }, p85: { days: 0, end_date: 2026-03-01 }, p100: { days: 0, end_date: 2026-03-01 } }
  - id: GENERATED-1
    type: DynamicToDo
    estimate: { type: story_point, estimate: 5.0 }
    done_date: null
    percentiles: { p0: { days: 1, end_date: 2026-03-02 }, p15: { days: 1, end_date: 2026-03-02 }, p50: { days: 2, end_date: 2026-03-03 }, p85: { days: 3, end_date: 2026-03-04 }, p100: { days: 4, end_date: 2026-03-05 } }
"#;
    let report_file = assert_fs::NamedTempFile::new("result.yaml").unwrap();
    report_file.write_str(report_yaml).unwrap();
    let output_file = assert_fs::NamedTempFile::new("burndown.png").unwrap();

    plot_burndown_from_yaml_files(
        report_file.path().to_str().unwrap(),
        output_file.path().to_str().unwrap(),
        None,
    )
    .unwrap();

    output_file.assert(predicate::path::exists());
    assert!(std::fs::metadata(output_file.path()).unwrap().len() > 0);
}
