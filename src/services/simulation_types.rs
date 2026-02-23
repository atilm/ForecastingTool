use serde::Deserialize;
use serde::Serialize;
use chrono::NaiveDate;
use crate::domain::issue_status::IssueStatus;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct SimulationPercentile {
    pub days: f32,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct SimulationReport {
    pub data_source: String,
    pub start_date: NaiveDate,
    pub velocity: Option<f32>,
    pub iterations: usize,
    pub simulated_items: usize,
    pub p0: SimulationPercentile,
    pub p50: SimulationPercentile,
    pub p85: SimulationPercentile,
    pub p100: SimulationPercentile,
}

#[derive(Serialize, Debug, Clone)]
pub struct WorkPackagePercentiles {
    pub p0: SimulationPercentile,
    pub p50: SimulationPercentile,
    pub p85: SimulationPercentile,
    pub p100: SimulationPercentile,
}

#[derive(Debug, Clone)]
pub struct WorkPackageSimulation {
    pub id: String,
    pub status: IssueStatus,
    pub percentiles: WorkPackagePercentiles,
}

#[derive(Debug, Clone)]
pub struct SimulationOutput {
    pub report: SimulationReport,
    pub results: Vec<f32>,
    pub work_packages: Option<Vec<WorkPackageSimulation>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn naive_date_serializes_and_deserializes_as_yyyy_mm_dd_in_yaml() {
        let percentile = SimulationPercentile {
            days: 12.5,
            start_date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            end_date: NaiveDate::from_ymd_opt(2026, 2, 22).unwrap(),
        };

        let yaml = serde_yaml::to_string(&percentile).unwrap();
        assert!(yaml.contains("2026-02-22"));

        let decoded: SimulationPercentile = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(decoded.start_date, percentile.start_date);
        assert_eq!(decoded.end_date, percentile.end_date);
        assert_eq!(decoded.days, percentile.days);
    }
}
