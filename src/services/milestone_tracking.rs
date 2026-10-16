use std::path::{Path, PathBuf};

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::services::project_simulation::simulation_types::{SimulationOutput, WorkPackageType};

#[derive(Debug, Error)]
pub enum MilestoneTrackingError {
    #[error("failed to read milestone tracking file '{path}': {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse milestone tracking file '{path}': {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: serde_yaml::Error,
    },
    #[error("milestone '{0}' is missing a summary")]
    MissingSummary(String),
    #[error("failed to serialize milestone tracking data: {0}")]
    Serialize(#[from] serde_yaml::Error),
    #[error("failed to write milestone tracking file '{path}': {source}")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

#[derive(Debug, Deserialize, Serialize)]
struct MilestoneTracking {
    start_date: NaiveDate,
    end_date: NaiveDate,
    milestones: Vec<TrackedMilestone>,
}

#[derive(Debug, Deserialize, Serialize)]
struct TrackedMilestone {
    id: String,
    name: String,
    #[serde(default = "default_plot")]
    plot: bool,
    forecasts: Vec<MilestoneForecast>,
}

#[derive(Debug, Deserialize, Serialize)]
struct MilestoneForecast {
    date: NaiveDate,
    p0: NaiveDate,
    p15: NaiveDate,
    p50: NaiveDate,
    p85: NaiveDate,
    p100: NaiveDate,
}

fn default_plot() -> bool {
    true
}

pub fn update_milestones_file(
    path: &Path,
    simulation: &SimulationOutput,
    run_date: NaiveDate,
) -> Result<(), MilestoneTrackingError> {
    let report = &simulation.report;
    let mut tracking: MilestoneTracking = match std::fs::read_to_string(path) {
        Ok(yaml) => {
            serde_yaml::from_str(&yaml).map_err(|source| MilestoneTrackingError::Parse {
                path: path.to_path_buf(),
                source,
            })?
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => MilestoneTracking {
            start_date: report.start_date,
            end_date: report.p100.end_date,
            milestones: Vec::new(),
        },
        Err(source) => {
            return Err(MilestoneTrackingError::Read {
                path: path.to_path_buf(),
                source,
            });
        }
    };

    let milestones = report
        .work_packages
        .iter()
        .flatten()
        .filter(|work_package| work_package.work_package_type == WorkPackageType::Milestone);
    let mut latest_end_date = None;
    for work_package in milestones {
        let name = simulation
            .milestone_summaries
            .get(&work_package.id)
            .and_then(|summary| summary.as_ref())
            .ok_or_else(|| MilestoneTrackingError::MissingSummary(work_package.id.clone()))?;
        let forecast = MilestoneForecast {
            date: run_date,
            p0: work_package.percentiles.p0.end_date,
            p15: work_package.percentiles.p15.end_date,
            p50: work_package.percentiles.p50.end_date,
            p85: work_package.percentiles.p85.end_date,
            p100: work_package.percentiles.p100.end_date,
        };
        latest_end_date = Some(latest_end_date.unwrap_or(forecast.p100).max(forecast.p100));

        if let Some(milestone) = tracking
            .milestones
            .iter_mut()
            .find(|milestone| milestone.id == work_package.id)
        {
            milestone.name = name.clone();
            milestone.forecasts.retain(|entry| entry.date != run_date);
            milestone.forecasts.push(forecast);
        } else {
            tracking.milestones.push(TrackedMilestone {
                id: work_package.id.clone(),
                name: name.clone(),
                plot: true,
                forecasts: vec![forecast],
            });
        }
    }
    if let Some(end_date) = latest_end_date {
        tracking.end_date = end_date;
    }

    let yaml = serde_yaml::to_string(&tracking)?;
    std::fs::write(path, yaml).map_err(|source| MilestoneTrackingError::Write {
        path: path.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests;
