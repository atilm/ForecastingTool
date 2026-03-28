use assert_fs::prelude::*;
use predicates::prelude::*;
use std::fs;

#[test]
fn plot_simulation_gantt_writes_mermaid_gantt_diagram() {
    let project_yaml = r#"
name: SimDemo
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
  - id: MS1
    summary: Release
    dependencies: [WP2]
    estimate:
      type: milestone
"#;

    let report_yaml = r#"
data_source: "test"
start_date: "2026-01-05"
velocity: 1.0
iterations: 1000
simulated_items: 3
p0:
  days: 5.0
  end_date: "2026-01-10"
p15:
  days: 7.0
  end_date: "2026-01-12"
p50:
  days: 10.0
  end_date: "2026-01-15"
p85:
  days: 14.0
  end_date: "2026-01-19"
p100:
  days: 20.0
  end_date: "2026-01-25"
work_packages:
  - id: WP1
    is_milestone: false
    percentiles:
      p0:
        days: 5.0
        end_date: "2026-01-10"
      p15:
        days: 6.0
        end_date: "2026-01-11"
      p50:
        days: 7.0
        end_date: "2026-01-12"
      p85:
        days: 10.0
        end_date: "2026-01-15"
      p100:
        days: 14.0
        end_date: "2026-01-19"
  - id: WP2
    is_milestone: false
    percentiles:
      p0:
        days: 5.0
        end_date: "2026-01-20"
      p15:
        days: 6.0
        end_date: "2026-01-21"
      p50:
        days: 8.0
        end_date: "2026-01-23"
      p85:
        days: 10.0
        end_date: "2026-01-25"
      p100:
        days: 14.0
        end_date: "2026-01-29"
  - id: MS1
    is_milestone: true
    percentiles:
      p0:
        days: 0.0
        end_date: "2026-01-25"
      p15:
        days: 0.0
        end_date: "2026-01-25"
      p50:
        days: 0.0
        end_date: "2026-01-25"
      p85:
        days: 0.0
        end_date: "2026-01-25"
      p100:
        days: 0.0
        end_date: "2026-01-25"
"#;

    let project_file = assert_fs::NamedTempFile::new("project.yaml").unwrap();
    project_file.write_str(project_yaml).unwrap();

    let report_file = assert_fs::NamedTempFile::new("report.yaml").unwrap();
    report_file.write_str(report_yaml).unwrap();

    let output_file = assert_fs::NamedTempFile::new("gantt.md").unwrap();

    let mut cmd = assert_cmd::cargo_bin_cmd!("forecasts");
    cmd.args(&[
        "plot",
        "simulation-gantt",
        "-i",
        project_file.path().to_str().unwrap(),
        "-r",
        report_file.path().to_str().unwrap(),
        "-o",
        output_file.path().to_str().unwrap(),
    ]);

    cmd.assert().success().stdout(predicate::str::contains(
        "Simulation Gantt diagram written to",
    ));

    let output = fs::read_to_string(output_file.path()).unwrap();

    assert!(output.contains("```mermaid"));
    assert!(output.contains("gantt"));
    assert!(output.contains("# SimDemo Simulation Gantt Diagram"));

    // WP1 has no deps → starts at report start_date 2026-01-05, ends at p85 2026-01-15
    assert!(output.contains("WP1 Design Phase"));
    assert!(output.contains(":WP1, 2026-01-05, 2026-01-15"));

    // WP2 depends on WP1 → starts at WP1's p85 2026-01-15, ends at its own p85 2026-01-25
    assert!(output.contains("WP2 Implementation"));
    assert!(output.contains(":WP2, 2026-01-15, 2026-01-25"));

    // MS1 is a milestone → rendered with milestone syntax, date = p85 end_date 2026-01-25
    assert!(output.contains("MS1 Release"));
    assert!(output.contains(":milestone, MS1, 2026-01-25, 0d"));
}
