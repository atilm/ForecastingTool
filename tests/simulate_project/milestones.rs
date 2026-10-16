use assert_fs::prelude::*;
use chrono::Local;
use predicates::prelude::*;
use serde_yaml::Value;
use std::fs;
use std::path::Path;

const PROJECT: &str = "name: Demo
work_packages:
  - id: TASK
    estimate: {type: three_point, optimistic: 5, most_likely: 5, pessimistic: 5}
  - id: MS_1
    summary: First milestone
    estimate: {type: milestone}
    dependencies: [TASK]
";

fn fixture() -> assert_fs::TempDir {
    let directory = assert_fs::TempDir::new().unwrap();
    directory.child("projects").create_dir_all().unwrap();
    directory
        .child("projects/project.yaml")
        .write_str(PROJECT)
        .unwrap();
    directory
}

fn command(directory: &Path, start_date: &str) -> assert_cmd::Command {
    let mut command = assert_cmd::cargo_bin_cmd!("forecasts");
    command.current_dir(directory).args([
        "simulate",
        "project",
        "-i",
        "projects/project.yaml",
        "-o",
        "report.yaml",
        "-s",
        start_date,
        "-n",
        "1",
    ]);
    command
}

fn read(path: &Path) -> Value {
    serde_yaml::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn default_tracking_is_beside_input_not_working_directory_and_no_png_is_created() {
    let directory = fixture();
    let earliest_run_date = Local::now().date_naive().to_string();
    command(directory.path(), "2026-01-01").assert().success();
    let latest_run_date = Local::now().date_naive().to_string();

    let tracking = read(directory.child("projects/milestones.yaml").path());
    let report = read(directory.child("report.yaml").path());
    assert_eq!(tracking["start_date"], "2026-01-01");
    assert_eq!(tracking["end_date"], "2026-01-06");
    assert_eq!(tracking["milestones"].as_sequence().unwrap().len(), 1);
    let milestone = &tracking["milestones"][0];
    assert_eq!(milestone["id"], "MS_1");
    assert_eq!(milestone["name"], "First milestone");
    assert_eq!(milestone["plot"], true);
    assert_eq!(milestone["forecasts"].as_sequence().unwrap().len(), 1);
    let forecast = &milestone["forecasts"][0];
    assert!(
        [earliest_run_date.as_str(), latest_run_date.as_str()]
            .contains(&forecast["date"].as_str().unwrap())
    );
    for percentile in ["p0", "p15", "p50", "p85", "p100"] {
        assert_eq!(
            forecast[percentile],
            report["work_packages"][1]["percentiles"][percentile]["end_date"]
        );
    }
    assert!(report.get("milestone_summaries").is_none());
    assert!(report["work_packages"][1].get("summary").is_none());
    assert!(!directory.child("milestones.yaml").path().exists());
    assert!(
        !directory
            .child("report.yaml.milestones.png")
            .path()
            .exists()
    );
}

#[test]
fn explicit_relative_path_uses_working_directory_and_replaces_same_day_forecast() {
    let directory = fixture();
    directory.child("tracking").create_dir_all().unwrap();
    let tracking_file = directory.child("tracking/history.yaml");
    command(directory.path(), "2026-01-01")
        .args(["--milestones-file", "tracking/history.yaml"])
        .assert()
        .success();
    let mut tracking = read(tracking_file.path());
    tracking["milestones"][0]["plot"] = Value::Bool(false);
    tracking["milestones"][0]["name"] = Value::String("Old name".into());
    fs::write(
        tracking_file.path(),
        serde_yaml::to_string(&tracking).unwrap(),
    )
    .unwrap();

    command(directory.path(), "2026-02-01")
        .args(["--milestones-file", "tracking/history.yaml"])
        .assert()
        .success();
    let tracking = read(tracking_file.path());
    assert_eq!(tracking["start_date"], "2026-01-01");
    assert_eq!(tracking["end_date"], "2026-02-06");
    let milestone = &tracking["milestones"][0];
    assert_eq!(milestone["plot"], false);
    assert_eq!(milestone["name"], "First milestone");
    assert_eq!(milestone["forecasts"].as_sequence().unwrap().len(), 1);
    assert_eq!(milestone["forecasts"][0]["p100"], "2026-02-06");
    assert!(!directory.child("projects/milestones.yaml").path().exists());
    assert!(
        !directory
            .child("projects/tracking/history.yaml")
            .path()
            .exists()
    );
}

#[test]
fn missing_summary_is_a_command_error() {
    let directory = fixture();
    directory
        .child("projects/project.yaml")
        .write_str(&PROJECT.replace("    summary: First milestone\n", ""))
        .unwrap();
    command(directory.path(), "2026-01-01")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "milestone 'MS_1' is missing a summary",
        ));
    assert!(!directory.child("projects/milestones.yaml").path().exists());
}

#[test]
fn tracking_parse_read_and_write_errors_fail_the_command() {
    let directory = fixture();
    directory.child("invalid.yaml").write_str("[").unwrap();
    directory
        .child("missing_fields.yaml")
        .write_str("{}")
        .unwrap();
    directory.child("unreadable.yaml").create_dir_all().unwrap();
    for (path, error) in [
        ("invalid.yaml", "failed to parse milestone tracking file"),
        (
            "missing_fields.yaml",
            "failed to parse milestone tracking file",
        ),
        ("unreadable.yaml", "failed to read milestone tracking file"),
        (
            "missing/milestones.yaml",
            "failed to write milestone tracking file",
        ),
    ] {
        command(directory.path(), "2026-01-01")
            .args(["--milestones-file", path])
            .assert()
            .failure()
            .stderr(predicate::str::contains(error));
    }
    assert_eq!(
        fs::read_to_string(directory.child("invalid.yaml").path()).unwrap(),
        "["
    );
}
