use chrono::NaiveDate;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IssueStatus {
    ToDo,
    InProgress,
    Done,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Issue {
    pub issue_id: Option<String>,
    pub summary: Option<String>,
    pub description: Option<String>,
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
        assert_eq!(issue.status, None);
        assert_eq!(issue.created_date, None);
        assert_eq!(issue.start_date, None);
        assert_eq!(issue.done_date, None);
    }
}
