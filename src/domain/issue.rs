use chrono::NaiveDate;

use thiserror::Error;
use crate::domain::estimate::Estimate;
use crate::domain::estimate::StoryPointEstimate;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IssueStatus {
    ToDo,
    InProgress,
    Done,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssueId {
    pub id: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Issue {
    pub issue_id: Option<IssueId>,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub estimate: Option<Estimate>,
    pub dependencies: Option<Vec<IssueId>>,
    pub subgraph: Option<String>,
    pub status: Option<IssueStatus>,
    pub created_date: Option<NaiveDate>,
    pub start_date: Option<NaiveDate>,
    pub done_date: Option<NaiveDate>,
}

#[derive(Error, Debug)]
pub enum IssueError {
    #[error("issue is missing an estimate")]
    NoEstimate,
}

impl Issue {
    pub fn new() -> Self {
        Self {
            dependencies: Some(Vec::new()),
            ..Self::default()
        }
    }

    pub fn story_point_value(&self) -> Option<f32> {
        match self.estimate.as_ref()? {
            Estimate::StoryPoint(StoryPointEstimate { estimate }) => *estimate,
            Estimate::ThreePoint(_) => None,
            Estimate::Reference(_) => None,
        }
    }

    pub fn has_zero_duration(&self) -> Result<bool, IssueError> {
        let estimate = self.estimate.as_ref().ok_or(IssueError::NoEstimate)?;
        let result = match estimate {
            Estimate::StoryPoint(StoryPointEstimate { estimate }) => *estimate == Some(0.0),
            Estimate::ThreePoint(three_point) => {
                three_point.optimistic == Some(0.0)
                    && three_point.most_likely == Some(0.0)
                    && three_point.pessimistic == Some(0.0)
            }
            Estimate::Reference(_) => false,
        };

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_issue_has_none_fields() {
        let issue = Issue::new();
        assert_eq!(issue.issue_id, None);
        assert_eq!(issue.summary, None);
        assert_eq!(issue.description, None);
        assert_eq!(issue.estimate, None);
        assert_eq!(issue.dependencies, Some(Vec::new()));
        assert_eq!(issue.subgraph, None);
        assert_eq!(issue.status, None);
        assert_eq!(issue.created_date, None);
        assert_eq!(issue.start_date, None);
        assert_eq!(issue.done_date, None);
    }
}
