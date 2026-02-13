use crate::commands::base_commands::Commands;
use crate::services::project_simulation::simulate_project_from_yaml_file;

pub async fn simulate_command(cmd: Commands) {
    if let Commands::Simulate {
        input,
        output,
        iterations,
    } = cmd
    {
        let report = match simulate_project_from_yaml_file(&input, iterations).await {
            Ok(report) => report,
            Err(e) => {
                eprintln!("Failed to simulate project: {e:?}");
                return;
            }
        };

        let yaml = match serde_yaml::to_string(&report) {
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
        }
    }
}
