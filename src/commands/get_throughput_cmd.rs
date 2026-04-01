use crate::commands::base_commands::GetThroughputArgs;
use crate::commands::{CommandError, CommandResult};
use crate::services::data_converter::DataConverter;
use crate::services::data_source::DataQuery;
use crate::services::jira_api::{AuthData, JiraApiClient, JiraConfigParser};
use crate::services::parsing::throughput_yaml::serialize_throughput_to_yaml;

pub fn get_throughput_command(args: GetThroughputArgs) -> CommandResult {
    let GetThroughputArgs { config, output } = args;
    let config_parser = JiraConfigParser;
    let jira_project = config_parser
        .parse(&config)
        .map_err(CommandError::ParseJiraConfig)?;

    let auth = AuthData::from_env().map_err(CommandError::LoadJiraAuth)?;
    let api_client = JiraApiClient::new(jira_project.clone(), auth)
        .map_err(CommandError::CreateJiraApiClient)?;

    let data_converter = DataConverter::new(Box::new(api_client));
    let throughput = data_converter
        .get_throughput_data(DataQuery::StringQuery(jira_project.throughput_query))
        .map_err(CommandError::GetThroughputData)?;

    let mut buffer = Vec::new();
    serialize_throughput_to_yaml(&mut buffer, &throughput)
        .map_err(CommandError::SerializeThroughput)?;
    std::fs::write(&output, buffer).map_err(CommandError::WriteOutput)?;

    Ok(vec![format!("Throughput data written to {output}")])
}
