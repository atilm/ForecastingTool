use crate::services::project_simulation::simulation_types::SimulationReport;
use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ReportParseError {
    #[error("failed to read report file: {0}")]
    Io(#[from] io::Error),
    #[error("failed to parse report yaml: {0}")]
    Parse(#[from] serde_yaml::Error),
}

pub fn load_simulation_report_from_file(path: &str) -> Result<SimulationReport, ReportParseError> {
    let contents = std::fs::read_to_string(path)?;
    parse_simulation_report_str(&contents)
}

fn parse_simulation_report_str(input: &str) -> Result<SimulationReport, ReportParseError> {
    let report: SimulationReport = serde_yaml::from_str(input)?;
    Ok(report)
}
