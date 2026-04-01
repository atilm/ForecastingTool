use thiserror::Error;

use crate::services::data_source::DataSourceError;
use crate::services::plotting::burndown_plot::BurndownPlotError;
use crate::services::plotting::estimate_gantt::EstimateGanttError;
use crate::services::plotting::project_flow_diagram::ProjectDiagramError;
use crate::services::plotting::simulation_gantt::SimulationGanttError;
use crate::services::plotting::throughput_plot::ThroughputPlotError;
use crate::services::project_simulation::project_simulation::ProjectSimulationError;
use crate::services::project_simulation::throughput_simulation::SimulationError;

pub mod base_commands;
pub mod get_project_cmd;
pub mod get_throughput_cmd;
pub mod plot_burndown_cmd;
pub mod plot_gantt_cmd;
pub mod plot_project_cmd;
pub mod plot_simulation_gantt_cmd;
pub mod plot_throughput_cmd;
pub mod report_format;
pub mod simulate_cmd;
pub mod simulate_n_cmd;

pub type CommandResult = Result<Vec<String>, CommandError>;

#[derive(Error, Debug)]
pub enum CommandError {
	#[error("failed to parse Jira config: {0}")]
	ParseJiraConfig(#[source] DataSourceError),
	#[error("failed to load Jira auth: {0}")]
	LoadJiraAuth(#[source] DataSourceError),
	#[error("failed to create Jira API client: {0}")]
	CreateJiraApiClient(#[source] DataSourceError),
	#[error("failed to get throughput data: {0}")]
	GetThroughputData(#[source] DataSourceError),
	#[error("failed to get project data: {0}")]
	GetProjectData(#[source] DataSourceError),
	#[error("failed to serialize throughput to YAML: {0}")]
	SerializeThroughput(#[source] std::io::Error),
	#[error("failed to serialize project to YAML: {0}")]
	SerializeProject(#[source] std::io::Error),
	#[error("failed to write output file: {0}")]
	WriteOutput(#[source] std::io::Error),
	#[error("failed to write project diagram: {0}")]
	PlotProject(#[source] ProjectDiagramError),
	#[error("failed to write Gantt diagram: {0}")]
	PlotGantt(#[source] EstimateGanttError),
	#[error("failed to write simulation Gantt diagram: {0}")]
	PlotSimulationGantt(#[source] SimulationGanttError),
	#[error("failed to plot burndown: {0}")]
	PlotBurndown(#[source] BurndownPlotError),
	#[error("failed to plot throughput: {0}")]
	PlotThroughput(#[source] ThroughputPlotError),
	#[error("failed to simulate project: {0}")]
	SimulateProject(#[source] ProjectSimulationError),
	#[error("failed to serialize simulation output: {0}")]
	SerializeSimulation(#[source] serde_yaml::Error),
	#[error("failed to simulate by throughput: {0}")]
	SimulateThroughput(#[source] SimulationError),
}
