use crate::commands::base_commands::Commands;
use crate::services::simulation::simulate_from_throughput_file;

pub async fn simulate_n_command(cmd: Commands) {
    if let Commands::SimulateN {
        throughput,
        output,
        iterations,
        number_of_issues,
        start_date,
    } = cmd
    {
        let histogram_path = format!("{output}.png");
        let simulation = match simulate_from_throughput_file(
            &throughput,
            iterations,
            number_of_issues,
            &start_date,
            &histogram_path,
        )
        .await
        {
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

        if let Err(e) = tokio::fs::write(&output, yaml).await {
            eprintln!("Failed to write simulation output: {e:?}");
        } else {
            println!("Simulation result for {number_of_issues} items written to {output}");
            println!("Simulation histogram written to {histogram_path}");
        }
    }
}
