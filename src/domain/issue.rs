use std::collections::HashMap;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Issue {
	pub issue_id: Option<String>,
	pub title: Option<String>,
	pub description: Option<String>,
	pub status: Option<String>,
	pub created_date: Option<String>,
	pub start_date: Option<String>,
	pub done_date: Option<String>,
}

impl Issue {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn to_dict(&self) -> HashMap<String, Option<String>> {
		let mut map = HashMap::with_capacity(7);
		map.insert("issue_id".to_string(), self.issue_id.clone());
		map.insert("title".to_string(), self.title.clone());
		map.insert("description".to_string(), self.description.clone());
		map.insert("status".to_string(), self.status.clone());
		map.insert("created_date".to_string(), self.created_date.clone());
		map.insert("start_date".to_string(), self.start_date.clone());
		map.insert("done_date".to_string(), self.done_date.clone());
		map
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn default_issue_has_none_fields() {
		let issue = Issue::new();
		assert_eq!(issue.issue_id, None);
		assert_eq!(issue.title, None);
		assert_eq!(issue.description, None);
		assert_eq!(issue.status, None);
		assert_eq!(issue.created_date, None);
		assert_eq!(issue.start_date, None);
		assert_eq!(issue.done_date, None);
	}

	#[test]
	fn to_dict_contains_all_keys_and_values() {
		let issue = Issue {
			issue_id: Some("ID-123".into()),
			title: Some("A title".into()),
			description: None,
			status: Some("Done".into()),
			created_date: Some("2025-01-01".into()),
			start_date: None,
			done_date: Some("2025-02-01".into()),
		};

		let dict = issue.to_dict();
		assert_eq!(dict.get("issue_id").cloned().flatten(), Some("ID-123".into()));
		assert_eq!(dict.get("title").cloned().flatten(), Some("A title".into()));
		assert_eq!(dict.get("description").cloned().flatten(), None);
		assert_eq!(dict.get("status").cloned().flatten(), Some("Done".into()));
		assert_eq!(dict.get("created_date").cloned().flatten(), Some("2025-01-01".into()));
		assert_eq!(dict.get("start_date").cloned().flatten(), None);
		assert_eq!(dict.get("done_date").cloned().flatten(), Some("2025-02-01".into()));
	}
}

