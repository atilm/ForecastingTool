use crate::commands::base_commands::PlotBurndownArgs;
use crate::commands::{CommandError, CommandResult};
use crate::services::plotting::burndown_plot::plot_burndown_from_yaml_files;

pub fn plot_burndown_command(args: PlotBurndownArgs) -> CommandResult {
    let PlotBurndownArgs {
        input,
        report,
        output,
        calendar_dir,
    } = args;

    plot_burndown_from_yaml_files(&input, &report, &output, calendar_dir.as_deref())
        .map_err(CommandError::PlotBurndown)?;

    Ok(vec![format!("Burndown plot written to {output}")])
}
