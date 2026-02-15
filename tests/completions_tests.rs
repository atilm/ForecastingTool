use predicates::prelude::*;

#[test]
fn completions_command_outputs_bash_script() {
    let mut cmd = assert_cmd::Command::cargo_bin("forecasts").unwrap();
    cmd.args(["completions", "bash"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("forecasts"));
}
