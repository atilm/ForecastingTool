use crate::commands::base_commands::Commands;
use crate::commands::date_parsing;
use crate::commands::report_format::format_simulation_report;
use crate::services::histogram::write_histogram_png;
use crate::services::mileston_plot::write_milestone_plot_png;
use crate::services::project_simulation::project_simulation::simulate_project_from_yaml_file;

pub fn simulate_command(cmd: Commands) {
    if let Commands::Simulate {
        input,
        output,
        iterations,
        start_date,
        calendar_dir,
    } = cmd
    {
        let start_date = date_parsing::parse_date(&start_date).unwrap_or_else(|e| {
            eprintln!("Failed to parse start date: {e:?}");
            std::process::exit(1);
        });

        let simulation = match simulate_project_from_yaml_file(
            &input,
            iterations,
            start_date,
            calendar_dir.as_deref(),
        ) {
            Ok(report) => report,
            Err(e) => {
                eprintln!("Failed to simulate project: {e:?}");
                return;
            }
        };

        let histogram_path = format!("{output}.png");
        let histogram_written = match write_histogram_png(&histogram_path, &simulation.results) {
            Ok(()) => true,
            Err(e) => {
                eprintln!("Failed to write simulation histogram: {e:?}");
                false
            }
        };

        let milestone_plot_path = format!("{output}.milestones.png");
        let milestone_plot_written =
            match write_milestone_plot_png(&milestone_plot_path, &simulation) {
                Ok(()) => true,
                Err(e) => {
                    eprintln!("Failed to write milestone plot: {e}");
                    false
                }
            };

        let yaml = match serde_yaml::to_string(&simulation.report) {
            Ok(contents) => contents,
            Err(e) => {
                eprintln!("Failed to serialize simulation output: {e:?}");
                return;
            }
        };

        if let Err(e) = std::fs::write(&output, yaml) {
            eprintln!("Failed to write simulation output: {e:?}");
        } else {
            println!("{}", format_simulation_report(&simulation.report));
            println!("Simulation result written to {output}");
            if histogram_written {
                println!("Simulation histogram written to {histogram_path}");
            }
            if milestone_plot_written {
                println!("Milestone plot written to {milestone_plot_path}");
            }
        }
    }
}
