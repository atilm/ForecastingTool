use crate::commands::base_commands::SimulateThroughputArgs;
use crate::commands::report_format::format_simulation_report;
use crate::services::project_simulation::throughput_simulation::simulate_from_throughput_file;

pub fn simulate_n_command(args: SimulateThroughputArgs) {
    let SimulateThroughputArgs {
        throughput,
        output,
        iterations,
        number_of_issues,
        start_date,
        calendar_dir,
    } = args;

    let histogram_path = format!("{output}.png");
    let simulation = match simulate_from_throughput_file(
        &throughput,
        iterations,
        number_of_issues,
        start_date,
        &histogram_path,
        calendar_dir.as_deref(),
    ) {
        Ok(result) => result,
        Err(e) => {
            eprintln!("Failed to simulate by throughput: {e:?}");
            return;
        }
    };

    let yaml = match serde_yaml::to_string(&simulation) {
        Ok(contents) => contents,
        Err(e) => {
            eprintln!("Failed to serialize simulation output: {e:?}");
            return;
        }
    };

    if let Err(e) = std::fs::write(&output, yaml) {
        eprintln!("Failed to write simulation output: {e:?}");
    } else {
        println!("{}", format_simulation_report(&simulation));
        println!("Simulation result for {number_of_issues} items written to {output}");
        println!("Simulation histogram written to {histogram_path}");
    }
}
