use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use chrono::NaiveDate;
use forecasts::domain::throughput::Throughput;
use forecasts::services::throughput_yaml::serialize_throughput_to_yaml;
use predicates::prelude::*;
use std::env;
use std::fs;
use std::net::SocketAddr;
use std::sync::Mutex;

#[test]
fn test_cli_help() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = assert_cmd::cargo_bin_cmd!("forecasts");
    cmd.arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Usage"));
    Ok(())
}

#[tokio::test]
async fn test_get_throughput_data() {
    let mut server = mockito::Server::new_async().await;
    let url = server.url();

    server
        .mock("GET", "/search/jql")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
  "isLast": false,
  "issues": [
    {
      "fields": {
        "created": "2026-01-12T10:13:04.983+0100",
        "actualStartDate": "2026-01-22T10:57:00.000+0100",
        "actualEndDate": "2026-01-26T08:42:00.000+0100",
        "description": { 
          "content": [
            {
              "content": [
                {
                  "text": "A description text.",
                  "type": "text"
                }
              ],
              "type": "paragraph"
            }
          ],
          "type": "doc",
          "version": 1
        },
        "status": {
          "name": "Done",
          "statusCategory": {
            "name": "Done"
          }
        },
        "summary": "A first task"
      },
      "key": "ABC-123"
    },
    {
      "fields": {
        "created": "2026-01-11T10:13:04.983+0100",
        "actualStartDate": "2026-01-22T10:57:00.000+0100",
        "actualEndDate": "2026-01-28T08:42:00.000+0100",
        "description": { 
          "content": [
            {
              "content": [
                {
                  "text": "Another description text.",
                  "type": "text"
                }
              ],
              "type": "paragraph"
            }
          ],
          "type": "doc",
          "version": 1
        },
        "status": {
          "name": "Done",
          "statusCategory": {
            "name": "Done"
          }
        },
        "summary": "A second task"
      },
      "key": "ABC-124"
    }
  ],
  "nextPageToken": "ChkjU3RyaW5nJlNVRlhSVlE9JUludCZPQT09EDIY8sP-ncQzIkpwcm9qZWN0ID0gSUFXRVQgQU5EIHR5cGUgSU4gKFN0b3J5LCAiUHJvZHVjdGlvbiBEZWZlY3QiKSBBTkQgc3RhdHVzID0gRG9uZSoCW10="
}"#,
        ).create_async().await;

    let base_url = url;
    let config_yaml = format!(
        r#"
base_url: {base_url}
project_key: MOCK
estimation_field_id: estimate
start_date_field_id: startDate
actual_start_date_field_id: actualStartDate
actual_end_date_field_id: actualEndDate
"#
    );
    let config_file = assert_fs::NamedTempFile::new("test_jira_config.yaml").unwrap();
    let config_path = config_file.path();
    config_file.write_str(&config_yaml).unwrap();

    unsafe {
        env::set_var("JIRA_USERNAME", "mockuser");
        env::set_var("JIRA_API_TOKEN", "mocktoken");
    }

    let output_path = "test_output.yaml";

    let mut cmd = assert_cmd::cargo_bin_cmd!("forecasts");
    cmd.args(&[
        "get-throughput",
        "-j",
        "project = TEST",
        "-c",
        config_path.to_str().unwrap(),
        "-o",
        output_path,
    ]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Throughput data written to"));

    let output = fs::read_to_string(output_path).unwrap();
    assert!(output.contains("2026-01-26"));
    assert!(output.contains("completed_issues: 1"));
    assert!(output.contains("2026-01-27"));
    assert!(output.contains("completed_issues: 0"));
    assert!(output.contains("2026-01-28"));
    assert!(output.contains("completed_issues: 1"));

    // Cleanup
    let _ = fs::remove_file(config_path);
    let _ = fs::remove_file(output_path);
}
