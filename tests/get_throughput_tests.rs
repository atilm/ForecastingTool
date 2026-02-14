use assert_fs::prelude::*;
use predicates::prelude::*;
use std::collections::HashMap;
use std::env;
use std::fs;
use tokio::task;
use warp::Filter;

#[tokio::test()]
async fn test_get_throughput_data() {
    let issues_response = serde_json::json!({
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
                    "statusCategory": {
                        "name": "Done"
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
                    "statusCategory": {
                        "name": "Done"
                    },
                    "summary": "A second task"
                },
                "key": "ABC-124"
            }
        ]
    });

    let issues_route = warp::path("search")
        .and(warp::path("jql"))
        .and(warp::get())
        .map(move || warp::reply::json(&issues_response));
    let (addr, server) = warp::serve(issues_route).bind_ephemeral(([127, 0, 0, 1], 0));
    tokio::spawn(server);

    // Act
    let output = run_get_throughput(addr).await.unwrap();

    // Assert
    assert!(output.contains("2026-01-26"));
    assert!(output.contains("completed_issues: 1"));
    assert!(output.contains("2026-01-27"));
    assert!(output.contains("completed_issues: 0"));
    assert!(output.contains("2026-01-28"));
    assert!(output.contains("completed_issues: 1"));
}

#[cfg(test)]
#[tokio::test]
async fn get_issues_paginates_start_at() {
    let issues_page1 = serde_json::json!({
        "issues": [
            {
                "fields": {
                    "created": "2026-01-12T10:13:04.983+0100",
                    "actualStartDate": "2026-01-22T10:57:00.000+0100",
                    "actualEndDate": "2026-01-26T08:42:00.000+0100",
                    "description": "First description.",
                    "statusCategory": {
                        "name": "Done"
                    },
                    "summary": "A first task"
                },
                "key": "ABC-123"
            }
        ],
        "startAt": 0,
        "maxResults": 1,
        "total": 2
    });

    let issues_page2 = serde_json::json!({
        "issues": [
            {
                "fields": {
                    "created": "2026-01-11T10:13:04.983+0100",
                    "actualStartDate": "2026-01-22T10:57:00.000+0100",
                    "actualEndDate": "2026-01-28T08:42:00.000+0100",
                    "description": "Second description.",
                    "statusCategory": {
                        "name": "Done"
                    },
                    "summary": "A second task"
                },
                "key": "ABC-124"
            }
        ],
        "startAt": 1,
        "maxResults": 1,
        "total": 2
    });

    let issues_route = warp::path("search")
        .and(warp::path("jql"))
        .and(warp::get())
        .and(warp::query::<HashMap<String, String>>())
        .map(move |query: HashMap<String, String>| {
            if query.get("startAt").map(|value| value.as_str()) == Some("1") {
                warp::reply::json(&issues_page2)
            } else {
                warp::reply::json(&issues_page1)
            }
        });

    let (addr, server) = warp::serve(issues_route).bind_ephemeral(([127, 0, 0, 1], 0));
    tokio::spawn(server);

    // Act
    let output = run_get_throughput(addr).await.unwrap();

    // Assert
    assert!(output.contains("2026-01-26"));
    assert!(output.contains("completed_issues: 1"));
    assert!(output.contains("2026-01-27"));
    assert!(output.contains("completed_issues: 0"));
    assert!(output.contains("2026-01-28"));
    assert!(output.contains("completed_issues: 1"));
}


async fn run_get_throughput(socket_addr: std::net::SocketAddr) -> Result<String, Box<dyn std::error::Error>> {
    let base_url = format!("http://{}", socket_addr);
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
        cmd.args(&["get-throughput", "-c", &config_arg, "-o", &output_arg]);

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("Throughput data written to"));
    })
    .await
    .unwrap();

    let output = fs::read_to_string(output_path).unwrap();

    // Cleanup
    let _ = fs::remove_file(config_path);
    let _ = fs::remove_file(output_path);

    Ok(output)
}
