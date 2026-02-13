use chrono::NaiveDate;

use crate::domain::estimate::Estimate;

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
    pub dependencies: Vec<IssueId>,
    pub status: Option<IssueStatus>,
    pub created_date: Option<NaiveDate>,
    pub start_date: Option<NaiveDate>,
    pub done_date: Option<NaiveDate>,
}

impl Issue {
    pub fn new() -> Self {
        Self::default()
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
        assert!(issue.dependencies.is_empty());
        assert_eq!(issue.status, None);
        assert_eq!(issue.created_date, None);
        assert_eq!(issue.start_date, None);
        assert_eq!(issue.done_date, None);
    }
}
