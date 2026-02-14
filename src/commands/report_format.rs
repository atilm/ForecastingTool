use crate::services::simulation_types::SimulationReport;

pub fn format_simulation_report(report: &SimulationReport) -> String {
    let velocity = match report.velocity {
        Some(value) => format!("{value:.2}"),
        None => "n/a".to_string(),
    };

    let mut lines = Vec::new();
    lines.push("Simulation Report".to_string());
    lines.push(format!("Data source: {}", report.data_source));
    lines.push(format!("Start date: {}", report.start_date));
    lines.push(format!("Iterations: {}", report.iterations));
    lines.push(format!("Simulated items: {}", report.simulated_items));
    lines.push(format!("Velocity: {}", velocity));
    lines.push(String::new());
    lines.push("Percentiles:".to_string());
    lines.push("Percentile | Days | Date".to_string());
    lines.push("-----------|------|-----".to_string());
    lines.push(format_percentile_row("P0", &report.p0));
    lines.push(format_percentile_row("P50", &report.p50));
    lines.push(format_percentile_row("P85", &report.p85));
    lines.push(format_percentile_row("P100", &report.p100));

    lines.join("\n")
}

fn format_percentile_row(label: &str, percentile: &crate::services::simulation_types::SimulationPercentile) -> String {
    format!(
        "{label} | {days} | {date}",
        label = label,
        days = format!("{:.2}", percentile.days),
        date = percentile.date
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::simulation_types::SimulationPercentile;

    fn build_report() -> SimulationReport {
        SimulationReport {
            data_source: "input.yaml".to_string(),
            start_date: "2026-02-01".to_string(),
            velocity: Some(2.5),
            iterations: 100,
            simulated_items: 12,
            p0: SimulationPercentile {
                days: 1.0,
                date: "2026-02-02".to_string(),
            },
            p50: SimulationPercentile {
                days: 5.5,
                date: "2026-02-06".to_string(),
            },
            p85: SimulationPercentile {
                days: 10.0,
                date: "2026-02-11".to_string(),
            },
            p100: SimulationPercentile {
                days: 15.25,
                date: "2026-02-16".to_string(),
            },
        }
    }

    #[test]
    fn format_simulation_report_includes_header_and_table() {
        let report = build_report();
        let output = format_simulation_report(&report);

        assert!(output.contains("Simulation Report"));
        assert!(output.contains("Data source: input.yaml"));
        assert!(output.contains("Start date: 2026-02-01"));
        assert!(output.contains("Iterations: 100"));
        assert!(output.contains("Simulated items: 12"));
        assert!(output.contains("Velocity: 2.50"));
        assert!(output.contains("Percentile | Days | Date"));
        assert!(output.contains("P0 | 1.00 | 2026-02-02"));
        assert!(output.contains("P50 | 5.50 | 2026-02-06"));
        assert!(output.contains("P85 | 10.00 | 2026-02-11"));
        assert!(output.contains("P100 | 15.25 | 2026-02-16"));
    }

    #[test]
    fn format_simulation_report_uses_na_for_missing_velocity() {
        let mut report = build_report();
        report.velocity = None;

        let output = format_simulation_report(&report);
        assert!(output.contains("Velocity: n/a"));
    }
}
