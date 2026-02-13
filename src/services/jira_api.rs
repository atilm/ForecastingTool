use std::collections::HashMap;
use std::env;
use std::fs;

use chrono::NaiveDate;
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use serde_json::Value;

use crate::domain::epic::Epic;
use crate::domain::estimate::{Estimate, StoryPointEstimate};
use crate::domain::issue::IssueId;
use crate::domain::project::Project;
use crate::domain::issue::{Issue, IssueStatus};
use crate::services::data_source::{DataSource, DataQuery, DataSourceError};

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct JiraProjectMetaData {
    pub base_url: String,
    pub project_key: String,
    pub throughput_query: String,
    pub project_query: String,
    pub estimation_field_id: String,
    pub start_date_field_id: String,
    pub actual_start_date_field_id: String,
    pub actual_end_date_field_id: String,
}

impl Default for JiraProjectMetaData {
    fn default() -> Self {
        Self {
            base_url: String::new(),
            project_key: String::new(),
            throughput_query: String::new(),
            project_query: String::new(),
            estimation_field_id: String::new(),
            start_date_field_id: String::new(),
            actual_start_date_field_id: String::new(),
            actual_end_date_field_id: String::new(),
        }
    }
}

impl JiraProjectMetaData {
    pub fn from_yaml_file(filepath: &str) -> Result<Self, DataSourceError> {
        let contents = fs::read_to_string(filepath)
            .map_err(|err| DataSourceError::Other(format!("failed to read config: {err}")))?;
        let metadata: JiraProjectMetaData =
            serde_yaml::from_str(&contents).map_err(|_| DataSourceError::Parse)?;
        Ok(metadata)
    }
}

pub struct JiraConfigParser;

impl JiraConfigParser {
    pub fn parse(&self, filepath: &str) -> Result<JiraProjectMetaData, DataSourceError> {
        JiraProjectMetaData::from_yaml_file(filepath)
    }
}

#[derive(Debug, Clone)]
pub struct AuthData {
    pub username: String,
    pub api_token: String,
}

impl AuthData {
    pub fn from_env() -> Result<Self, DataSourceError> {
        let username = env::var("JIRA_USERNAME").ok();
        let api_token = env::var("JIRA_API_TOKEN").ok();
        match (username, api_token) {
            (Some(username), Some(api_token)) => Ok(Self {
                username,
                api_token,
            }),
            _ => Err(DataSourceError::Unauthorized),
        }
    }
}

pub struct JiraApiClient {
    jira_project: JiraProjectMetaData,
    auth: AuthData,
    client: Client,
}

impl JiraApiClient {
    pub fn new(jira_project: JiraProjectMetaData, auth: AuthData) -> Result<Self, DataSourceError> {
        if jira_project.base_url.is_empty() || jira_project.project_key.is_empty() {
            return Err(DataSourceError::Other(
                "jira_project metadata is missing base_url or project_key".to_string(),
            ));
        }

        Ok(Self {
            jira_project,
            auth,
            client: Client::new(),
        })
    }

    async fn fetch_json(
        &self,
        url: &str,
        params: &HashMap<&str, String>,
    ) -> Result<Value, DataSourceError> {
        let response = self
            .client
            .get(url)
            .query(params)
            .basic_auth(
                self.auth.username.clone(),
                Some(self.auth.api_token.clone()),
            )
            .send()
            .await
            .map_err(|_| DataSourceError::Connection)?;

        let status = response.status();
        if status == StatusCode::UNAUTHORIZED {
            return Err(DataSourceError::Unauthorized);
        }
        if status == StatusCode::NOT_FOUND {
            return Err(DataSourceError::NotFound);
        }
        if !status.is_success() {
            return Err(DataSourceError::Connection);
        }

        response
            .json::<Value>()
            .await
            .map_err(|_| DataSourceError::Parse)
    }

    async fn get_issues_by_jql(&self, jql: &str) -> Result<Vec<Issue>, DataSourceError> {
        let url = format!("{}/search/jql", self.jira_project.base_url);
        let fields = format!(
            "summary,description,statusCategory,created,{},{},{}",
            self.jira_project.actual_start_date_field_id,
            self.jira_project.actual_end_date_field_id,
            self.jira_project.estimation_field_id
        );
        let mut params = HashMap::new();
        params.insert("jql", jql.to_string());
        params.insert("fields", fields);

        let mut mapped = Vec::new();
        let mut last_page_token: Option<String> = None;

        loop {
            let payload = self.fetch_json(&url, &params).await?;

            let issues = payload
                .get("issues")
                .and_then(|value| value.as_array())
                .ok_or(DataSourceError::Parse)?;

            for issue in issues {
                if let Some(issue_obj) = issue.as_object() {
                    let mapped_issue = self.map_issue(issue_obj)?;
                    mapped.push(mapped_issue);
                }
            }

            if let Some(token) = payload.get("nextPageToken").and_then(|value| value.as_str()) {
                if last_page_token.as_deref() == Some(token) {
                    break;
                }
                last_page_token = Some(token.to_string());
                params.insert("nextPageToken", token.to_string());
                params.remove("startAt");
                continue;
            }

            if payload
                .get("isLast")
                .and_then(|value| value.as_bool())
                .unwrap_or(false)
            {
                break;
            }

            let start_at = payload.get("startAt").and_then(|value| value.as_u64());
            let max_results = payload.get("maxResults").and_then(|value| value.as_u64());
            let total = payload.get("total").and_then(|value| value.as_u64());

            if let (Some(start_at), Some(max_results), Some(total)) =
                (start_at, max_results, total)
            {
                let next_start_at = start_at.saturating_add(max_results);
                if next_start_at >= total {
                    break;
                }
                params.remove("nextPageToken");
                params.insert("startAt", next_start_at.to_string());
                continue;
            }

            break;
        }

        Ok(mapped)
    }

    fn map_issue(&self, issue: &serde_json::Map<String, Value>) -> Result<Issue, DataSourceError> {
        let key = issue
            .get("key")
            .and_then(|value| value.as_str())
            .ok_or(DataSourceError::Parse)?;
        let fields = issue
            .get("fields")
            .and_then(|value| value.as_object())
            .ok_or(DataSourceError::Parse)?;

        let mut mapped = Issue::new();
        mapped.issue_id = Some(IssueId {
            id: key.to_string(),
        });
        mapped.summary = get_field_string(fields, "summary");
        mapped.description = get_field_description(fields, "description");
        mapped.status = get_field_status_category(fields);
        mapped.created_date = parse_date_opt(get_field_string(fields, "created").as_deref());
        mapped.estimate = get_field_f32(fields, &self.jira_project.estimation_field_id).map(
            |value| Estimate::StoryPoint(StoryPointEstimate {
                estimate: Some(value),
            }),
        );
        mapped.start_date = parse_date_opt(
            get_field_string(fields, &self.jira_project.actual_start_date_field_id).as_deref(),
        );
        mapped.done_date = parse_date_opt(
            get_field_string(fields, &self.jira_project.actual_end_date_field_id).as_deref(),
        );
        Ok(mapped)
    }
}

#[async_trait::async_trait]
impl DataSource for JiraApiClient {
    async fn get_epic(&self, epic_id: &str) -> Result<Epic, DataSourceError> {
        let url = format!("{}/issue/{epic_id}", self.jira_project.base_url);
        let fields = format!(
            "summary,description,statusCategory,{},duedate",
            self.jira_project.start_date_field_id
        );
        let mut params = HashMap::new();
        params.insert("fields", fields);

        let payload = self.fetch_json(&url, &params).await?;
        let fields = payload
            .get("fields")
            .and_then(|value| value.as_object())
            .ok_or(DataSourceError::Parse)?;

        let children_of_epic_jql = format!("\"Epic Link\"={epic_id}");
        let issues_of_epic = self.get_issues_by_jql(&children_of_epic_jql).await?;

        let mut epic = Epic::new();
        epic.issue_id = Some(IssueId { id: epic_id.to_string()} );
        epic.summary = get_field_string(fields, "summary");
        epic.description = get_field_description(fields, "description");
        epic.status = get_field_status_category(fields);
        epic.start_date = parse_date_opt(
            get_field_string(fields, &self.jira_project.start_date_field_id).as_deref(),
        );
        epic.due_date = parse_date_opt(get_field_string(fields, "duedate").as_deref());
        epic.issues = issues_of_epic;

        Ok(epic)
    }

    async fn get_issues(&self, query: DataQuery) -> Result<Vec<Issue>, DataSourceError> {
        match query {
            DataQuery::StringQuery(jql) => self.get_issues_by_jql(&jql).await,
        }
    }

    async fn get_project(&self, query: DataQuery) -> Result<Project, DataSourceError> {
        match query {
            DataQuery::StringQuery(jql) => {
                let issues = self.get_issues_by_jql(&jql).await?;
                Ok(crate::domain::project::Project {
                    name: self.jira_project.project_key.clone(),
                    work_packages: issues,
                })
            }
        }
    }
}

fn get_field_string(fields: &serde_json::Map<String, Value>, key: &str) -> Option<String> {
    fields.get(key).and_then(|value| match value {
        Value::String(text) => Some(text.clone()),
        Value::Null => None,
        _ => None,
    })
}

fn get_field_f32(fields: &serde_json::Map<String, Value>, key: &str) -> Option<f32> {
    fields.get(key).and_then(|value| match value {
        Value::Number(number) => number.as_f64().map(|value| value as f32),
        Value::String(text) => text.parse::<f32>().ok(),
        Value::Null => None,
        _ => None,
    })
}

fn get_field_description(fields: &serde_json::Map<String, Value>, key: &str) -> Option<String> {
    fields.get(key).and_then(|value| match value {
        Value::String(text) => Some(text.clone()),
        Value::Object(_) => {
            let text = adf_to_text(value);
            if text.is_empty() { None } else { Some(text) }
        }
        _ => None,
    })
}

fn get_field_status_category(fields: &serde_json::Map<String, Value>) -> Option<IssueStatus> {
    let status_name = fields
        .get("statusCategory")
        .and_then(|value| value.get("name"))
        .and_then(|value| value.as_str());
    match status_name.map(|value| value.to_ascii_lowercase()) {
        Some(value) if value == "to do" => Some(IssueStatus::ToDo),
        Some(value) if value == "in progress" => Some(IssueStatus::InProgress),
        Some(value) if value == "done" => Some(IssueStatus::Done),
        _ => None,
    }
}

fn parse_date_opt(value: Option<&str>) -> Option<NaiveDate> {
    let text = value?;
    let date = if let Some((date_part, _)) = text.split_once('T') {
        date_part
    } else {
        text
    };
    NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()
}

fn adf_to_text(value: &Value) -> String {
    let mut output = String::new();
    if let Some(obj) = value.as_object() {
        if let Some(content) = obj.get("content").and_then(|v| v.as_array()) {
            for node in content {
                output.push_str(&adf_to_text(node));
            }
        }
        if let Some(text) = obj.get("text").and_then(|v| v.as_str()) {
            output.push_str(text);
        }
        if let Some(node_type) = obj.get("type").and_then(|v| v.as_str()) {
            if node_type == "listItem" {
                output.push_str("\n* ");
            } else if node_type == "heading" {
                output.push('\n');
            }
        }
    } else if let Some(array) = value.as_array() {
        for node in array {
            output.push_str(&adf_to_text(node));
        }
    }

    output
}

