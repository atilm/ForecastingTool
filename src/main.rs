use clap::{CommandFactory, Parser};
use clap_complete;
use forecasts::commands::base_commands::{CliArgs, Commands};
use forecasts::commands::get_project_cmd::get_project_command;
use forecasts::commands::get_throughput_cmd::get_throughput_command;
use forecasts::commands::plot_burndown_cmd::plot_burndown_command;
use forecasts::commands::plot_gantt_cmd::plot_gantt_command;
use forecasts::commands::plot_project_cmd::plot_project_command;
use forecasts::commands::plot_simulation_gantt_cmd::plot_simulation_gantt_command;
use forecasts::commands::plot_throughput_cmd::plot_throughput_command;
use forecasts::commands::simulate_cmd::simulate_command;
use forecasts::commands::simulate_n_cmd::simulate_n_command;
use std::io;

fn main() {
    let args = CliArgs::parse();
    match args.command {
        cmd @ Commands::GetThroughput { .. } => {
            get_throughput_command(cmd);
        }
        cmd @ Commands::PlotThroughput { .. } => {
            plot_throughput_command(cmd);
        }
        cmd @ Commands::PlotProject { .. } => {
            plot_project_command(cmd);
        }
        cmd @ Commands::PlotGantt { .. } => {
            plot_gantt_command(cmd);
        }
        cmd @ Commands::PlotSimulationGantt { .. } => {
            plot_simulation_gantt_command(cmd);
        }
        cmd @ Commands::PlotBurndown { .. } => {
            plot_burndown_command(cmd);
        }
        cmd @ Commands::GetProject { .. } => {
            get_project_command(cmd);
        }
        cmd @ Commands::SimulateN { .. } => {
            simulate_n_command(cmd);
        }
        cmd @ Commands::Simulate { .. } => {
            simulate_command(cmd);
        }
        Commands::GitHash { .. } => {
            println!("Git Hash: {}", env!("GIT_HASH"));
        }
        Commands::Completions { shell } => {
            let mut cmd = CliArgs::command();
            clap_complete::generate(shell, &mut cmd, env!("CARGO_PKG_NAME"), &mut io::stdout());
        }
    }
}
