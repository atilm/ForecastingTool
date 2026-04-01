use crate::commands::base_commands::PlotSimulationGanttArgs;
use crate::commands::{CommandError, CommandResult};
use crate::services::plotting::simulation_gantt::write_simulation_gantt_markdown;

pub fn plot_simulation_gantt_command(args: PlotSimulationGanttArgs) -> CommandResult {
    let PlotSimulationGanttArgs {
        input,
        report,
        output,
    } = args;

    write_simulation_gantt_markdown(&input, &report, &output)
        .map_err(CommandError::PlotSimulationGantt)?;

    Ok(vec![format!("Simulation Gantt diagram written to {output}")])
}
