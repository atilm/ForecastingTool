use crate::commands::base_commands::Commands;
use crate::services::estimate_gantt::write_pert_gantt_markdown;

pub fn plot_gantt_command(cmd: Commands) {
    if let Commands::PlotGantt { input, output } = cmd {
        if let Err(e) = write_pert_gantt_markdown(&input, &output) {
            eprintln!("Failed to write Gantt diagram: {e:?}");
        } else {
            println!("Gantt diagram written to {output}");
        }
    }
}
