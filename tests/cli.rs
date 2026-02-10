use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use predicates::prelude::*;
use std::env;
use std::fs;
use tokio::task;
use warp::Filter;

#[test]
fn test_cli_help() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = assert_cmd::cargo_bin_cmd!("forecasts");
    cmd.arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Usage"));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_throughput_data() {
    let issues_response = serde_json::json!({
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
    });

    let issues_route = warp::path("search")
        .and(warp::path("jql"))
        .and(warp::get())
        .map(move || warp::reply::json(&issues_response));
    let (addr, server) = warp::serve(issues_route).bind_ephemeral(([127, 0, 0, 1], 0));
    tokio::spawn(server);

    let base_url = format!("http://{}", addr);
    let config_yaml = format!(
        r#"
base_url: {base_url}
project_key: MOCK
throughput_query: project = TEST
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

    let output_file = assert_fs::NamedTempFile::new("test_output.yaml").unwrap();
    let output_path = output_file.path();

    let config_arg = config_path.to_str().unwrap().to_string();
    let output_arg = output_path.to_str().unwrap().to_string();
    task::spawn_blocking(move || {
        let mut cmd = assert_cmd::cargo_bin_cmd!("forecasts");
        cmd.args(&[
            "get-throughput",
            "-c",
            &config_arg,
            "-o",
            &output_arg,
        ]);

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("Throughput data written to"));
    })
    .await
    .unwrap();

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
