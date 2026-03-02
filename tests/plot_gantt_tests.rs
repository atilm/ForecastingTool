use assert_fs::prelude::*;
use predicates::prelude::*;
use std::fs;
use tokio::task;

#[tokio::test]
async fn plot_gantt_writes_mermaid_gantt_diagram() {
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

    task::spawn_blocking(move || {
        let mut cmd = assert_cmd::cargo_bin_cmd!("forecasts");
        cmd.args(&["plot-gantt", "-i", &input_arg, "-o", &output_arg]);

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("Gantt diagram written to"));
    })
    .await
    .unwrap();

    let output = fs::read_to_string(output_file.path()).unwrap();
    assert!(output.contains("```mermaid"));
    assert!(output.contains("gantt"));
    assert!(output.contains("# Demo Gantt Diagram"));
    assert!(output.contains("Design Phase"));
    assert!(output.contains("Implementation"));
    // WP2 should depend on WP1
    assert!(output.contains("after WP1"));
    // WP3 is a milestone (0 duration)
    assert!(output.contains("milestone"));
    // (2 + 4*5 + 14) / 6 = 36/6 = 6 days for WP1
    assert!(output.contains("WP1, 6d"));
    // (5 + 4*10 + 21) / 6 = 66/6 = 11 days for WP2
    assert!(output.contains("WP2, after WP1, 11d"));
}
