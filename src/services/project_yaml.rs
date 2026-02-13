use std::io::{self, Write};

use crate::domain::estimate::{Estimate, StoryPointEstimate};
use crate::domain::issue::{Issue, IssueStatus};
use crate::domain::project::Project;
use serde::Serialize;

#[derive(Serialize)]
struct ProjectRecord {
    name: String,
    work_packages: Vec<IssueRecord>,
}

#[derive(Serialize)]
struct IssueRecord {
    id: Option<String>,
    summary: Option<String>,
    description: Option<String>,
    estimate: Option<f32>,
    status: Option<String>,
    created_date: Option<String>,
    start_date: Option<String>,
    done_date: Option<String>,
    dependencies: Vec<String>,
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
        id: issue.issue_id.as_ref().map(|id| id.id.clone()),
        summary: issue.summary.clone(),
        description: issue.description.clone(),
        estimate: estimate_to_f32(issue.estimate.as_ref()),
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
        dependencies: issue
            .dependencies
            .iter()
            .map(|id| id.id.clone())
            .collect(),
    }
}

fn estimate_to_f32(estimate: Option<&Estimate>) -> Option<f32> {
    match estimate? {
        Estimate::StoryPoint(StoryPointEstimate { estimate }) => *estimate,
        Estimate::ThreePoint(_) => None,
    }
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
    use crate::domain::issue::{Issue, IssueId};
    use chrono::NaiveDate;

    #[test]
    fn serialize_project_to_yaml_includes_issue_fields() {
        let mut issue = Issue::new();
        issue.issue_id = Some(IssueId {
            id: "ABC-1".to_string(),
        });
        issue.summary = Some("Example issue".to_string());
        issue.description = Some("Example description".to_string());
        issue.estimate = Some(Estimate::StoryPoint(StoryPointEstimate { estimate: Some(3.0) }));
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
        assert!(output.contains("estimate: 3"));
        assert!(output.contains("status: Done"));
        assert!(output.contains("created_date: 2026-01-12"));
        assert!(output.contains("start_date: 2026-01-13"));
        assert!(output.contains("done_date: 2026-01-15"));
    }
}
