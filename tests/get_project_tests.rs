use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use predicates::prelude::*;
use std::env;
use std::fs;
use tokio::task;
use warp::Filter;

#[tokio::test()]
async fn test_get_project_data() {
    let issues_response = serde_json::json!({
        "issues": [
            {
                "fields": {
                    "created": "2026-01-12T10:13:04.983+0100",
                    "actualStartDate": "2026-01-22T10:57:00.000+0100",
                    "actualEndDate": "2026-01-26T08:42:00.000+0100",
                    "estimate": 5,
                    "description": "A description text.",
                    "statusCategory": {
                        "name": "Done"
                    },
                    "summary": "A first task"
                },
                "key": "ABC-123"
            },
            {
                "fields": {
                    "created": "2026-01-12T10:13:04.983+0100",
                    "actualStartDate": "2026-01-22T10:57:00.000+0100",
                    "actualEndDate": "2026-01-26T08:42:00.000+0100",
                    "description": "A description text.",
                    "statusCategory": {
                        "name": "In Progress"
                    },
                    "summary": "A second task"
                },
                "key": "ABC-456"
            }
        ]
    });

    let issues_route = warp::path("search")
        .and(warp::path("jql"))
        .and(warp::get())
        .map(move || warp::reply::json(&issues_response));
    let (addr, server) = warp::serve(issues_route).bind_ephemeral(([127, 0, 0, 1], 0));
    tokio::spawn(server);

    let output = run_get_project(addr).await.unwrap();

    assert!(output.contains("name: MOCK"));
    assert!(output.contains("id: ABC-123"));
    assert!(output.contains("summary: A first task"));
    assert!(output.contains("description: A description text."));
    assert!(output.contains("type: story_points"));
    assert!(output.contains("value: 5"));
    assert!(output.contains("created_date: 2026-01-12"));
    assert!(output.contains("start_date: 2026-01-22"));
    assert!(output.contains("done_date: 2026-01-26"));
    assert!(output.contains("id: ABC-456"));
    assert!(output.contains("summary: A second task"));
    assert!(output.contains("dependencies: null")); // First issue should have null dependencies
    assert!(output.contains("dependencies: []")); // Second issue should have empty dependencies
}

async fn run_get_project(socket_addr: std::net::SocketAddr) -> Result<String, Box<dyn std::error::Error>> {
    let base_url = format!("http://{}", socket_addr);
    let config_yaml = format!(
        r#"
base_url: {base_url}
project_key: MOCK
throughput_query: project = TEST
project_query: project = TEST
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
        cmd.args(&["get-project", "-c", &config_arg, "-o", &output_arg]);

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("Project data written to"));
    })
    .await
    .unwrap();

    let output = fs::read_to_string(output_path).unwrap();

    let _ = fs::remove_file(config_path);
    let _ = fs::remove_file(output_path);

    Ok(output)
}
