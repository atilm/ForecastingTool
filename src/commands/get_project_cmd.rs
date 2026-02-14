use crate::commands::base_commands::Commands;
use crate::services::data_source::{DataQuery, DataSource};
use crate::services::jira_api::{AuthData, JiraApiClient, JiraConfigParser};
use crate::services::project_yaml::serialize_project_to_yaml;

pub fn get_project_command(cmd: Commands) {
    if let Commands::GetProject { config, output } = cmd {
        let config_parser = JiraConfigParser;
        let jira_project = match config_parser.parse(&config) {
            Ok(cfg) => cfg,
            Err(e) => {
                eprintln!("Failed to parse Jira config: {e:?}");
                return;
            }
        };

        let auth = match AuthData::from_env() {
            Ok(auth) => auth,
            Err(e) => {
                eprintln!("Failed to load Jira auth: {e:?}");
                return;
            }
        };

        let api_client = match JiraApiClient::new(jira_project.clone(), auth) {
            Ok(client) => client,
            Err(e) => {
                eprintln!("Failed to create JiraApiClient: {e:?}");
                return;
            }
        };

        let project = match api_client
            .get_project(DataQuery::StringQuery(jira_project.project_query))
        {
            Ok(project) => project,
            Err(e) => {
                eprintln!("Failed to get project data: {e:?}");
                return;
            }
        };

        let mut buffer = Vec::new();
        if let Err(e) = serialize_project_to_yaml(&mut buffer, &project) {
            eprintln!("Failed to serialize project to YAML: {e:?}");
            return;
        }

        if let Err(e) = std::fs::write(&output, buffer) {
            eprintln!("Failed to write output file: {e:?}");
        } else {
            println!("Project data written to {output}");
        }
    }
}