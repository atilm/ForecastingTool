mod commands;
mod domain;
mod services;

use crate::commands::base_commands::{CliArgs, Commands};
use crate::commands::get_throughput_cmd::get_throughput_command;
use crate::commands::get_project_cmd::get_project_command;
use crate::commands::plot_throughput_cmd::plot_throughput_command;
use crate::commands::plot_project_cmd::plot_project_command;
use crate::commands::simulate_n_cmd::simulate_n_command;
use crate::commands::simulate_cmd::simulate_command;
use clap::Parser;

#[tokio::main]
async fn main() {
    let args = CliArgs::parse();
    match args.command {
        cmd @ Commands::GetThroughput { .. } => {
            get_throughput_command(cmd).await;
        }
        cmd @ Commands::PlotThroughput { .. } => {
            plot_throughput_command(cmd).await;
        }
        cmd @ Commands::PlotProject { .. } => {
            plot_project_command(cmd).await;
        }
        cmd @ Commands::GetProject { .. } => {
            get_project_command(cmd).await;
        }
        cmd @ Commands::SimulateN { .. } => {
            simulate_n_command(cmd).await;
        }
        cmd @ Commands::Simulate { .. } => {
            simulate_command(cmd).await;
        }
    }
}
