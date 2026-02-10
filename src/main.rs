use tokio;

mod domain;
mod services;

use clap::{Parser, Subcommand};
use std::fs::File;
use std::io::BufWriter;
use crate::services::jira_api::{JiraApiClient, JiraConfigParser, AuthData};
use crate::services::data_converter::DataConverter;
use crate::services::throughput_yaml::serialize_throughput_to_yaml;
use crate::services::data_source::DataQuery;


#[derive(Parser)]
#[command(author, version, about)]
struct CliArgs {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Print hello message
    Hello {
        #[arg(short, long)]
        name: String,
    },
    /// Get throughput data from Jira and serialize to YAML
    GetThroughput {
        /// Path to Jira config YAML
        #[arg(short, long)]
        config: String,
        /// Output YAML file
        #[arg(short, long)]
        output: String,
    },
}


#[tokio::main]
async fn main() {
    let args = CliArgs::parse();
    match args.command {
        Commands::Hello { name } => {
            println!("Hello, {}!", name);
        }
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
            let throughput = match data_converter.get_throughput_data(DataQuery::StringQuery(jira_project.throughput_query)).await {
                Ok(data) => data,
                Err(e) => {
                    eprintln!("Failed to get throughput data: {e:?}");
                    return;
                }
            };
            // Serialize to YAML
            let file = match File::create(&output) {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("Failed to create output file: {e:?}");
                    return;
                }
            };
            let mut writer = BufWriter::new(file);
            if let Err(e) = serialize_throughput_to_yaml(&mut writer, &throughput) {
                eprintln!("Failed to serialize throughput to YAML: {e:?}");
            } else {
                println!("Throughput data written to {output}");
            }
        }
    }
}

