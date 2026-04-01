use crate::commands::base_commands::PlotThroughputArgs;
use crate::commands::{CommandError, CommandResult};
use crate::services::plotting::throughput_plot::plot_throughput_from_yaml_file;

pub fn plot_throughput_command(args: PlotThroughputArgs) -> CommandResult {
    let PlotThroughputArgs { input, output } = args;
    plot_throughput_from_yaml_file(&input, &output).map_err(CommandError::PlotThroughput)?;

    Ok(vec![format!("Throughput plot written to {output}")])
}
