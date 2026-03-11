use crate::commands::base_commands::Commands;
use crate::services::simulation_gantt::write_simulation_gantt_markdown;

pub fn plot_simulation_gantt_command(cmd: Commands) {
    if let Commands::PlotSimulationGantt {
        input,
        report,
        output,
    } = cmd
    {
        if let Err(e) = write_simulation_gantt_markdown(&input, &report, &output) {
            eprintln!("Failed to write simulation Gantt diagram: {e:?}");
            std::process::exit(1);
        } else {
            println!("Simulation Gantt diagram written to {output}");
        }
    }
}
