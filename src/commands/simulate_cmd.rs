use crate::commands::base_commands::SimulateProjectArgs;
use crate::commands::report_format::format_simulation_report;
use crate::commands::{CommandError, CommandResult};
use crate::services::plotting::histogram::write_histogram_png;
use crate::services::plotting::milestone_plot::write_milestone_plot_png;
use crate::services::project_simulation::project_simulation::simulate_project_from_yaml_file;

pub fn simulate_command(args: SimulateProjectArgs) -> CommandResult {
    let SimulateProjectArgs {
        input,
        output,
        iterations,
        start_date,
        calendar_dir,
    } = args;

    let simulation = simulate_project_from_yaml_file(&input, iterations, start_date, calendar_dir.as_deref())
        .map_err(CommandError::SimulateProject)?;

    let histogram_path = format!("{output}.png");
    let mut messages = Vec::new();

    match write_histogram_png(&histogram_path, &simulation.results) {
        Ok(()) => messages.push(format!("Simulation histogram written to {histogram_path}")),
        Err(error) => messages.push(format!("Warning: failed to write simulation histogram: {error}")),
    }

    let milestone_plot_path = format!("{output}.milestones.png");
    match write_milestone_plot_png(&milestone_plot_path, &simulation) {
        Ok(()) => messages.push(format!("Milestone plot written to {milestone_plot_path}")),
        Err(error) => messages.push(format!("Warning: failed to write milestone plot: {error}")),
    }

    let yaml = serde_yaml::to_string(&simulation.report).map_err(CommandError::SerializeSimulation)?;
    std::fs::write(&output, yaml).map_err(CommandError::WriteOutput)?;

    messages.insert(0, format!("Simulation result written to {output}"));
    messages.insert(0, format_simulation_report(&simulation.report));

    Ok(messages)
}
