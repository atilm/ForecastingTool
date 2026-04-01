use std::collections::HashSet;
use std::fmt;
use std::ops::Deref;

use crate::domain::issue_status::IssueStatus;
use crate::domain::project::Project;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DateType {
    StartDate,
    EndDate,
}

impl fmt::Display for DateType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DateType::StartDate => write!(f, "start_date"),
            DateType::EndDate => write!(f, "done_date"),
        }
    }
}

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ProjectValidationError {
    #[error("Duplicate issue id: {0}")]
    DuplicateIssueId(String),
    #[error("Non-existing dependency: {0}")]
    NonExistingDependency(String),
    #[error("Issue {id} has status {status:?} but no {date_type} is set.")]
    InvalidIssueStatus {
        id: String,
        status: IssueStatus,
        date_type: DateType,
    },
    #[error("Issue {id} has {date_type} set but status is {status:?}.")]
    UnexpectedIssueDate {
        id: String,
        status: IssueStatus,
        date_type: DateType,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationErrors(pub Vec<ProjectValidationError>);

impl fmt::Display for ValidationErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "project has {} validation error(s):", self.0.len())?;
        for (index, error) in self.0.iter().enumerate() {
            write!(f, "  - {error}")?;
            if index + 1 < self.0.len() {
                writeln!(f)?;
            }
        }
        Ok(())
    }
}

impl std::error::Error for ValidationErrors {}

impl Deref for ValidationErrors {
    type Target = [ProjectValidationError];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub fn validate_project(project: &Project) -> Result<(), ValidationErrors> {
    let mut errors = Vec::new();
    let mut seen_ids: HashSet<&str> = HashSet::new();

    for issue in &project.work_packages {
        let Some(issue_id) = issue.issue_id.as_ref() else {
            continue;
        };

        if !seen_ids.insert(&issue_id.id) {
            errors.push(ProjectValidationError::DuplicateIssueId(issue_id.id.clone()));
        }
    }

    for issue in &project.work_packages {
        let id = issue
            .issue_id
            .as_ref()
            .map(|value| value.id.clone())
            .unwrap_or_default();

        if let Some(dependencies) = issue.dependencies.as_ref() {
            for dependency in dependencies {
                if !seen_ids.contains(dependency.id.as_str()) {
                    errors.push(ProjectValidationError::NonExistingDependency(format!(
                        "{} -> {}",
                        id, dependency.id
                    )));
                }
            }
        }

        if let Some(status) = issue.status.clone() {
            validate_status_dates(&id, &status, issue.start_date.is_some(), issue.done_date.is_some(), &mut errors);
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(ValidationErrors(errors))
    }
}

fn validate_status_dates(
    id: &str,
    status: &IssueStatus,
    has_start_date: bool,
    has_done_date: bool,
    errors: &mut Vec<ProjectValidationError>,
) {
    match status {
        IssueStatus::ToDo => {
            if has_start_date {
                errors.push(ProjectValidationError::UnexpectedIssueDate {
                    id: id.to_string(),
                    status: IssueStatus::ToDo,
                    date_type: DateType::StartDate,
                });
            }
            if has_done_date {
                errors.push(ProjectValidationError::UnexpectedIssueDate {
                    id: id.to_string(),
                    status: IssueStatus::ToDo,
                    date_type: DateType::EndDate,
                });
            }
        }
        IssueStatus::InProgress => {
            if !has_start_date {
                errors.push(ProjectValidationError::InvalidIssueStatus {
                    id: id.to_string(),
                    status: IssueStatus::InProgress,
                    date_type: DateType::StartDate,
                });
            }
            if has_done_date {
                errors.push(ProjectValidationError::UnexpectedIssueDate {
                    id: id.to_string(),
                    status: IssueStatus::InProgress,
                    date_type: DateType::EndDate,
                });
            }
        }
        IssueStatus::Done => {
            if !has_start_date {
                errors.push(ProjectValidationError::InvalidIssueStatus {
                    id: id.to_string(),
                    status: IssueStatus::Done,
                    date_type: DateType::StartDate,
                });
            }
            if !has_done_date {
                errors.push(ProjectValidationError::InvalidIssueStatus {
                    id: id.to_string(),
                    status: IssueStatus::Done,
                    date_type: DateType::EndDate,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;
    use crate::domain::issue::{Issue, IssueId};

    fn make_issue(id: &str) -> Issue {
        let mut issue = Issue::new();
        issue.issue_id = Some(IssueId { id: id.to_string() });
        issue
    }

    #[test]
    fn validate_project_accepts_valid_project() {
        let mut issue = make_issue("ABC-1");
        issue.status = Some(IssueStatus::Done);
        issue.start_date = NaiveDate::from_ymd_opt(2026, 1, 1);
        issue.done_date = NaiveDate::from_ymd_opt(2026, 1, 2);

        let project = Project {
            name: "Demo".to_string(),
            work_packages: vec![issue],
        };

        assert!(validate_project(&project).is_ok());
    }

    #[test]
    fn validate_project_reports_duplicate_issue_ids() {
        let issue1 = make_issue("ABC-1");
        let issue2 = make_issue("ABC-1");
        let project = Project {
            name: "Demo".to_string(),
            work_packages: vec![issue1, issue2],
        };

        let errors = validate_project(&project).unwrap_err();
        assert!(errors
            .iter()
            .any(|error| matches!(error, ProjectValidationError::DuplicateIssueId(id) if id == "ABC-1")));
    }

    #[test]
    fn validate_project_reports_non_existing_dependency() {
        let mut issue = make_issue("ABC-2");
        issue.dependencies = Some(vec![IssueId {
            id: "ABC-404".to_string(),
        }]);

        let project = Project {
            name: "Demo".to_string(),
            work_packages: vec![issue],
        };

        let errors = validate_project(&project).unwrap_err();
        assert!(errors.iter().any(|error| {
            matches!(error, ProjectValidationError::NonExistingDependency(link) if link == "ABC-2 -> ABC-404")
        }));
    }

    #[test]
    fn validate_project_reports_status_date_violations() {
        let mut todo = make_issue("TODO-1");
        todo.status = Some(IssueStatus::ToDo);
        todo.start_date = NaiveDate::from_ymd_opt(2026, 1, 1);

        let mut in_progress = make_issue("IP-1");
        in_progress.status = Some(IssueStatus::InProgress);

        let mut done = make_issue("DONE-1");
        done.status = Some(IssueStatus::Done);
        done.start_date = NaiveDate::from_ymd_opt(2026, 1, 1);

        let project = Project {
            name: "Demo".to_string(),
            work_packages: vec![todo, in_progress, done],
        };

        let errors = validate_project(&project).unwrap_err();

        assert!(errors.iter().any(|error| {
            matches!(
                error,
                ProjectValidationError::UnexpectedIssueDate {
                    id,
                    status: IssueStatus::ToDo,
                    date_type: DateType::StartDate
                } if id == "TODO-1"
            )
        }));
        assert!(errors.iter().any(|error| {
            matches!(
                error,
                ProjectValidationError::InvalidIssueStatus {
                    id,
                    status: IssueStatus::InProgress,
                    date_type: DateType::StartDate
                } if id == "IP-1"
            )
        }));
        assert!(errors.iter().any(|error| {
            matches!(
                error,
                ProjectValidationError::InvalidIssueStatus {
                    id,
                    status: IssueStatus::Done,
                    date_type: DateType::EndDate
                } if id == "DONE-1"
            )
        }));
    }
}