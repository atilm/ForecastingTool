use crate::commands::base_commands::Commands;
use crate::services::data_converter::DataConverter;
use crate::services::data_source::DataQuery;
use crate::services::jira_api::{AuthData, JiraApiClient, JiraConfigParser};
use crate::services::throughput_yaml::serialize_throughput_to_yaml;

pub fn get_throughput_command(cmd: Commands) {
    println!("This is the get_throughput command");
    if let Commands::GetThroughput { config, output } = cmd {
        let config_parser = JiraConfigParser;
        let jira_project = match config_parser.parse(&config) {
            Ok(cfg) => cfg,
            Err(e) => {
                eprintln!("Failed to parse Jira config: {e:?}");
                return;
            }
        };

        // Load auth from env
        let auth = match AuthData::from_env() {
            Ok(auth) => auth,
            Err(e) => {
                eprintln!("Failed to load Jira auth: {e:?}");
                return;
            }
        };
        // Create JiraApiClient
        let api_client = match JiraApiClient::new(jira_project.clone(), auth) {
            Ok(client) => client,
            Err(e) => {
                eprintln!("Failed to create JiraApiClient: {e:?}");
                return;
            }
        };
        let data_converter = DataConverter::new(Box::new(api_client));
        // Fetch throughput data
        let throughput = match data_converter
            .get_throughput_data(DataQuery::StringQuery(jira_project.throughput_query))
        {
            Ok(data) => data,
            Err(e) => {
                eprintln!("Failed to get throughput data: {e:?}");
                return;
            }
        };
        // Serialize to YAML
        let mut buffer = Vec::new();
        if let Err(e) = serialize_throughput_to_yaml(&mut buffer, &throughput) {
            eprintln!("Failed to serialize throughput to YAML: {e:?}");
            return;
        }
        if let Err(e) = std::fs::write(&output, buffer) {
            eprintln!("Failed to write output file: {e:?}");
        } else {
            println!("Throughput data written to {output}");
        }
    }
}
