use assert_fs::prelude::*;
use predicates::prelude::*;
use std::fs;
use serde_yaml::Value;

#[tokio::test()]
async fn simulate_by_throughput() {
    let throughput_yaml = "- date: 2026-01-26
  completed_issues: 2
- date: 2026-01-27
  completed_issues: 0
- date: 2026-01-28
  completed_issues: 0
- date: 2026-01-29
  completed_issues: 0
- date: 2026-01-30
  completed_issues: 1";

    let thoughput_file_name = "test_throughput.yaml";

    let throughput_file = assert_fs::NamedTempFile::new(&thoughput_file_name).unwrap();
    throughput_file.write_str(throughput_yaml).unwrap();
    let throughput_arg = throughput_file.path().to_str().unwrap();
    
    let output_file = assert_fs::NamedTempFile::new("output.yaml").unwrap();
    let output_arg = output_file.path().to_str().unwrap();

    let iterations_arg = "5";
    let number_of_issues_arg = "10";
    let start_date_arg = "2026-01-30";

    let mut cmd = assert_cmd::cargo_bin_cmd!("forecasts");
    cmd.args(&["simulate-n", "-f", &throughput_arg, "-o", &output_arg, "-i", &iterations_arg, "-n", &number_of_issues_arg, "-s", &start_date_arg]);

    cmd.assert()
        .success()
      .stdout(
        predicate::str::contains(format!(
          "Simulation result for {number_of_issues_arg} items written to {output_arg}"
        ))
        .and(predicate::str::contains("Simulation Report"))
        .and(predicate::str::contains("Percentile | Days | Date"))
        .and(predicate::str::contains("P50")),
      );

    let output = std::fs::read_to_string(output_arg).unwrap();

    fs::remove_file(throughput_arg).unwrap();
    fs::remove_file(output_arg).unwrap();

    // The output should contain a report with the following structure:
    //  start_date: 2026-01-30
    //  simulated_items: 10
    //  p0: 
    //    days: 1
    //    date: 2026-01-31
    //  p50:
    //    days: 5
    //    date: 2026-02-04
    //  p85:
    //    days: 10
    //    date: 2026-02-09
    //  p100:
    //    days: 20
    //    date: 2026-02-19
    assert!(output.contains("start_date:"));
    assert!(output.contains("simulated_items:"));
    assert!(output.contains("p0:"));
    assert!(output.contains("p50:"));
    assert!(output.contains("p85:"));
    assert!(output.contains("p100:"));
}

#[tokio::test()]
async fn simulate_by_throughput_parses_calendar_dir() {
  // Deterministic throughput distribution: always 1 issue/day.
  let throughput_yaml = "- date: 2026-02-16\n  completed_issues: 1\n";

  let throughput_file = assert_fs::NamedTempFile::new("test_throughput.yaml").unwrap();
  throughput_file.write_str(throughput_yaml).unwrap();

  let output_file = assert_fs::NamedTempFile::new("output.yaml").unwrap();

  // Calendar makes Mondays free (capacity 0), so starting on Monday should delay completion.
  let calendar_dir = assert_fs::TempDir::new().unwrap();
  calendar_dir
    .child("team.yaml")
    .write_str("free_weekdays: [Mon]\n")
    .unwrap();

  let mut cmd = assert_cmd::cargo_bin_cmd!("forecasts");
  cmd.args(&[
    "simulate-n",
    "-f",
    throughput_file.path().to_str().unwrap(),
    "-o",
    output_file.path().to_str().unwrap(),
    "-i",
    "1",
    "-n",
    "2",
    "-s",
    "2026-02-16",
    "-c",
    calendar_dir.path().to_str().unwrap(),
  ]);

  cmd.assert().success();

  let output = std::fs::read_to_string(output_file.path()).unwrap();
  let value: Value = serde_yaml::from_str(&output).unwrap();
  let p50_days = value
    .get("p50")
    .and_then(|v| v.get("days"))
    .and_then(|v| v.as_f64())
    .unwrap();
  let p50_date = value
    .get("p50")
    .and_then(|v| v.get("date"))
    .and_then(|v| v.as_str())
    .unwrap();

  // Day 1 (Mon): capacity 0 => 0 progress; Day 2 (Tue): 1; Day 3 (Wed): 1 => done.
  assert_eq!(p50_days, 3.0);
  assert_eq!(p50_date, "2026-02-19");
}
