use assert_fs::prelude::*;
use predicates::prelude::*;
use std::fs;

#[test]
fn simulate_project_writes_output_and_histogram() {
    let report_yaml = r#"
data_source: "unit"
start_date: "2026-01-01"
velocity: 10.0
iterations: 25
simulated_items: 100
p0:
  days: 5.0
  end_date: "2026-01-06"
p15:
    days: 6
    end_date: "2026-01-07"
p50:
  days: 10.0
  end_date: "2026-01-11"
p85:
  days: 15.0
  end_date: "2026-01-16"
p100:
  days: 20.0
  end_date: "2026-01-21" 
"#;

    let report_file = assert_fs::NamedTempFile::new("report.yaml").unwrap();
    report_file.write_str(report_yaml).unwrap();

    let project_yaml = format!(
        r#"
name: Demo
work_packages:
  - id: DONE-1
    status: Done
    estimate:
      type: story_points
      value: 10
    start_date: 2026-01-01
    done_date: 2026-01-06
  - id: WP0
    estimate:
      type: three_point
      optimistic: 1
      most_likely: 1
      pessimistic: 1
  - id: WP1
    estimate:
      type: reference
      report_file_path: "{}"
    dependencies: []
  - id: FIN
    estimate:
      type: three_point
      optimistic: 0
      most_likely: 0
      pessimistic: 0
    dependencies: [WP0, WP1]
"#,
        report_file.path().to_str().unwrap()
    );

    let input_file = assert_fs::NamedTempFile::new("project.yaml").unwrap();
    input_file.write_str(&project_yaml).unwrap();
    let output_file = assert_fs::NamedTempFile::new("simulation.yaml").unwrap();

    let input_arg = input_file.path().to_str().unwrap().to_string();
    let output_arg = output_file.path().to_str().unwrap().to_string();
    let histogram_path = format!("{output_arg}.png");

    let mut cmd = assert_cmd::cargo_bin_cmd!("forecasts");
    cmd.args(&[
        "simulate",
        "project",
        "-i",
        &input_arg,
        "-o",
        &output_arg,
        "-s",
        "2026-02-01",
        "--iterations",
        "25",
    ]);

    cmd.assert().success().stdout(
        predicate::str::contains("Simulation result written to")
            .and(predicate::str::contains("Simulation Report"))
            .and(predicate::str::contains("Percentile | Days | Date"))
            .and(predicate::str::contains("P85")),
    );

    let output = fs::read_to_string(output_file.path()).unwrap();
    assert!(output.contains("start_date: 2026-02-01"));
    assert!(output.contains("simulated_items:"));
    assert!(output.contains("p0:"));

    assert!(fs::metadata(&histogram_path).is_ok());
}

#[test]
fn simulate_project_handles_referenced_estimates_correctly_when_the_are_in_progress() {
  // Build report file for simulated sub-project
  // All-equal durations for predictable test results
  // Today is 2026-01-14 and there are 9 days remaining
  // In the project file below we can see, that the sub-project started on 2026-01-08, so the total duration should be 15 days
  let report_yaml = r#"
data_source: "sub-project.yaml"
start_date: "2026-01-14"
velocity: null
iterations: 10
simulated_items: 10
p0:
  days: 9.0
  end_date: "2026-01-23"
p15:
  days: 9.0
  end_date: "2026-01-23"
p50:
  days: 9.0
  end_date: "2026-01-23"
p85:
  days: 9.0
  end_date: "2026-01-23"
p100:
  days: 9.0
  end_date: "2026-01-23"
"#;

  let report_file = assert_fs::NamedTempFile::new("report.yaml").unwrap();
  report_file.write_str(report_yaml).unwrap();

  // Build the project file that references the above report for WP0, which is InProgress and depends on DONE-1
  let project_yaml = format!(
        r#"
name: Demo
work_packages:
  - id: DONE-1
    status: Done
    estimate:
      type: three_point
      optimistic: 5
      most_likely: 5
      pessimistic: 5
    start_date: 2026-01-01
    done_date: 2026-01-06
  - id: WP0
    status: InProgress
    estimate:
      type: reference
      report_file_path: "{}"
    start_date: 2026-01-08 # This should be the effective start date, not the end date of DONE-1
    dependencies: [DONE-1]
"#,
        report_file.path().to_str().unwrap()
    );

    // Act: Run the simulation
    let input_file = assert_fs::NamedTempFile::new("project.yaml").unwrap();
    input_file.write_str(&project_yaml).unwrap();
    let output_file = assert_fs::NamedTempFile::new("simulation.yaml").unwrap();

    let input_arg = input_file.path().to_str().unwrap().to_string();
    let output_arg = output_file.path().to_str().unwrap().to_string();

    let mut cmd = assert_cmd::cargo_bin_cmd!("forecasts");
    cmd.args(&[
        "simulate",
        "project",
        "-i",
        &input_arg,
        "-o",
        &output_arg,
        "-s",
        "2026-01-14", // Now is the 14th, DONE-1 is done, WP0 is in progress and should have started on 2026-01-08, so it should end on 2026-01-23
        "--iterations",
        "25",
    ]);

    cmd.assert().success().stdout(
        predicate::str::contains("Simulation result written to")
            .and(predicate::str::contains("Simulation Report"))
            .and(predicate::str::contains("Percentile | Days | Date"))
            .and(predicate::str::contains("P85")),
    );

    // Assert: The end date of WP0 should be 2026-01-23 (2026-01-08 + 15 days), not 2026-01-21 (2026-01-06 + 15 days)
    let output = fs::read_to_string(output_file.path()).unwrap();

    // End date of done issue
    assert!(output.contains("      end_date: 2026-01-06"));
    // End date of in-progress issue
    assert!(output.contains("      end_date: 2026-01-23"));
}

#[test]
fn simulate_project_with_calendar_files() {
    let project_yaml = r#"
name: Demo
work_packages:
  - id: DONE-1
    status: Done
    estimate:
      type: story_points
      value: 10
    start_date: 2026-01-01
    done_date: 2026-01-06
  - id: WP0
    estimate:
      type: story_points
      value: 12
  - id: FIN
    estimate:
      type: three_point
      optimistic: 0
      most_likely: 0
      pessimistic: 0
    dependencies: [WP0]
"#;

    let input_file = assert_fs::NamedTempFile::new("project.yaml").unwrap();
    input_file.write_str(&project_yaml).unwrap();
    let input_arg = input_file.path().to_str().unwrap().to_string();

    let calendar_yaml = r#"
free_weekdays: [Sat, Sun]
free_date_ranges:
  - start_date: 2025-05-13
    end_date: 2025-07-07
"#;

    // Create a temporary calendar dir and within it a temporary calendar file
    let calendar_dir = assert_fs::TempDir::new().unwrap();
    let calendar_file = calendar_dir.child("calendar.yaml");
    calendar_file.write_str(calendar_yaml).unwrap();
    let calendar_dir_path = calendar_dir.path().to_str().unwrap();

    let output_file = assert_fs::NamedTempFile::new("simulation.yaml").unwrap();
    let output_arg = output_file.path().to_str().unwrap();

    let mut cmd = assert_cmd::cargo_bin_cmd!("forecasts");
    cmd.args(&[
        "simulate",
        "project",
        "-i",
        &input_arg,
        "-o",
        &output_arg,
        "-s",
        "2026-02-01",
        "--iterations",
        "25",
        "--calendar-dir",
        &calendar_dir_path,
    ]);

    cmd.assert().success().stdout(
        predicate::str::contains("Simulation result written to")
            .and(predicate::str::contains("Simulation Report"))
            .and(predicate::str::contains("Percentile | Days | Date"))
            .and(predicate::str::contains("P85")),
    );

    let output = fs::read_to_string(output_arg).unwrap();
    assert!(output.contains("start_date: 2026-02-01"));
    assert!(output.contains("simulated_items:"));
    assert!(output.contains("p0:"));
}
