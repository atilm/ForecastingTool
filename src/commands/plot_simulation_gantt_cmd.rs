use crate::commands::base_commands::PlotSimulationGanttArgs;
use crate::services::plotting::simulation_gantt::write_simulation_gantt_markdown;

pub fn plot_simulation_gantt_command(args: PlotSimulationGanttArgs) {
    let PlotSimulationGanttArgs {
        input,
        report,
        output,
    } = args;

    if let Err(e) = write_simulation_gantt_markdown(&input, &report, &output) {
        eprintln!("Failed to write simulation Gantt diagram: {e:?}");
        std::process::exit(1);
    } else {
        println!("Simulation Gantt diagram written to {output}");
    }
}
