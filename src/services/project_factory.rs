use crate::domain::issue::Issue;
use crate::domain::issue_status::IssueStatus;
use crate::domain::project::Project;
use crate::services::data_source::{DataQuery, DataSource, DataSourceError};

pub struct ProjectFactory<'a> {
    data_source: &'a dyn DataSource,
}

impl<'a> ProjectFactory<'a> {
    pub fn new(data_source: &'a dyn DataSource) -> Self {
        Self { data_source }
    }

    pub fn create_project(
        &self,
        project_name: String,
        query: DataQuery,
    ) -> Result<Project, DataSourceError> {
        let mut issues = self.data_source.get_issues(query)?;
        sort_issues_by_status(&mut issues);
        clear_dependencies_for_boundary_issues(&mut issues);

        Ok(Project {
            name: project_name,
            work_packages: issues,
        })
    }
}

fn sort_issues_by_status(issues: &mut [Issue]) {
    issues.sort_by_key(|issue| status_rank(issue.status.as_ref()));
}

fn status_rank(status: Option<&IssueStatus>) -> u8 {
    match status {
        Some(IssueStatus::Done) => 0,
        Some(IssueStatus::InProgress) => 1,
        Some(IssueStatus::ToDo) => 2,
        None => 3,
    }
}

fn clear_dependencies_for_boundary_issues(issues: &mut [Issue]) {
    if let Some(first_issue) = issues.first_mut() {
        first_issue.dependencies = None;
    }

    if let Some(first_not_done) = issues
        .iter_mut()
        .find(|issue| issue.status.as_ref() != Some(&IssueStatus::Done))
    {
        first_not_done.dependencies = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::issue::IssueId;

    enum MockMode {
        Success(Vec<Issue>),
        ConnectionError,
    }

    struct MockDataSource {
        mode: MockMode,
    }

    impl DataSource for MockDataSource {
        fn get_issues(&self, _query: DataQuery) -> Result<Vec<Issue>, DataSourceError> {
            match &self.mode {
                MockMode::Success(issues) => Ok(issues.clone()),
                MockMode::ConnectionError => Err(DataSourceError::Connection),
            }
        }
    }

    #[test]
    fn create_project_sorts_statuses_and_clears_boundary_dependencies() {
        let source = MockDataSource {
            mode: MockMode::Success(vec![
                issue_with_status("TODO-1", Some(IssueStatus::ToDo), true),
                issue_with_status("DONE-1", Some(IssueStatus::Done), true),
                issue_with_status("IP-1", Some(IssueStatus::InProgress), true),
                issue_with_status("NONE-1", None, true),
            ]),
        };
        let factory = ProjectFactory::new(&source);

        let project = factory
            .create_project(
                "Project Alpha".to_string(),
                DataQuery::StringQuery("project = ALPHA".to_string()),
            )
            .unwrap();

        let ids: Vec<&str> = project
            .work_packages
            .iter()
            .map(|issue| issue.issue_id.as_ref().unwrap().id.as_str())
            .collect();
        assert_eq!(ids, vec!["DONE-1", "IP-1", "TODO-1", "NONE-1"]);
        assert_eq!(project.work_packages[0].dependencies, None);
        assert_eq!(project.work_packages[1].dependencies, None);
        assert!(project.work_packages[2].dependencies.is_some());
    }

    #[test]
    fn create_project_clears_only_first_when_all_issues_are_done() {
        let source = MockDataSource {
            mode: MockMode::Success(vec![
                issue_with_status("DONE-1", Some(IssueStatus::Done), true),
                issue_with_status("DONE-2", Some(IssueStatus::Done), true),
            ]),
        };
        let factory = ProjectFactory::new(&source);

        let project = factory
            .create_project(
                "Project Done".to_string(),
                DataQuery::StringQuery("project = DONE".to_string()),
            )
            .unwrap();

        assert_eq!(project.work_packages[0].dependencies, None);
        assert!(project.work_packages[1].dependencies.is_some());
    }

    #[test]
    fn create_project_when_first_issue_is_not_done_clears_it_once() {
        let source = MockDataSource {
            mode: MockMode::Success(vec![
                issue_with_status("IP-1", Some(IssueStatus::InProgress), true),
                issue_with_status("TODO-1", Some(IssueStatus::ToDo), true),
            ]),
        };
        let factory = ProjectFactory::new(&source);

        let project = factory
            .create_project(
                "Project Active".to_string(),
                DataQuery::StringQuery("project = ACTIVE".to_string()),
            )
            .unwrap();

        assert_eq!(project.work_packages[0].dependencies, None);
        assert!(project.work_packages[1].dependencies.is_some());
    }

    #[test]
    fn create_project_propagates_data_source_errors() {
        let source = MockDataSource {
            mode: MockMode::ConnectionError,
        };
        let factory = ProjectFactory::new(&source);

        let result = factory.create_project(
            "Project Error".to_string(),
            DataQuery::StringQuery("project = ERROR".to_string()),
        );

        assert!(matches!(result, Err(DataSourceError::Connection)));
    }

    fn issue_with_status(id: &str, status: Option<IssueStatus>, with_dependencies: bool) -> Issue {
        let mut issue = Issue::new();
        issue.issue_id = Some(IssueId { id: id.to_string() });
        issue.status = status;
        issue.dependencies = if with_dependencies {
            Some(vec![IssueId {
                id: format!("DEP-{id}"),
            }])
        } else {
            None
        };
        issue
    }
}
