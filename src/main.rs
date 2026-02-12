mod commands;
mod domain;
mod services;

use crate::commands::base_commands::{CliArgs, Commands};
use crate::commands::get_throughput_cmd::get_throughput_command;
use crate::services::simulation::simulate_from_throughput_file;
use clap::Parser;

#[tokio::main]
async fn main() {
    let args = CliArgs::parse();
    match args.command {
        cmd @ Commands::GetThroughput {..} => {
            get_throughput_command(cmd).await;
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
