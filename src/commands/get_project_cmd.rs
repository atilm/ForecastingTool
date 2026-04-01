use crate::commands::base_commands::GetProjectArgs;
use crate::commands::{CommandError, CommandResult};
use crate::services::data_source::DataQuery;
use crate::services::jira_api::{AuthData, JiraApiClient, JiraConfigParser};
use crate::services::parsing::project_yaml::serialize_project_to_yaml;
use crate::services::project_factory::ProjectFactory;

pub fn get_project_command(args: GetProjectArgs) -> CommandResult {
    let GetProjectArgs { config, output } = args;
    let config_parser = JiraConfigParser;
    let jira_project = config_parser
        .parse(&config)
        .map_err(CommandError::ParseJiraConfig)?;

    let auth = AuthData::from_env().map_err(CommandError::LoadJiraAuth)?;

    let api_client = JiraApiClient::new(jira_project.clone(), auth)
        .map_err(CommandError::CreateJiraApiClient)?;

    let project_factory = ProjectFactory::new(&api_client);
    let project = project_factory
        .create_project(
            jira_project.project_key.clone(),
            DataQuery::StringQuery(jira_project.project_query),
        )
        .map_err(CommandError::GetProjectData)?;

    let mut buffer = Vec::new();
    serialize_project_to_yaml(&mut buffer, &project).map_err(CommandError::SerializeProject)?;

    std::fs::write(&output, buffer).map_err(CommandError::WriteOutput)?;

    Ok(vec![format!("Project data written to {output}")])
}
