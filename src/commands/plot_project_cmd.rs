use crate::commands::base_commands::Commands;
use crate::services::project_flow_diagram::write_project_diagram_markdown;

pub fn plot_project_command(cmd: Commands) {
    if let Commands::PlotProject { input, output } = cmd {
        if let Err(e) = write_project_diagram_markdown(&input, &output) {
            eprintln!("Failed to write project diagram: {e:?}");
        } else {
            println!("Project diagram written to {output}");
        }
    }
}
