use clap::{CommandFactory, Parser};
use clap_complete;
use forecasts::commands::base_commands::{
    CliArgs, Commands, GetCommands, PlotCommands, SimulateCommands, UtilCommands,
};
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
        Commands::Get { command } => match command {
            GetCommands::Throughput(args) => get_throughput_command(args),
            GetCommands::Project(args) => get_project_command(args),
        },
        Commands::Plot { command } => match command {
            PlotCommands::Throughput(args) => plot_throughput_command(args),
            PlotCommands::Project(args) => plot_project_command(args),
            PlotCommands::Gantt(args) => plot_gantt_command(args),
            PlotCommands::SimulationGantt(args) => plot_simulation_gantt_command(args),
            PlotCommands::Burndown(args) => plot_burndown_command(args),
        },
        Commands::Simulate { command } => match command {
            SimulateCommands::Project(args) => simulate_command(args),
            SimulateCommands::Throughput(args) => simulate_n_command(args),
        },
        Commands::Util { command } => match command {
            UtilCommands::GitHash => {
                println!("Git Hash: {}", env!("GIT_HASH"));
            }
            UtilCommands::Completions(args) => {
                let mut cmd = CliArgs::command();
                clap_complete::generate(
                    args.shell,
                    &mut cmd,
                    env!("CARGO_PKG_NAME"),
                    &mut io::stdout(),
                );
            }
        },
    }
}
