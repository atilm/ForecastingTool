use chrono::NaiveDate;

use crate::domain::issue::IssueId;

use super::issue::{Issue, IssueStatus};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Epic {
	pub issue_id: Option<IssueId>,
	pub summary: Option<String>,
	pub description: Option<String>,
	pub status: Option<IssueStatus>,
	pub issues: Vec<Issue>,
	pub start_date: Option<NaiveDate>,
	pub due_date: Option<NaiveDate>,
}

impl Epic {
	pub fn new() -> Self {
		Self::default()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn default_epic_has_none_fields_and_empty_issues() {
		let epic = Epic::new();
		assert_eq!(epic.issue_id, None);
		assert_eq!(epic.summary, None);
		assert_eq!(epic.description, None);
		assert_eq!(epic.status, None);
		assert!(epic.issues.is_empty());
		assert_eq!(epic.start_date, None);
		assert_eq!(epic.due_date, None);
	}
}
