use clap::{Parser, Subcommand};
use clap_complete::Shell;
use chrono::Local;

#[derive(Parser)]
#[command(author, version, about)]
pub struct CliArgs {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Get throughput data from Jira and serialize to YAML
    GetThroughput {
        /// Path to Jira config YAML
        #[arg(short, long)]
        config: String,
        /// Output YAML file
        #[arg(short, long)]
        output: String,
    },
    /// Plot throughput data from YAML into a PNG chart
    PlotThroughput {
        /// Throughput YAML file
        #[arg(short, long)]
        input: String,
        /// Output PNG file
        #[arg(short, long)]
        output: String,
    },
    /// Plot project dependencies as a Mermaid diagram
    PlotProject {
        /// Project YAML file
        #[arg(short, long)]
        input: String,
        /// Output Markdown file
        #[arg(short, long)]
        output: String,
    },
    /// Get project data from Jira and serialize to YAML
    GetProject {
        /// Path to Jira config YAML
        #[arg(short, long)]
        config: String,
        /// Output YAML file
        #[arg(short, long)]
        output: String,
    },
    /// Simulate project completion with dependency-aware Monte Carlo
    Simulate {
        /// Project YAML file
        #[arg(short, long)]
        input: String,
        /// Output YAML file
        #[arg(short, long)]
        output: String,
        /// Simulation start date (YYYY-MM-DD)
        #[arg(short, long, default_value_t = default_start_date())]
        start_date: String,
        /// Number of simulation iterations
        #[arg(short = 'n', long, default_value_t = 10000)]
        iterations: usize,
        /// Optional path to a calendar directory
        #[arg(short, long)]
        calendar_dir: Option<String>,
    },
    /// Simulate completion dates from throughput data
    SimulateN {
        /// Throughput YAML file
        #[arg(short = 'f', long)]
        throughput: String,
        /// Output YAML file
        #[arg(short, long)]
        output: String,
        /// Number of simulation iterations
        #[arg(short, long, default_value_t = 10000)]
        iterations: usize,
        /// Number of issues to simulate
        #[arg(short, long)]
        number_of_issues: usize,
        /// Simulation start date (YYYY-MM-DD)
        #[arg(short, long, default_value_t = default_start_date())]
        start_date: String,
    },
    /// Generate shell completion scripts
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },
}

fn default_start_date() -> String {
    Local::now().date_naive().format("%Y-%m-%d").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simulate_defaults_start_date_to_today() {
        let args = CliArgs::parse_from([
            "forecasts",
            "simulate",
            "-i",
            "input.yaml",
            "-o",
            "output.yaml",
        ]);

        if let Commands::Simulate { start_date, .. } = args.command {
            assert_eq!(start_date, default_start_date());
        } else {
            panic!("expected simulate command");
        }
    }

    #[test]
    fn simulate_n_defaults_start_date_to_today() {
        let args = CliArgs::parse_from([
            "forecasts",
            "simulate-n",
            "-f",
            "throughput.yaml",
            "-o",
            "output.yaml",
            "-n",
            "5",
        ]);

        if let Commands::SimulateN { start_date, .. } = args.command {
            assert_eq!(start_date, default_start_date());
        } else {
            panic!("expected simulate-n command");
        }
    }
}