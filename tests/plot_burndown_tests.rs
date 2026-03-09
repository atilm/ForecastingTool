use assert_fs::prelude::*;
use predicates::prelude::*;
use std::fs;

#[test]
fn plot_burndown_creates_png() {
    let project_yaml = r#"
name: Demo
work_packages:
  - id: DONE-1
    status: Done
    done_date: 2026-03-01
    estimate:
      type: story_points
      value: 3
  - id: TODO-1
    status: ToDo
  - id: INP-1
    status: InProgress
    estimate:
      type: story_points
      value: 2
"#;

    let report_yaml = r#"
data_source: unit
start_date: 2026-03-01
velocity: 2.0
iterations: 250
simulated_items: 3
p0:
  days: 1
  end_date: 2026-03-02
p15:
  days: 2
  end_date: 2026-03-03
p50:
  days: 4
  end_date: 2026-03-05
p85:
  days: 6
  end_date: 2026-03-07
p100:
  days: 8
  end_date: 2026-03-09
work_packages:
  - id: TODO-1
    is_milestone: false
    percentiles:
      p0:
        days: 1
        end_date: 2026-03-02
      p15:
        days: 2
        end_date: 2026-03-03
      p50:
        days: 3
        end_date: 2026-03-04
      p85:
        days: 5
        end_date: 2026-03-06
      p100:
        days: 6
        end_date: 2026-03-07
  - id: INP-1
    is_milestone: false
    percentiles:
      p0:
        days: 2
        end_date: 2026-03-03
      p15:
        days: 3
        end_date: 2026-03-04
      p50:
        days: 4
        end_date: 2026-03-05
      p85:
        days: 6
        end_date: 2026-03-07
      p100:
        days: 7
        end_date: 2026-03-08
"#;

    let project_file = assert_fs::NamedTempFile::new("project.yaml").unwrap();
    project_file.write_str(project_yaml).unwrap();
    let report_file = assert_fs::NamedTempFile::new("result.yaml").unwrap();
    report_file.write_str(report_yaml).unwrap();
    let output_file = assert_fs::NamedTempFile::new("burndown.png").unwrap();

    let project_arg = project_file.path().to_str().unwrap().to_string();
    let report_arg = report_file.path().to_str().unwrap().to_string();
    let output_arg = output_file.path().to_str().unwrap().to_string();

    let mut cmd = assert_cmd::cargo_bin_cmd!("forecasts");
    cmd.args(&[
        "plot-burndown",
        "-i",
        &project_arg,
        "-r",
        &report_arg,
        "-o",
        &output_arg,
    ]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Burndown plot written to"));

    let metadata = fs::metadata(output_arg).unwrap();
    assert!(metadata.len() > 0);
}
