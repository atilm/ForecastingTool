use assert_fs::prelude::*;
use predicates::prelude::*;
use std::fs;

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
        .stdout(predicate::str::contains(format!("Simulation result for {number_of_issues_arg} items written to {output_arg}")));

    let output = std::fs::read_to_string(output_arg).unwrap();

    fs::remove_file(throughput_arg).unwrap();
    fs::remove_file(output_arg).unwrap();

    // The output should contain a report with the following structure:
    // report:
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
    assert!(output.contains("report:"));
    assert!(output.contains("start_date:"));
    assert!(output.contains("psimulated_items:"));
    assert!(output.contains("p0:"));
    assert!(output.contains("p50:"));
    assert!(output.contains("p85:"));
    assert!(output.contains("p100:"));
}
