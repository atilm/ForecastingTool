use crate::commands::base_commands::PlotBurndownArgs;
use crate::services::burndown_plot::plot_burndown_from_yaml_files;

pub fn plot_burndown_command(args: PlotBurndownArgs) {
    let PlotBurndownArgs {
        input,
        report,
        output,
        calendar_dir,
    } = args;

    match plot_burndown_from_yaml_files(&input, &report, &output, calendar_dir.as_deref()) {
        Ok(()) => println!("Burndown plot written to {output}"),
        Err(e) => eprintln!("Failed to plot burndown: {e}"),
    }
}
