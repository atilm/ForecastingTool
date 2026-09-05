use assert_fs::prelude::*;
use predicates::prelude::*;
use std::fs;

fn report_yaml() -> &'static str {
    r#"
data_source: integration
start_date: 2026-03-01
velocity: 2.0
iterations: 250
simulated_items: 3
p0: { days: 1, end_date: 2026-03-02 }
p15: { days: 1, end_date: 2026-03-02 }
p50: { days: 1, end_date: 2026-03-02 }
p85: { days: 1, end_date: 2026-03-02 }
p100: { days: 1, end_date: 2026-03-02 }
work_packages:
  - id: DONE-1
    type: Done
    estimate: { type: story_point, estimate: 3.0 }
    done_date: 2026-03-01
    percentiles: { p0: { days: 0, end_date: 2026-03-01 }, p15: { days: 0, end_date: 2026-03-01 }, p50: { days: 0, end_date: 2026-03-01 }, p85: { days: 0, end_date: 2026-03-01 }, p100: { days: 0, end_date: 2026-03-01 } }
  - id: TODO-1
    type: ToDo
    estimate: { type: story_point, estimate: 2.0 }
    done_date: null
    percentiles: { p0: { days: 1, end_date: 2026-03-02 }, p15: { days: 1, end_date: 2026-03-02 }, p50: { days: 2, end_date: 2026-03-03 }, p85: { days: 3, end_date: 2026-03-04 }, p100: { days: 4, end_date: 2026-03-05 } }
  - id: __generated_story_point_1
    type: DynamicToDo
    estimate: { type: story_point, estimate: 5.0 }
    done_date: null
    percentiles: { p0: { days: 2, end_date: 2026-03-03 }, p15: { days: 2, end_date: 2026-03-03 }, p50: { days: 3, end_date: 2026-03-04 }, p85: { days: 4, end_date: 2026-03-05 }, p100: { days: 5, end_date: 2026-03-06 } }
"#
}

#[test]
fn plot_burndown_creates_png_from_report_with_dynamic_work() {
    let temp = assert_fs::TempDir::new().unwrap();
    let report_file = temp.child("result.yaml");
    report_file.write_str(report_yaml()).unwrap();
    let output_file = temp.child("burndown.png");

    let mut command = assert_cmd::cargo_bin_cmd!("forecasts");
    command.args([
        "plot",
        "burndown",
        "--report",
        report_file.path().to_str().unwrap(),
        "--output",
        output_file.path().to_str().unwrap(),
    ]);
    command
        .assert()
        .success()
        .stdout(predicate::str::contains("Burndown plot written to"));
    assert!(fs::metadata(output_file.path()).unwrap().len() > 0);
}

#[test]
fn plot_burndown_rejects_removed_project_input_option() {
    let mut command = assert_cmd::cargo_bin_cmd!("forecasts");
    command.args(["plot", "burndown", "--input", "project.yaml"]);
    command
        .assert()
        .failure()
        .stderr(predicate::str::contains("--input"));
}
