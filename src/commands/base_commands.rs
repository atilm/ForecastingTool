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