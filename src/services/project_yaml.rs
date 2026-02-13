use std::io::{self, Write};

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::domain::estimate::{Estimate, StoryPointEstimate, ThreePointEstimate};
use crate::domain::issue::{Issue, IssueId, IssueStatus};
use crate::domain::project::Project;

#[derive(Error, Debug)]
pub enum ProjectYamlError {
    #[error("failed to read project yaml: {0}")]
    Read(#[from] io::Error),
    #[error("failed to parse project yaml: {0}")]
    Parse(#[from] serde_yaml::Error),
    #[error("missing issue id")]
    MissingIssueId,
    #[error("invalid date format: {0}")]
    InvalidDate(String),
    #[error("invalid status value: {0}")]
    InvalidStatus(String),
    #[error("missing previous issue for implicit dependency")]
    MissingPreviousDependency,
}

#[derive(Serialize, Deserialize)]
struct ProjectRecord {
    name: String,
    work_packages: Vec<IssueRecord>,
}

#[derive(Serialize, Deserialize)]
struct IssueRecord {
    id: String,
    summary: Option<String>,
    description: Option<String>,
    estimate: Option<EstimateRecord>,
    status: Option<String>,
    created_date: Option<String>,
    start_date: Option<String>,
    done_date: Option<String>,
    dependencies: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum EstimateRecord {
    StoryPoints { value: f32 },
    ThreePoint {
        optimistic: f32,
        most_likely: f32,
        pessimistic: f32,
    },
}

pub async fn load_project_from_yaml_file(path: &str) -> Result<Project, ProjectYamlError> {
    let contents = tokio::fs::read_to_string(path).await?;
    deserialize_project_from_yaml_str(&contents)
}

pub fn deserialize_project_from_yaml_str(input: &str) -> Result<Project, ProjectYamlError> {
    let record: ProjectRecord = serde_yaml::from_str(input)?;
    let mut work_packages = Vec::with_capacity(record.work_packages.len());
    let mut previous_id: Option<String> = None;

    for issue_record in record.work_packages {
        if issue_record.id.trim().is_empty() {
            return Err(ProjectYamlError::MissingIssueId);
        }

        let mut issue = Issue::new();
        issue.issue_id = Some(IssueId {
            id: issue_record.id,
        });
        issue.summary = issue_record.summary;
        issue.description = issue_record.description;
        issue.estimate = issue_record.estimate.map(estimate_from_record);
        issue.status = parse_status(issue_record.status.as_deref())?;
        issue.created_date = parse_date_opt(issue_record.created_date.as_deref())?;
        issue.start_date = parse_date_opt(issue_record.start_date.as_deref())?;
        issue.done_date = parse_date_opt(issue_record.done_date.as_deref())?;
        issue.dependencies = match issue_record.dependencies {
            None => Vec::new(),
            Some(values) if values.is_empty() => {
                let previous = previous_id
                    .clone()
                    .ok_or(ProjectYamlError::MissingPreviousDependency)?;
                vec![IssueId { id: previous }]
            }
            Some(values) => values.into_iter().map(|id| IssueId { id }).collect(),
        };
        previous_id = issue.issue_id.as_ref().map(|id| id.id.clone());
        work_packages.push(issue);
    }

    Ok(Project {
        name: record.name,
        work_packages,
    })
}

pub fn serialize_project_to_yaml<W: Write>(writer: &mut W, project: &Project) -> io::Result<()> {
    let record = ProjectRecord {
        name: project.name.clone(),
        work_packages: project
            .work_packages
            .iter()
            .map(issue_to_record)
            .collect(),
    };

    let yaml = serde_yaml::to_string(&record)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    writer.write_all(yaml.as_bytes())
}

fn issue_to_record(issue: &Issue) -> IssueRecord {
    IssueRecord {
        id: issue
            .issue_id
            .as_ref()
            .map(|id| id.id.clone())
            .unwrap_or_default(),
        summary: issue.summary.clone(),
        description: issue.description.clone(),
        estimate: estimate_to_record(issue.estimate.as_ref()),
        status: issue.status.as_ref().map(status_to_string),
        created_date: issue
            .created_date
            .map(|date| date.format("%Y-%m-%d").to_string()),
        start_date: issue
            .start_date
            .map(|date| date.format("%Y-%m-%d").to_string()),
        done_date: issue
            .done_date
            .map(|date| date.format("%Y-%m-%d").to_string()),
        dependencies: if issue.dependencies.is_empty() {
            None
        } else {
            Some(issue.dependencies.iter().map(|id| id.id.clone()).collect())
        },
    }
}

fn estimate_from_record(record: EstimateRecord) -> Estimate {
    match record {
        EstimateRecord::StoryPoints { value } => Estimate::StoryPoint(StoryPointEstimate {
            estimate: Some(value),
        }),
        EstimateRecord::ThreePoint {
            optimistic,
            most_likely,
            pessimistic,
        } => Estimate::ThreePoint(ThreePointEstimate {
            optimistic: Some(optimistic),
            most_likely: Some(most_likely),
            pessimistic: Some(pessimistic),
        }),
    }
}

fn estimate_to_record(estimate: Option<&Estimate>) -> Option<EstimateRecord> {
    match estimate? {
        Estimate::StoryPoint(StoryPointEstimate { estimate }) => estimate.map(|value| {
            EstimateRecord::StoryPoints { value }
        }),
        Estimate::ThreePoint(ThreePointEstimate {
            optimistic,
            most_likely,
            pessimistic,
        }) => match (optimistic, most_likely, pessimistic) {
            (Some(optimistic), Some(most_likely), Some(pessimistic)) => {
                Some(EstimateRecord::ThreePoint {
                    optimistic: *optimistic,
                    most_likely: *most_likely,
                    pessimistic: *pessimistic,
                })
            }
            _ => None,
        },
    }
}

fn parse_date_opt(value: Option<&str>) -> Result<Option<NaiveDate>, ProjectYamlError> {
    let text = match value {
        Some(text) => text,
        None => return Ok(None),
    };
    let date = NaiveDate::parse_from_str(text, "%Y-%m-%d")
        .map_err(|_| ProjectYamlError::InvalidDate(text.to_string()))?;
    Ok(Some(date))
}

fn parse_status(value: Option<&str>) -> Result<Option<IssueStatus>, ProjectYamlError> {
    let status = match value {
        Some(text) => text,
        None => return Ok(None),
    };
    let status = match status.to_ascii_lowercase().as_str() {
        "todo" | "to do" => IssueStatus::ToDo,
        "inprogress" | "in progress" => IssueStatus::InProgress,
        "done" => IssueStatus::Done,
        _ => return Err(ProjectYamlError::InvalidStatus(status.to_string())),
    };
    Ok(Some(status))
}

fn status_to_string(status: &IssueStatus) -> String {
    match status {
        IssueStatus::ToDo => "ToDo".to_string(),
        IssueStatus::InProgress => "InProgress".to_string(),
        IssueStatus::Done => "Done".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::estimate::Estimate;
    use crate::domain::issue::IssueId;

    #[test]
    fn serialize_project_to_yaml_includes_estimate_format() {
        let mut issue = Issue::new();
        issue.issue_id = Some(IssueId {
            id: "ABC-1".to_string(),
        });
        issue.summary = Some("Example issue".to_string());
        issue.description = Some("Example description".to_string());
        issue.estimate = Some(Estimate::StoryPoint(StoryPointEstimate {
            estimate: Some(3.0),
        }));
        issue.status = Some(IssueStatus::Done);
        issue.created_date = Some(NaiveDate::from_ymd_opt(2026, 1, 12).unwrap());
        issue.start_date = Some(NaiveDate::from_ymd_opt(2026, 1, 13).unwrap());
        issue.done_date = Some(NaiveDate::from_ymd_opt(2026, 1, 15).unwrap());

        let project = Project {
            name: "TEST".to_string(),
            work_packages: vec![issue],
        };

        let mut buffer = Vec::new();
        serialize_project_to_yaml(&mut buffer, &project).unwrap();
        let output = String::from_utf8(buffer).unwrap();

        assert!(output.contains("name: TEST"));
        assert!(output.contains("id: ABC-1"));
        assert!(output.contains("summary: Example issue"));
        assert!(output.contains("description: Example description"));
        assert!(output.contains("type: story_points"));
        assert!(output.contains("value: 3"));
        assert!(output.contains("status: Done"));
        assert!(output.contains("created_date: 2026-01-12"));
        assert!(output.contains("start_date: 2026-01-13"));
        assert!(output.contains("done_date: 2026-01-15"));
    }

    #[test]
    fn deserialize_project_with_story_points() {
        let yaml = r#"
name: Demo
work_packages:
  - id: ABC-1
    summary: First
    estimate:
      type: story_points
      value: 5
    status: Done
    start_date: 2026-01-02
    done_date: 2026-01-05
    dependencies: [ABC-0]
"#;

        let project = deserialize_project_from_yaml_str(yaml).unwrap();
        let issue = &project.work_packages[0];
        assert_eq!(project.name, "Demo");
        assert_eq!(issue.issue_id.as_ref().unwrap().id, "ABC-1");
        assert!(matches!(issue.status, Some(IssueStatus::Done)));
        assert_eq!(issue.dependencies.len(), 1);
        assert!(matches!(
            issue.estimate,
            Some(Estimate::StoryPoint(StoryPointEstimate { estimate: Some(5.0) }))
        ));
    }

    #[test]
    fn deserialize_project_with_three_point_estimate() {
        let yaml = r#"
name: Demo
work_packages:
  - id: ABC-2
    estimate:
      type: three_point
      optimistic: 2
      most_likely: 3
      pessimistic: 8
"#;

        let project = deserialize_project_from_yaml_str(yaml).unwrap();
        let issue = &project.work_packages[0];
        assert!(matches!(
            issue.estimate,
            Some(Estimate::ThreePoint(ThreePointEstimate {
                optimistic: Some(2.0),
                most_likely: Some(3.0),
                pessimistic: Some(8.0)
            }))
        ));
    }

    #[test]
    fn deserialize_project_rejects_invalid_date() {
        let yaml = r#"
name: Demo
work_packages:
  - id: ABC-3
    start_date: 2026-99-01
"#;

        let error = deserialize_project_from_yaml_str(yaml).unwrap_err();
        assert!(matches!(error, ProjectYamlError::InvalidDate(_)));
    }

    #[test]
    fn deserialize_project_rejects_invalid_status() {
        let yaml = r#"
name: Demo
work_packages:
  - id: ABC-4
    status: Blocked
"#;

        let error = deserialize_project_from_yaml_str(yaml).unwrap_err();
        assert!(matches!(error, ProjectYamlError::InvalidStatus(_)));
    }

    #[test]
    fn deserialize_project_rejects_missing_id() {
        let yaml = r#"
name: Demo
work_packages:
  - id: ""
"#;

        let error = deserialize_project_from_yaml_str(yaml).unwrap_err();
        assert!(matches!(error, ProjectYamlError::MissingIssueId));
    }

    #[test]
    fn deserialize_project_uses_previous_issue_for_empty_dependencies() {
        let yaml = r#"
name: Demo
work_packages:
  - id: ABC-1
    dependencies: null
  - id: ABC-2
    dependencies: []
"#;

        let project = deserialize_project_from_yaml_str(yaml).unwrap();
        let issue = &project.work_packages[1];
        assert_eq!(issue.dependencies.len(), 1);
        assert_eq!(issue.dependencies[0].id, "ABC-1");
    }

    #[test]
    fn deserialize_project_rejects_empty_dependencies_for_first_issue() {
        let yaml = r#"
name: Demo
work_packages:
  - id: ABC-1
    dependencies: []
"#;

        let error = deserialize_project_from_yaml_str(yaml).unwrap_err();
        assert!(matches!(error, ProjectYamlError::MissingPreviousDependency));
    }
}
