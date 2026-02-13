use crate::domain::throughput::Throughput;
use crate::services::throughput_yaml::{deserialize_throughput_from_yaml_str, ThroughputYamlError};
use chrono::{Datelike, NaiveDate, Weekday};
use rand::seq::SliceRandom;
use rand::Rng;
use thiserror::Error;

use crate::services::histogram::{write_histogram_png, HistogramError};
use crate::services::simulation_types::{SimulationOutput, SimulationPercentile, SimulationReport};
#[derive(Error, Debug)]
pub enum SimulationError {
    #[error("failed to read throughput file: {0}")]
    ReadThroughput(#[from] std::io::Error),
    #[error("failed to parse throughput yaml: {0}")]
    ParseThroughput(#[from] ThroughputYamlError),
    #[error("invalid start date: {0}")]
    InvalidStartDate(String),
    #[error("iterations must be greater than zero")]
    InvalidIterations,
    #[error("number of issues must be greater than zero")]
    InvalidIssueCount,
    #[error("throughput data is empty")]
    EmptyThroughput,
    #[error("throughput data has no nonzero values")]
    ZeroThroughput,
    #[error("failed to render histogram: {0}")]
    Histogram(#[from] HistogramError),
}

pub(crate) async fn simulate_from_throughput_file(
    throughput_path: &str,
    iterations: usize,
    number_of_issues: usize,
    start_date: &str,
    histogram_path: &str,
) -> Result<SimulationReport, SimulationError> {
    let throughput_yaml = tokio::fs::read_to_string(throughput_path).await?;
    let throughput = deserialize_throughput_from_yaml_str(&throughput_yaml)?;
    let start_date = NaiveDate::parse_from_str(start_date, "%Y-%m-%d")
        .map_err(|_| SimulationError::InvalidStartDate(start_date.to_string()))?;

    let simulation = run_simulation(&throughput, iterations, number_of_issues, start_date)?;
    write_histogram_png(histogram_path, &simulation.results).await?;
    Ok(simulation.report)
}

pub(crate) fn run_simulation(
    throughput: &[Throughput],
    iterations: usize,
    number_of_issues: usize,
    start_date: NaiveDate,
) -> Result<SimulationOutput, SimulationError> {
    let mut rng = rand::thread_rng();
    run_simulation_with_rng(throughput, iterations, number_of_issues, start_date, &mut rng)
}

pub(crate) fn run_simulation_with_rng<R: Rng + ?Sized>(
    throughput: &[Throughput],
    iterations: usize,
    number_of_issues: usize,
    start_date: NaiveDate,
    rng: &mut R,
) -> Result<SimulationOutput, SimulationError> {
    if iterations == 0 {
        return Err(SimulationError::InvalidIterations);
    }
    if number_of_issues == 0 {
        return Err(SimulationError::InvalidIssueCount);
    }
    if throughput.is_empty() {
        return Err(SimulationError::EmptyThroughput);
    }

    let throughput_values: Vec<usize> = throughput.iter().map(|t| t.completed_issues).collect();
    if throughput_values.iter().all(|value| *value == 0) {
        return Err(SimulationError::ZeroThroughput);
    }

    let mut results = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let days = simulate_single_run(&throughput_values, number_of_issues, start_date, rng);
        results.push(days as f32);
    }
    results.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let p0_days = percentile_value(&results, 0.0);
    let p50_days = percentile_value(&results, 50.0);
    let p85_days = percentile_value(&results, 85.0);
    let p100_days = percentile_value(&results, 100.0);

    let report = SimulationReport {
        start_date: start_date.format("%Y-%m-%d").to_string(),
        simulated_items: number_of_issues,
        p0: SimulationPercentile {
            days: p0_days,
            date: end_date_from_days(start_date, p0_days).format("%Y-%m-%d").to_string(),
        },
        p50: SimulationPercentile {
            days: p50_days,
            date: end_date_from_days(start_date, p50_days).format("%Y-%m-%d").to_string(),
        },
        p85: SimulationPercentile {
            days: p85_days,
            date: end_date_from_days(start_date, p85_days).format("%Y-%m-%d").to_string(),
        },
        p100: SimulationPercentile {
            days: p100_days,
            date: end_date_from_days(start_date, p100_days).format("%Y-%m-%d").to_string(),
        },
    };

    Ok(SimulationOutput {
        report,
        results,
        work_packages: None,
    })
}

fn simulate_single_run<R: Rng + ?Sized>(
    throughput_values: &[usize],
    number_of_issues: usize,
    start_date: NaiveDate,
    rng: &mut R,
) -> usize {
    let mut completed = 0;
    let mut days = 0;
    let mut date = next_workday(start_date);

    while completed < number_of_issues {
        days += 1;
        let throughput = throughput_values
            .choose(rng)
            .copied()
            .unwrap_or(0);
        completed = completed.saturating_add(throughput);
        if completed >= number_of_issues {
            break;
        }
        date = next_workday(date.succ_opt().unwrap());
    }

    days
}

fn percentile_value(sorted_values: &[f32], percentile: f64) -> f32 {
    if sorted_values.is_empty() {
        return 0.0;
    }
    if percentile <= 0.0 {
        return sorted_values[0];
    }
    if percentile >= 100.0 {
        return sorted_values[sorted_values.len() - 1];
    }
    let position = (percentile / 100.0) * (sorted_values.len() as f64 - 1.0);
    let index = position.round() as usize;
    sorted_values[index]
}

fn end_date_from_days(start_date: NaiveDate, days: f32) -> NaiveDate {
    let days = days.ceil().max(0.0) as usize;
    if days == 0 {
        return next_workday(start_date);
    }
    let mut date = next_workday(start_date);
    for _ in 1..days {
        date = next_workday(date.succ_opt().unwrap());
    }
    date
}

fn next_workday(mut date: NaiveDate) -> NaiveDate {
    while is_weekend(date) {
        date = date.succ_opt().unwrap();
    }
    date
}

fn is_weekend(date: NaiveDate) -> bool {
    matches!(date.weekday(), Weekday::Sat | Weekday::Sun)
}


#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[test]
    fn run_simulation_with_rng_uses_workdays_for_dates() {
        let throughput = vec![Throughput {
            date: NaiveDate::from_ymd_opt(2026, 1, 30).unwrap(),
            completed_issues: 1,
        }];
        let start_date = NaiveDate::from_ymd_opt(2026, 1, 30).unwrap();
        let mut rng = StdRng::seed_from_u64(42);
        let simulation = run_simulation_with_rng(&throughput, 3, 2, start_date, &mut rng).unwrap();

        assert_eq!(simulation.results, vec![2.0, 2.0, 2.0]);
        assert_eq!(simulation.report.p0.days, 2.0);
        assert_eq!(simulation.report.p100.days, 2.0);
        assert_eq!(simulation.report.p50.days, 2.0);
        assert_eq!(simulation.report.p85.days, 2.0);
        assert_eq!(simulation.report.p0.date, "2026-02-02");
        assert_eq!(simulation.report.p100.date, "2026-02-02");
    }
}
