use assert_fs::prelude::*;
use predicates::prelude::*;
use std::fs;
use tokio::task;

#[tokio::test]
async fn simulate_project_writes_output_and_histogram() {
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
      type: three_point
      optimistic: 1
      most_likely: 1
      pessimistic: 1
  - id: WP1
    estimate:
      type: story_points
      value: 3
    dependencies: []
  - id: FIN
    estimate:
      type: three_point
      optimistic: 0
      most_likely: 0
      pessimistic: 0
    dependencies: [WP0, WP1]
"#;

    let input_file = assert_fs::NamedTempFile::new("project.yaml").unwrap();
    input_file.write_str(project_yaml).unwrap();
    let output_file = assert_fs::NamedTempFile::new("simulation.yaml").unwrap();

    let input_arg = input_file.path().to_str().unwrap().to_string();
    let output_arg = output_file.path().to_str().unwrap().to_string();
    let histogram_path = format!("{output_arg}.png");

    task::spawn_blocking(move || {
        let mut cmd = assert_cmd::cargo_bin_cmd!("forecasts");
        cmd.args(&[
            "simulate",
            "-i",
            &input_arg,
            "-o",
            &output_arg,
            "-s",
            "2026-02-01",
            "--iterations",
            "25",
        ]);

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("Simulation result written to"));
    })
    .await
    .unwrap();

    let output = fs::read_to_string(output_file.path()).unwrap();
    assert!(output.contains("report:"));
    assert!(output.contains("results:"));
    assert!(output.contains("start_date: 2026-02-01"));

    assert!(fs::metadata(&histogram_path).is_ok());
}
