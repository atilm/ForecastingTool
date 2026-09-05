use crate::domain::estimate::Estimate;
use chrono::NaiveDate;
use serde::Deserialize;
use serde::Serialize;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct SimulationPercentile {
    pub days: f32,
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
    pub p15: SimulationPercentile,
    pub p50: SimulationPercentile,
    pub p85: SimulationPercentile,
    pub p100: SimulationPercentile,
    pub work_packages: Option<Vec<WorkPackageSimulation>>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct WorkPackagePercentiles {
    pub p0: SimulationPercentile,
    pub p15: SimulationPercentile,
    pub p50: SimulationPercentile,
    pub p85: SimulationPercentile,
    pub p100: SimulationPercentile,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub enum WorkPackageType {
    #[serde(rename = "ToDo")]
    ToDo,
    #[serde(rename = "DynamicToDo")]
    DynamicToDo,
    #[serde(rename = "InProgress")]
    InProgress,
    #[serde(rename = "Done")]
    Done,
    #[serde(rename = "Milestone")]
    Milestone,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct WorkPackageSimulation {
    pub id: String,
    #[serde(rename = "type")]
    pub work_package_type: WorkPackageType,
    pub estimate: Option<Estimate>,
    pub done_date: Option<NaiveDate>,
    pub percentiles: WorkPackagePercentiles,
}

#[derive(Debug, Clone)]
pub struct SimulationOutput {
    pub report: SimulationReport,
    pub results: Vec<f32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn naive_date_serializes_and_deserializes_as_yyyy_mm_dd_in_yaml() {
        let percentile = SimulationPercentile {
            days: 12.5,
            end_date: NaiveDate::from_ymd_opt(2026, 2, 22).unwrap(),
        };

        let yaml = serde_yaml::to_string(&percentile).unwrap();
        assert!(yaml.contains("2026-02-22"));

        let decoded: SimulationPercentile = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(decoded.end_date, percentile.end_date);
        assert_eq!(decoded.days, percentile.days);
    }

    #[test]
    fn work_package_serializes_lifecycle_estimate_and_done_date() {
        let work_package = WorkPackageSimulation {
            id: "DONE-1".to_string(),
            work_package_type: WorkPackageType::Done,
            estimate: Some(Estimate::StoryPoint(
                crate::domain::estimate::StoryPointEstimate {
                    estimate: Some(3.0),
                },
            )),
            done_date: Some(NaiveDate::from_ymd_opt(2026, 2, 22).unwrap()),
            percentiles: WorkPackagePercentiles {
                p0: SimulationPercentile {
                    days: 0.0,
                    end_date: NaiveDate::from_ymd_opt(2026, 2, 22).unwrap(),
                },
                p15: SimulationPercentile {
                    days: 0.0,
                    end_date: NaiveDate::from_ymd_opt(2026, 2, 22).unwrap(),
                },
                p50: SimulationPercentile {
                    days: 0.0,
                    end_date: NaiveDate::from_ymd_opt(2026, 2, 22).unwrap(),
                },
                p85: SimulationPercentile {
                    days: 0.0,
                    end_date: NaiveDate::from_ymd_opt(2026, 2, 22).unwrap(),
                },
                p100: SimulationPercentile {
                    days: 0.0,
                    end_date: NaiveDate::from_ymd_opt(2026, 2, 22).unwrap(),
                },
            },
        };

        let yaml = serde_yaml::to_string(&work_package).unwrap();
        assert!(yaml.contains("type: Done"));
        assert!(yaml.contains("type: story_point"));
        assert!(yaml.contains("2026-02-22"));
        let decoded: WorkPackageSimulation = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(decoded.work_package_type, WorkPackageType::Done);
    }
}
