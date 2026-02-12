mod commands;
mod domain;
mod services;

use crate::commands::base_commands::{CliArgs, Commands};
use crate::services::data_converter::DataConverter;
use crate::services::data_source::DataQuery;
use crate::services::jira_api::{AuthData, JiraApiClient, JiraConfigParser};
use crate::services::simulation::simulate_from_throughput_file;
use crate::services::throughput_yaml::serialize_throughput_to_yaml;
use clap::Parser;

#[tokio::main]
async fn main() {
    let args = CliArgs::parse();
    match args.command {
        Commands::GetThroughput { config, output } => {
            // Load Jira config
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
                .await
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
            if let Err(e) = tokio::fs::write(&output, buffer).await {
                eprintln!("Failed to write output file: {e:?}");
            } else {
                println!("Throughput data written to {output}");
            }
        }
        Commands::SimulateN {
            throughput,
            output,
            iterations,
            number_of_issues,
            start_date,
        } => {
            let histogram_path = format!("{output}.png");
            let simulation = match simulate_from_throughput_file(
                &throughput,
                iterations,
                number_of_issues,
                &start_date,
                &histogram_path,
            )
            .await
            {
                Ok(result) => result,
                Err(e) => {
                    eprintln!("Failed to simulate by throughput: {e:?}");
                    return;
                }
            };

            let yaml = match serde_yaml::to_string(&simulation) {
                Ok(contents) => contents,
                Err(e) => {
                    eprintln!("Failed to serialize simulation output: {e:?}");
                    return;
                }
            };

            if let Err(e) = tokio::fs::write(&output, yaml).await {
                eprintln!("Failed to write simulation output: {e:?}");
            } else {
                println!("Simulation result for {number_of_issues} items written to {output}");
                println!("Simulation histogram written to {histogram_path}");
            }
        }
    }
}
