use chrono::NaiveDate;

use crate::domain::estimate::Estimate;
use crate::domain::estimate::StoryPointEstimate;
use crate::domain::issue_status::IssueStatus;

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
            Estimate::Milestone => None,
            Estimate::ThreePoint(_) => None,
            Estimate::Reference(_) => None,
        }
    }

    pub fn is_milestone(&self) -> bool {
        matches!(self.estimate, Some(Estimate::Milestone))
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

    #[test]
    fn is_milestone_returns_true_for_milestone_estimate() {
        let milestone_issue = Issue {
            estimate: Some(Estimate::Milestone),
            ..Issue::new()
        };

        let three_point_issue = Issue {
            estimate: Some(Estimate::ThreePoint(crate::domain::estimate::ThreePointEstimate {
                optimistic: Some(1.0),
                most_likely: Some(2.0),
                pessimistic: Some(3.0),
            })),
            ..Issue::new()
        };

        let story_point_issue = Issue {
            estimate: Some(Estimate::StoryPoint(crate::domain::estimate::StoryPointEstimate {
                estimate: Some(5.0),
            })),
            ..Issue::new()
        };

        assert!(milestone_issue.is_milestone());
        assert!(!three_point_issue.is_milestone());
        assert!(!story_point_issue.is_milestone());
    }
}
