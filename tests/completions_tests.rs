use predicates::prelude::*;

#[test]
fn completions_command_outputs_bash_script() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("forecasts");
    cmd.args(["completions", "bash"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("forecasts"));
}
