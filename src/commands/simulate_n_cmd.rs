use crate::commands::base_commands::SimulateThroughputArgs;
use crate::commands::report_format::format_simulation_report;
use crate::commands::{CommandError, CommandResult};
use crate::services::project_simulation::throughput_simulation::simulate_from_throughput_file;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

fn histogram_png_path_from_output(output: &str) -> String {
    let output_path = Path::new(output);
    let histogram_base = if output_path.file_name().is_some() {
        let mut path = output_path.to_path_buf();
        let base_name = output_path
            .file_stem()
            .map(|name| name.to_os_string())
            .or_else(|| output_path.file_name().map(|name| name.to_os_string()))
            .unwrap_or_else(|| OsString::from("output"));
        let mut histogram_name = base_name;
        histogram_name.push("_histogram");
        path.set_file_name(histogram_name);
        path
    } else {
        PathBuf::from(format!("{output}_histogram"))
    };

    histogram_base
        .with_extension("png")
        .to_string_lossy()
        .to_string()
}

pub fn simulate_n_command(args: SimulateThroughputArgs) -> CommandResult {
    let SimulateThroughputArgs {
        throughput,
        output,
        iterations,
        number_of_issues,
        start_date,
        calendar_dir,
    } = args;

    let histogram_path = histogram_png_path_from_output(&output);
    let simulation = simulate_from_throughput_file(
        &throughput,
        iterations,
        number_of_issues,
        start_date,
        &histogram_path,
        calendar_dir.as_deref(),
    )
    .map_err(CommandError::SimulateThroughput)?;

    let yaml = serde_yaml::to_string(&simulation).map_err(CommandError::SerializeSimulation)?;
    std::fs::write(&output, yaml).map_err(CommandError::WriteOutput)?;

    Ok(vec![
        format_simulation_report(&simulation),
        format!("Simulation result for {number_of_issues} items written to {output}"),
        format!("Simulation histogram written to {histogram_path}"),
    ])
}
