use crate::commands::base_commands::PlotProjectArgs;
use crate::commands::{CommandError, CommandResult};
use crate::services::plotting::project_flow_diagram::write_project_diagram_markdown;

pub fn plot_project_command(args: PlotProjectArgs) -> CommandResult {
    let PlotProjectArgs { input, output } = args;
    write_project_diagram_markdown(&input, &output).map_err(CommandError::PlotProject)?;

    Ok(vec![format!("Project diagram written to {output}")])
}
