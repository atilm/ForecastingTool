use crate::commands::base_commands::Commands;
use crate::services::gantt_diagram::generate_gantt_diagram;
use crate::services::histogram::write_histogram_png;
use crate::services::project_yaml::load_project_from_yaml_file;
use crate::services::project_simulation::simulate_project_from_yaml_file;

pub async fn simulate_command(cmd: Commands) {
    if let Commands::Simulate {
        input,
        output,
        iterations,
        start_date,
    } = cmd
    {
        let simulation = match simulate_project_from_yaml_file(&input, iterations, &start_date).await {
            Ok(report) => report,
            Err(e) => {
                eprintln!("Failed to simulate project: {e:?}");
                return;
            }
        };

        let histogram_path = format!("{output}.png");
        if let Err(e) = write_histogram_png(&histogram_path, &simulation.results).await {
            eprintln!("Failed to write simulation histogram: {e:?}");
        }

        let gantt_path = format!("{output}.gantt.md");
        match load_project_from_yaml_file(&input).await {
            Ok(project) => {
                if let Ok(start_date) = chrono::NaiveDate::parse_from_str(&start_date, "%Y-%m-%d") {
                    match generate_gantt_diagram(&project, &simulation, start_date, 85.0) {
                        Ok(diagram) => {
                            if let Err(e) = tokio::fs::write(&gantt_path, diagram).await {
                                eprintln!("Failed to write gantt diagram: {e:?}");
                            }
                        }
                        Err(e) => eprintln!("Failed to generate gantt diagram: {e:?}"),
                    }
                }
            }
            Err(e) => eprintln!("Failed to load project for gantt diagram: {e:?}"),
        }

        let yaml = match serde_yaml::to_string(&simulation) {
            Ok(contents) => contents,
            Err(e) => {
                eprintln!("Failed to serialize simulation output: {e:?}");
                return;
            }
        };

        if let Err(e) = tokio::fs::write(&output, yaml).await {
            eprintln!("Failed to write simulation output: {e:?}");
        } else {
            println!("Simulation result written to {output}");
            println!("Simulation histogram written to {histogram_path}");
            println!("Gantt diagram written to {gantt_path}");
        }
    }
}
