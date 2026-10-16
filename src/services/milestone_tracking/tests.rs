use super::*;
use crate::services::project_simulation::simulation_types::{
    SimulationPercentile, SimulationReport, WorkPackagePercentiles, WorkPackageSimulation,
};

fn date(day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 1, day).unwrap()
}

fn percentile(day: u32) -> SimulationPercentile {
    SimulationPercentile {
        days: day as f32 - 1.0,
        end_date: date(day),
    }
}

fn simulation() -> SimulationOutput {
    SimulationOutput {
        report: SimulationReport {
            data_source: "project.yaml".into(),
            start_date: date(1),
            velocity: None,
            iterations: 1,
            simulated_items: 1,
            p0: percentile(10),
            p15: percentile(10),
            p50: percentile(10),
            p85: percentile(10),
            p100: percentile(10),
            work_packages: Some(vec![WorkPackageSimulation {
                id: "MS_1".into(),
                work_package_type: WorkPackageType::Milestone,
                estimate: None,
                done_date: None,
                percentiles: WorkPackagePercentiles {
                    p0: percentile(3),
                    p15: percentile(4),
                    p50: percentile(5),
                    p85: percentile(6),
                    p100: percentile(7),
                },
            }]),
        },
        results: vec![9.0],
        milestone_summaries: [("MS_1".into(), Some("First milestone".into()))].into(),
    }
}

fn read(path: &Path) -> MilestoneTracking {
    serde_yaml::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn creates_tracking_with_milestone_dates_and_summary() {
    let file = assert_fs::NamedTempFile::new("milestones.yaml").unwrap();
    update_milestones_file(file.path(), &simulation(), date(2)).unwrap();
    let tracking = read(file.path());
    assert_eq!(tracking.start_date, date(1));
    assert_eq!(tracking.end_date, date(7));
    assert_eq!(tracking.milestones.len(), 1);
    let milestone = &tracking.milestones[0];
    assert_eq!(milestone.id, "MS_1");
    assert_eq!(milestone.name, "First milestone");
    assert!(milestone.plot);
    assert_eq!(milestone.forecasts.len(), 1);
    let forecast = &milestone.forecasts[0];
    assert_eq!(forecast.date, date(2));
    assert_eq!(
        [
            forecast.p0,
            forecast.p15,
            forecast.p50,
            forecast.p85,
            forecast.p100
        ],
        [date(3), date(4), date(5), date(6), date(7)]
    );
    let yaml = std::fs::read_to_string(file.path()).unwrap();
    assert!(yaml.contains("start_date: 2026-01-01"));
    assert!(yaml.contains("date: 2026-01-02"));
    assert!(yaml.contains("p100: 2026-01-07"));
    assert!(!yaml.contains("days:"));
}

#[test]
fn appends_dates_refreshes_names_and_preserves_plot_and_start_date() {
    let file = assert_fs::NamedTempFile::new("milestones.yaml").unwrap();
    let mut output = simulation();
    update_milestones_file(file.path(), &output, date(2)).unwrap();
    let mut tracking = read(file.path());
    tracking.milestones[0].plot = false;
    std::fs::write(file.path(), serde_yaml::to_string(&tracking).unwrap()).unwrap();

    output
        .milestone_summaries
        .insert("MS_1".into(), Some("Renamed".into()));
    output.report.start_date = date(3);
    output.report.work_packages.as_mut().unwrap()[0]
        .percentiles
        .p100 = percentile(8);
    update_milestones_file(file.path(), &output, date(3)).unwrap();
    let tracking = read(file.path());
    assert_eq!(tracking.start_date, date(1));
    assert_eq!(tracking.end_date, date(8));
    let milestone = &tracking.milestones[0];
    assert_eq!(milestone.name, "Renamed");
    assert!(!milestone.plot);
    assert_eq!(milestone.forecasts.len(), 2);
    assert_eq!(milestone.forecasts[0].date, date(2));
    assert_eq!(milestone.forecasts[1].date, date(3));
}

#[test]
fn replaces_same_day_forecast() {
    let file = assert_fs::NamedTempFile::new("milestones.yaml").unwrap();
    let mut output = simulation();
    update_milestones_file(file.path(), &output, date(2)).unwrap();
    output.report.work_packages.as_mut().unwrap()[0]
        .percentiles
        .p100 = percentile(6);
    update_milestones_file(file.path(), &output, date(2)).unwrap();
    let tracking = read(file.path());
    assert_eq!(tracking.milestones.len(), 1);
    assert_eq!(tracking.milestones[0].forecasts.len(), 1);
    assert_eq!(tracking.milestones[0].forecasts[0].p100, date(6));
    assert_eq!(tracking.end_date, date(6));
}

#[test]
fn retains_absent_milestones_and_adds_new_ids() {
    let file = assert_fs::NamedTempFile::new("milestones.yaml").unwrap();
    let mut output = simulation();
    update_milestones_file(file.path(), &output, date(2)).unwrap();
    output.report.work_packages.as_mut().unwrap()[0].id = "MS_2".into();
    output
        .milestone_summaries
        .insert("MS_2".into(), Some("Second".into()));
    update_milestones_file(file.path(), &output, date(3)).unwrap();
    let tracking = read(file.path());
    assert_eq!(tracking.milestones.len(), 2);
    assert_eq!(tracking.milestones[0].id, "MS_1");
    assert_eq!(tracking.milestones[0].name, "First milestone");
    assert_eq!(tracking.milestones[0].forecasts.len(), 1);
    assert_eq!(tracking.milestones[0].forecasts[0].date, date(2));
    assert_eq!(tracking.milestones[1].id, "MS_2");
    assert!(tracking.milestones[1].plot);
}

#[test]
fn end_date_uses_latest_current_milestone_not_overall_or_historical_end() {
    let file = assert_fs::NamedTempFile::new("milestones.yaml").unwrap();
    let mut output = simulation();
    output.report.work_packages.as_mut().unwrap()[0]
        .percentiles
        .p100 = percentile(20);
    update_milestones_file(file.path(), &output, date(2)).unwrap();
    let work_packages = output.report.work_packages.as_mut().unwrap();
    work_packages[0].percentiles.p100 = percentile(4);
    let mut second = work_packages[0].clone();
    second.id = "MS_2".into();
    second.percentiles.p100 = percentile(8);
    work_packages.push(second);
    output
        .milestone_summaries
        .insert("MS_2".into(), Some("Second".into()));
    update_milestones_file(file.path(), &output, date(3)).unwrap();
    assert_eq!(read(file.path()).end_date, date(8));
}

#[test]
fn missing_plot_defaults_to_true() {
    let file = assert_fs::NamedTempFile::new("milestones.yaml").unwrap();
    std::fs::write(file.path(), "start_date: 2026-01-01\nend_date: 2026-01-20\nmilestones:\n- id: MS_1\n  name: Old\n  forecasts: []\n").unwrap();
    update_milestones_file(file.path(), &simulation(), date(2)).unwrap();
    assert!(read(file.path()).milestones[0].plot);
}

#[test]
fn runs_without_milestones_initialize_overall_end_then_preserve_metadata() {
    for work_packages in [None, Some(Vec::new()), {
        let mut packages = simulation().report.work_packages.unwrap();
        packages[0].work_package_type = WorkPackageType::ToDo;
        Some(packages)
    }] {
        let file = assert_fs::NamedTempFile::new("milestones.yaml").unwrap();
        let mut output = simulation();
        output.report.work_packages = work_packages;
        update_milestones_file(file.path(), &output, date(2)).unwrap();
        let tracking = read(file.path());
        assert!(tracking.milestones.is_empty());
        assert_eq!(tracking.end_date, date(10));
        output.report.p100 = percentile(20);
        output.report.start_date = date(3);
        update_milestones_file(file.path(), &output, date(3)).unwrap();
        let tracking = read(file.path());
        assert_eq!(tracking.start_date, date(1));
        assert_eq!(tracking.end_date, date(10));
    }
}

#[test]
fn no_milestones_preserves_existing_records() {
    let file = assert_fs::NamedTempFile::new("milestones.yaml").unwrap();
    let mut output = simulation();
    update_milestones_file(file.path(), &output, date(2)).unwrap();
    let original = std::fs::read_to_string(file.path()).unwrap();
    output.report.work_packages = None;
    update_milestones_file(file.path(), &output, date(3)).unwrap();
    assert_eq!(std::fs::read_to_string(file.path()).unwrap(), original);
}

#[test]
fn missing_summary_fails_without_creating_or_modifying_tracking() {
    for summary in [Some(None), None] {
        let file = assert_fs::NamedTempFile::new("milestones.yaml").unwrap();
        let mut output = simulation();
        output.milestone_summaries.clear();
        if let Some(summary) = summary {
            output.milestone_summaries.insert("MS_1".into(), summary);
        }
        assert!(
            matches!(update_milestones_file(file.path(), &output, date(2)),
            Err(MilestoneTrackingError::MissingSummary(id)) if id == "MS_1")
        );
        assert!(!file.path().exists());
        update_milestones_file(file.path(), &simulation(), date(2)).unwrap();
        let original = std::fs::read_to_string(file.path()).unwrap();
        assert!(update_milestones_file(file.path(), &output, date(3)).is_err());
        assert_eq!(std::fs::read_to_string(file.path()).unwrap(), original);
    }
}

#[test]
fn invalid_yaml_and_missing_required_fields_fail_without_overwriting() {
    for yaml in [
        "[",
        "",
        "{}",
        "start_date: 2026-01-01\nend_date: 2026-01-02\n",
        "start_date: invalid\nend_date: 2026-01-02\nmilestones: []\n",
        "start_date: 2026-01-01\nend_date: 2026-01-02\nmilestones:\n- id: MS_1\n  forecasts: []\n",
        "start_date: 2026-01-01\nend_date: 2026-01-02\nmilestones:\n- id: MS_1\n  name: Name\n  forecasts:\n  - date: 2026-01-02\n",
    ] {
        let file = assert_fs::NamedTempFile::new("milestones.yaml").unwrap();
        std::fs::write(file.path(), yaml).unwrap();
        assert!(matches!(
            update_milestones_file(file.path(), &simulation(), date(2)),
            Err(MilestoneTrackingError::Parse { .. })
        ));
        assert_eq!(std::fs::read_to_string(file.path()).unwrap(), yaml);
    }
}

#[test]
fn read_and_write_failures_propagate() {
    let directory = assert_fs::TempDir::new().unwrap();
    assert!(matches!(
        update_milestones_file(directory.path(), &simulation(), date(2)),
        Err(MilestoneTrackingError::Read { .. })
    ));
    let missing_parent = directory.path().join("missing/milestones.yaml");
    assert!(matches!(
        update_milestones_file(&missing_parent, &simulation(), date(2)),
        Err(MilestoneTrackingError::Write { .. })
    ));
}
