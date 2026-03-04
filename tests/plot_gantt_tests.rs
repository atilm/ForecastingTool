use assert_fs::prelude::*;
use predicates::prelude::*;
use std::fs;

#[test]
fn plot_gantt_writes_mermaid_gantt_diagram() {
    let project_yaml = r#"
name: Demo
work_packages:
  - id: WP1
    summary: Design Phase
    dependencies: null
    estimate:
      type: three_point
      optimistic: 2
      most_likely: 5
      pessimistic: 14
  - id: WP2
    summary: Implementation
    dependencies: [WP1]
    estimate:
      type: three_point
      optimistic: 5
      most_likely: 10
      pessimistic: 21
  - id: WP3
    summary: Milestone
    dependencies: [WP2]
    estimate:
      type: three_point
      optimistic: 0
      most_likely: 0
      pessimistic: 0
"#;

    let input_file = assert_fs::NamedTempFile::new("project.yaml").unwrap();
    input_file.write_str(project_yaml).unwrap();
    let output_file = assert_fs::NamedTempFile::new("gantt.md").unwrap();

    let input_arg = input_file.path().to_str().unwrap().to_string();
    let output_arg = output_file.path().to_str().unwrap().to_string();

    let mut cmd = assert_cmd::cargo_bin_cmd!("forecasts");
    cmd.args(&[
        "plot-gantt",
        "-i",
        &input_arg,
        "-o",
        &output_arg,
        "-s",
        "2026-01-05",
    ]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Gantt diagram written to"));

    let output = fs::read_to_string(output_file.path()).unwrap();
    assert!(output.contains("```mermaid"));
    assert!(output.contains("gantt"));
    assert!(output.contains("# Demo Gantt Diagram"));
    assert!(output.contains("WP1 Design Phase"));
    assert!(output.contains("WP2 Implementation"));
    // WP3 is a milestone (0 duration)
    assert!(output.contains("milestone"));
    // All items on the critical path should be marked crit
    assert!(output.contains("crit"));
    // Start date is 2026-01-05
    assert!(output.contains("2026-01-05"));
}

#[test]
fn plot_gantt_with_calendar_dir() {
    let project_yaml = r#"
name: CalendarDemo
work_packages:
  - id: A
    summary: Task A
    dependencies: null
    estimate:
      type: three_point
      optimistic: 3
      most_likely: 3
      pessimistic: 3
  - id: B
    summary: Task B
    dependencies: [A]
    estimate:
      type: three_point
      optimistic: 2
      most_likely: 2
      pessimistic: 2
"#;

    // Calendar YAML with weekends free
    let calendar_yaml = r#"
free_weekdays: [Sat, Sun]
free_date_ranges: []
"#;

    let input_file = assert_fs::NamedTempFile::new("project.yaml").unwrap();
    input_file.write_str(project_yaml).unwrap();
    let output_file = assert_fs::NamedTempFile::new("gantt.md").unwrap();

    let calendar_dir = assert_fs::TempDir::new().unwrap();
    let calendar_file = calendar_dir.child("team.yaml");
    calendar_file.write_str(calendar_yaml).unwrap();

    let input_arg = input_file.path().to_str().unwrap().to_string();
    let output_arg = output_file.path().to_str().unwrap().to_string();
    let calendar_arg = calendar_dir.path().to_str().unwrap().to_string();

    let mut cmd = assert_cmd::cargo_bin_cmd!("forecasts");
    cmd.args(&[
        "plot-gantt",
        "-i",
        &input_arg,
        "-o",
        &output_arg,
        "-s",
        "2026-01-05",
        "-c",
        &calendar_arg,
    ]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Gantt diagram written to"));

    let output = fs::read_to_string(output_file.path()).unwrap();
    assert!(output.contains("```mermaid"));
    assert!(output.contains("gantt"));
    assert!(output.contains("# CalendarDemo Gantt Diagram"));
    assert!(output.contains("A Task A"));
    assert!(output.contains("B Task B"));
}
