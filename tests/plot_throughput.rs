use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use predicates::prelude::*;
use std::fs;

#[tokio::test]
async fn plot_throughput_creates_png() {
    let throughput_yaml = "- date: 2026-01-26\n  completed_issues: 2\n- date: 2026-01-27\n  completed_issues: 0\n- date: 2026-01-28\n  completed_issues: 3\n";

    let input_file = assert_fs::NamedTempFile::new("throughput.yaml").unwrap();
    input_file.write_str(throughput_yaml).unwrap();
    let output_file = assert_fs::NamedTempFile::new("throughput.png").unwrap();

    let input_arg = input_file.path().to_str().unwrap().to_string();
    let output_arg = output_file.path().to_str().unwrap().to_string();

    let mut cmd = assert_cmd::cargo_bin_cmd!("forecasts");
    cmd.args(&["plot-throughput", "-i", &input_arg, "-o", &output_arg]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Throughput plot written to"));

    let metadata = fs::metadata(output_arg).unwrap();
    assert!(metadata.len() > 0);
}
