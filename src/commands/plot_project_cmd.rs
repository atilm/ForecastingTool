use crate::commands::base_commands::PlotProjectArgs;
use crate::services::plotting::project_flow_diagram::write_project_diagram_markdown;

pub fn plot_project_command(args: PlotProjectArgs) {
    let PlotProjectArgs { input, output } = args;
    if let Err(e) = write_project_diagram_markdown(&input, &output) {
        eprintln!("Failed to write project diagram: {e:?}");
    } else {
        println!("Project diagram written to {output}");
    }
}
