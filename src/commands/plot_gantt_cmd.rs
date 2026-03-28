use crate::commands::base_commands::PlotGanttArgs;
use crate::services::plotting::estimate_gantt::write_pert_gantt_markdown;

pub fn plot_gantt_command(args: PlotGanttArgs) {
    let PlotGanttArgs {
        input,
        output,
        calendar_dir,
        start_date,
    } = args;

    if let Err(e) = write_pert_gantt_markdown(&input, &output, start_date, calendar_dir.as_deref())
    {
        eprintln!("Failed to write Gantt diagram: {e:?}");
    } else {
        println!("Gantt diagram written to {output}");
    }
}
