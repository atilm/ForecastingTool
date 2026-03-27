use chrono::{Local, NaiveDate};
use clap::{Args, Parser, Subcommand};
use clap_complete::Shell;

#[derive(Parser)]
#[command(author, version, about)]
pub struct CliArgs {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Fetch data from external systems
    Get {
        #[command(subcommand)]
        command: GetCommands,
    },
    /// Generate charts and diagrams
    Plot {
        #[command(subcommand)]
        command: PlotCommands,
    },
    /// Run Monte Carlo simulations
    Simulate {
        #[command(subcommand)]
        command: SimulateCommands,
    },
    /// Utility commands
    Util {
        #[command(subcommand)]
        command: UtilCommands,
    },
}

#[derive(Subcommand)]
pub enum GetCommands {
    /// Get throughput data from Jira and serialize to YAML
    Throughput(GetThroughputArgs),
    /// Get project data from Jira and serialize to YAML
    Project(GetProjectArgs),
}

#[derive(Subcommand)]
pub enum PlotCommands {
    /// Plot throughput data from YAML into a PNG chart
    Throughput(PlotThroughputArgs),
    /// Plot project dependencies as a Mermaid diagram
    Project(PlotProjectArgs),
    /// Plot a Gantt diagram using PERT expected durations from a project YAML
    Gantt(PlotGanttArgs),
    /// Plot a simulation Gantt diagram from a project YAML and a simulation report YAML
    #[command(name = "simulation-gantt")]
    SimulationGantt(PlotSimulationGanttArgs),
    /// Plot a burndown chart using project and simulation result YAML files
    Burndown(PlotBurndownArgs),
}

#[derive(Subcommand)]
pub enum SimulateCommands {
    /// Simulate project completion with dependency-aware Monte Carlo
    Project(SimulateProjectArgs),
    /// Simulate completion dates from throughput data
    Throughput(SimulateThroughputArgs),
}

#[derive(Subcommand)]
pub enum UtilCommands {
    /// Show the Git hash of the current build
    #[command(name = "git-hash")]
    GitHash,
    /// Generate shell completion scripts
    Completions(CompletionsArgs),
}

#[derive(Args)]
pub struct GetThroughputArgs {
    /// Path to Jira config YAML
    #[arg(short, long)]
    pub config: String,
    /// Output YAML file
    #[arg(short, long)]
    pub output: String,
}

#[derive(Args)]
pub struct GetProjectArgs {
    /// Path to Jira config YAML
    #[arg(short, long)]
    pub config: String,
    /// Output YAML file
    #[arg(short, long)]
    pub output: String,
}

#[derive(Args)]
pub struct PlotThroughputArgs {
    /// Throughput YAML file
    #[arg(short, long)]
    pub input: String,
    /// Output PNG file
    #[arg(short, long)]
    pub output: String,
}

#[derive(Args)]
pub struct PlotProjectArgs {
    /// Project YAML file
    #[arg(short, long)]
    pub input: String,
    /// Output Markdown file
    #[arg(short, long)]
    pub output: String,
}

#[derive(Args)]
pub struct PlotGanttArgs {
    /// Project YAML file
    #[arg(short, long)]
    pub input: String,
    /// Output Markdown file
    #[arg(short, long)]
    pub output: String,
    /// Optional path to a calendar directory
    #[arg(short, long)]
    pub calendar_dir: Option<String>,
    /// Project start date (YYYY-MM-DD)
    #[arg(short, long, default_value_t = default_start_date())]
    pub start_date: NaiveDate,
}

#[derive(Args)]
pub struct PlotSimulationGanttArgs {
    /// Project YAML file
    #[arg(short, long)]
    pub input: String,
    /// Simulation report YAML file
    #[arg(short, long)]
    pub report: String,
    /// Output Markdown file
    #[arg(short, long)]
    pub output: String,
}

#[derive(Args)]
pub struct PlotBurndownArgs {
    /// Project YAML file
    #[arg(short, long)]
    pub input: String,
    /// Simulation result YAML file
    #[arg(short, long)]
    pub report: String,
    /// Output PNG file
    #[arg(short, long)]
    pub output: String,
    /// Optional path to a calendar directory
    #[arg(short, long)]
    pub calendar_dir: Option<String>,
}

#[derive(Args)]
pub struct SimulateProjectArgs {
    /// Project YAML file
    #[arg(short, long)]
    pub input: String,
    /// Output YAML file
    #[arg(short, long)]
    pub output: String,
    /// Simulation start date (YYYY-MM-DD)
    #[arg(short, long, default_value_t = default_start_date())]
    pub start_date: NaiveDate,
    /// Number of simulation iterations
    #[arg(short = 'n', long, default_value_t = 10000)]
    pub iterations: usize,
    /// Optional path to a calendar directory
    #[arg(short, long)]
    pub calendar_dir: Option<String>,
}

#[derive(Args)]
pub struct SimulateThroughputArgs {
    /// Throughput YAML file
    #[arg(short = 'f', long)]
    pub throughput: String,
    /// Output YAML file
    #[arg(short, long)]
    pub output: String,
    /// Number of simulation iterations
    #[arg(short = 'n', long, default_value_t = 10000)]
    pub iterations: usize,
    /// Number of issues to simulate
    #[arg(short = 'k', long)]
    pub number_of_issues: usize,
    /// Simulation start date (YYYY-MM-DD)
    #[arg(short, long, default_value_t = default_start_date())]
    pub start_date: NaiveDate,
    /// Optional path to a calendar directory
    #[arg(short, long)]
    pub calendar_dir: Option<String>,
}

#[derive(Args)]
pub struct CompletionsArgs {
    /// Shell to generate completions for
    #[arg(value_enum)]
    pub shell: Shell,
}

fn default_start_date() -> NaiveDate {
    Local::now().date_naive()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simulate_project_defaults_start_date_to_today() {
        let args = CliArgs::parse_from([
            "forecasts",
            "simulate",
            "project",
            "-i",
            "input.yaml",
            "-o",
            "output.yaml",
        ]);

        if let Commands::Simulate {
            command: SimulateCommands::Project(simulate),
        } = args.command
        {
            assert_eq!(simulate.start_date, default_start_date());
            assert_eq!(simulate.iterations, 10000);
        } else {
            panic!("expected simulate project command");
        }
    }

    #[test]
    fn simulate_throughput_defaults_start_date_to_today() {
        let args = CliArgs::parse_from([
            "forecasts",
            "simulate",
            "throughput",
            "-f",
            "throughput.yaml",
            "-o",
            "output.yaml",
            "-k",
            "5",
        ]);

        if let Commands::Simulate {
            command: SimulateCommands::Throughput(simulate),
        } = args.command
        {
            assert_eq!(simulate.start_date, default_start_date());
            assert_eq!(simulate.number_of_issues, 5);
            assert_eq!(simulate.iterations, 10000);
        } else {
            panic!("expected simulate throughput command");
        }
    }

    #[test]
    fn plot_burndown_accepts_optional_calendar_dir() {
        let args = CliArgs::parse_from([
            "forecasts",
            "plot",
            "burndown",
            "-i",
            "input.yaml",
            "-r",
            "report.yaml",
            "-o",
            "output.png",
            "--calendar-dir",
            "calendars",
        ]);

        if let Commands::Plot {
            command: PlotCommands::Burndown(plot),
        } = args.command
        {
            assert_eq!(plot.calendar_dir.as_deref(), Some("calendars"));
        } else {
            panic!("expected plot burndown command");
        }
    }

    #[test]
    fn util_completions_parses_shell() {
        let args = CliArgs::parse_from(["forecasts", "util", "completions", "bash"]);

        if let Commands::Util {
            command: UtilCommands::Completions(completions),
        } = args.command
        {
            assert!(matches!(completions.shell, Shell::Bash));
        } else {
            panic!("expected util completions command");
        }
    }
}
