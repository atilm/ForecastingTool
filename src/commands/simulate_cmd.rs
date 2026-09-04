use crate::commands::base_commands::SimulateProjectArgs;
use crate::commands::report_format::format_simulation_report;
use crate::commands::{CommandError, CommandResult};
use crate::services::parsing::histogram_yaml::serialize_histogram_to_yaml_file;
use crate::services::plotting::histogram::write_histogram_png;
use crate::services::plotting::milestone_plot::write_milestone_plot_png;
use crate::services::project_simulation::project_simulation::simulate_project_from_yaml_file_with_creation_rate;
use crate::services::util::histogram::Histogram;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

fn histogram_paths_from_output(output: &str) -> (String, String) {
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

    let histogram_png_path = histogram_base.with_extension("png");
    let histogram_yaml_path = histogram_base.with_extension("yaml");
    (
        histogram_png_path.to_string_lossy().to_string(),
        histogram_yaml_path.to_string_lossy().to_string(),
    )
}

pub fn simulate_command(args: SimulateProjectArgs) -> CommandResult {
    let SimulateProjectArgs {
        input,
        output,
        iterations,
        start_date,
        calendar_dir,
        story_point_creation_rate,
    } = args;

    let simulation = simulate_project_from_yaml_file_with_creation_rate(
        &input,
        iterations,
        start_date,
        calendar_dir.as_deref(),
        story_point_creation_rate,
    )
    .map_err(CommandError::SimulateProject)?;

    let (histogram_path, histogram_yaml_path) = histogram_paths_from_output(&output);
    let mut messages = Vec::new();

    match Histogram::create(&simulation.results) {
        Ok(histogram) => {
            match serialize_histogram_to_yaml_file(&histogram_yaml_path, &histogram) {
                Ok(()) => messages.push(format!(
                    "Simulation histogram written to {histogram_yaml_path}"
                )),
                Err(error) => messages.push(format!(
                    "Warning: failed to write simulation histogram yaml: {error}"
                )),
            }

            match write_histogram_png(&histogram_path, &histogram) {
                Ok(()) => {
                    messages.push(format!("Simulation histogram written to {histogram_path}"))
                }
                Err(error) => messages.push(format!(
                    "Warning: failed to write simulation histogram png: {error}"
                )),
            }
        }
        Err(error) => messages.push(format!(
            "Warning: failed to build simulation histogram: {error}"
        )),
    }

    let milestone_plot_path = format!("{output}.milestones.png");
    match write_milestone_plot_png(&milestone_plot_path, &simulation) {
        Ok(()) => messages.push(format!("Milestone plot written to {milestone_plot_path}")),
        Err(error) => messages.push(format!("Warning: failed to write milestone plot: {error}")),
    }

    let yaml =
        serde_yaml::to_string(&simulation.report).map_err(CommandError::SerializeSimulation)?;
    std::fs::write(&output, yaml).map_err(CommandError::WriteOutput)?;

    messages.insert(0, format!("Simulation result written to {output}"));
    messages.insert(0, format_simulation_report(&simulation.report));

    Ok(messages)
}
