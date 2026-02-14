use serde::Serialize;

#[derive(Serialize, Debug, Clone)]
pub struct SimulationPercentile {
    pub days: f32,
    pub date: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct SimulationReport {
    pub data_source: String,
    pub start_date: String,
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
    pub p0: f32,
    pub p50: f32,
    pub p85: f32,
    pub p100: f32,
}

#[derive(Serialize, Debug, Clone)]
pub struct WorkPackageSimulation {
    pub id: String,
    pub percentiles: WorkPackagePercentiles,
}

#[derive(Serialize, Debug, Clone)]
pub struct SimulationOutput {
    pub report: SimulationReport,
    pub results: Vec<f32>,
    pub work_packages: Option<Vec<WorkPackageSimulation>>,
}
