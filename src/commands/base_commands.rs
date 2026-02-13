use clap::{Parser, Subcommand};

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
        /// Number of simulation iterations
        #[arg(short, long, default_value_t = 10000)]
        iterations: usize,
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
        #[arg(short, long)]
        iterations: usize,
        /// Number of issues to simulate
        #[arg(short, long)]
        number_of_issues: usize,
        /// Simulation start date (YYYY-MM-DD)
        #[arg(short, long)]
        start_date: String,
    },
}