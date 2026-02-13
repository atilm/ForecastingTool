use assert_fs::prelude::*;
use predicates::prelude::*;
use std::fs;
use tokio::task;

#[tokio::test]
async fn plot_project_writes_markdown_diagram() {
    let project_yaml = r#"
name: Demo
work_packages:
  - id: WP1
    summary: Work package 1
    description: |
      This is
      work package 1.
    dependencies: null
    estimate:
      type: story_points
      value: 5
  - id: WP2
    summary: Work package 2
    dependencies: [WP1]
    estimate:
      type: three_point
      optimistic: 1
      most_likely: 5
      pessimistic: 10
"#;

    let input_file = assert_fs::NamedTempFile::new("project.yaml").unwrap();
    input_file.write_str(project_yaml).unwrap();
    let output_file = assert_fs::NamedTempFile::new("diagram.md").unwrap();

    let input_arg = input_file.path().to_str().unwrap().to_string();
    let output_arg = output_file.path().to_str().unwrap().to_string();

    task::spawn_blocking(move || {
        let mut cmd = assert_cmd::cargo_bin_cmd!("forecasts");
        cmd.args(&["plot-project", "-i", &input_arg, "-o", &output_arg]);

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("Project diagram written to"));
    })
    .await
    .unwrap();

    let output = fs::read_to_string(output_file.path()).unwrap();
    assert!(output.contains("```mermaid"));
    assert!(output.contains("flowchart TD"));
    assert!(output.contains("WP1 --> WP2"));
    assert!(output.contains("## WP1: Work package 1"));
}
