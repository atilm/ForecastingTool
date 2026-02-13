use serde::Serialize;

#[derive(Serialize, Debug, Clone)]
pub struct SimulationPercentile {
    pub days: f32,
    pub date: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct SimulationReport {
    pub start_date: String,
    pub simulated_items: usize,
    pub p0: SimulationPercentile,
    pub p50: SimulationPercentile,
    pub p85: SimulationPercentile,
    pub p100: SimulationPercentile,
}

#[derive(Serialize, Debug, Clone)]
pub struct SimulationOutput {
    pub report: SimulationReport,
    pub results: Vec<f32>,
}
