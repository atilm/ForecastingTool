use crate::commands::base_commands::Commands;
use crate::commands::date_parsing;
use crate::commands::report_format::format_simulation_report;
use crate::services::gantt_diagram::generate_gantt_diagram;
use crate::services::histogram::write_histogram_png;
use crate::services::project_simulation::simulate_project_from_yaml_file;
use crate::services::project_yaml::load_project_from_yaml_file;

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
        if let Err(e) = write_histogram_png(&histogram_path, &simulation.results) {
            eprintln!("Failed to write simulation histogram: {e:?}");
        }

        let gantt_path = format!("{output}.gantt.md");
        match load_project_from_yaml_file(&input) {
            Ok(project) => match generate_gantt_diagram(&project, &simulation, start_date, 85.0) {
                Ok(diagram) => {
                    if let Err(e) = std::fs::write(&gantt_path, diagram) {
                        eprintln!("Failed to write gantt diagram: {e:?}");
                    }
                }
                Err(e) => eprintln!("Failed to generate gantt diagram: {e:?}"),
            },
            Err(e) => eprintln!("Failed to load project for gantt diagram: {e:?}"),
        }

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
            println!("Simulation histogram written to {histogram_path}");
            println!("Gantt diagram written to {gantt_path}");
        }
    }
}
