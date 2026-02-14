use assert_fs::prelude::*;
use predicates::prelude::*;
use std::fs;
use tokio::task;

#[tokio::test]
async fn simulate_project_writes_output_and_histogram() {
    let report_yaml = r#"
data_source: "unit"
start_date: "2026-01-01"
velocity: 10.0
iterations: 25
simulated_items: 100
p0:
  days: 5.0
  date: "2026-01-06"
p50:
  days: 10.0
  date: "2026-01-11"
p85:
  days: 15.0
  date: "2026-01-16"
p100:
  days: 20.0
  date: "2026-01-21" 
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
    let gantt_path = format!("{output_arg}.gantt.md");

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

        cmd.assert().success().stdout(
            predicate::str::contains("Simulation result written to")
                .and(predicate::str::contains("Simulation Report"))
                .and(predicate::str::contains("Percentile | Days | Date"))
                .and(predicate::str::contains("P85")),
        );
    })
    .await
    .unwrap();

    let output = fs::read_to_string(output_file.path()).unwrap();
    assert!(output.contains("start_date: 2026-02-01"));
    assert!(output.contains("simulated_items:"));
    assert!(output.contains("p0:"));

    assert!(fs::metadata(&histogram_path).is_ok());
    assert!(fs::metadata(&gantt_path).is_ok());
    let gantt_output = fs::read_to_string(&gantt_path).unwrap();
    assert!(gantt_output.contains("gantt"));
    assert!(gantt_output.contains("dateFormat"));
}
