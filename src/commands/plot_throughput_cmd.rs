use crate::commands::base_commands::Commands;
use crate::services::throughput_plot::plot_throughput_from_yaml_file;

pub fn plot_throughput_command(cmd: Commands) {
    if let Commands::PlotThroughput { input, output } = cmd {
        match plot_throughput_from_yaml_file(&input, &output) {
            Ok(()) => println!("Throughput plot written to {output}"),
            Err(e) => eprintln!("Failed to plot throughput: {e:?}"),
        }
    }
}
