use assert_cmd::prelude::*;
use predicates::prelude::*;

#[test]
fn test_cli_help() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = assert_cmd::cargo_bin_cmd!("forecasts");
    cmd.arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Usage"));
    Ok(())
}
