use crate::commands::base_commands::SimulateThroughputArgs;
use crate::commands::report_format::format_simulation_report;
use crate::commands::{CommandError, CommandResult};
use crate::services::project_simulation::throughput_simulation::simulate_from_throughput_file;

pub fn simulate_n_command(args: SimulateThroughputArgs) -> CommandResult {
    let SimulateThroughputArgs {
        throughput,
        output,
        iterations,
        number_of_issues,
        start_date,
        calendar_dir,
    } = args;

    let histogram_path = format!("{output}.png");
    let simulation = simulate_from_throughput_file(
        &throughput,
        iterations,
        number_of_issues,
        start_date,
        &histogram_path,
        calendar_dir.as_deref(),
    )
    .map_err(CommandError::SimulateThroughput)?;

    let yaml = serde_yaml::to_string(&simulation).map_err(CommandError::SerializeSimulation)?;
    std::fs::write(&output, yaml).map_err(CommandError::WriteOutput)?;

    Ok(vec![
        format_simulation_report(&simulation),
        format!("Simulation result for {number_of_issues} items written to {output}"),
        format!("Simulation histogram written to {histogram_path}"),
    ])
}
