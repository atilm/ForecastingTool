use crate::commands::base_commands::PlotGanttArgs;
use crate::commands::{CommandError, CommandResult};
use crate::services::plotting::estimate_gantt::write_pert_gantt_markdown;

pub fn plot_gantt_command(args: PlotGanttArgs) -> CommandResult {
    let PlotGanttArgs {
        input,
        output,
        calendar_dir,
        start_date,
    } = args;

    write_pert_gantt_markdown(&input, &output, start_date, calendar_dir.as_deref())
        .map_err(CommandError::PlotGantt)?;

    Ok(vec![format!("Gantt diagram written to {output}")])
}
