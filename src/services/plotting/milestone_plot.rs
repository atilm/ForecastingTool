use std::collections::BTreeMap;

use crate::services::project_simulation::simulation_types::{
    SimulationOutput, SimulationReport, WorkPackageSimulation,
};
use plotters::prelude::*;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MilestonePlotError {
    #[error("simulation report has no work package data")]
    NoWorkPackages,
    #[error("simulation report has no milestones")]
    NoMilestones,
    #[error("failed to render milestone plot: {0}")]
    Plot(String),
}

pub fn write_milestone_plot_png(
    output_path: &str,
    simulation: &SimulationOutput,
) -> Result<(), MilestonePlotError> {
    let mut milestones = collect_milestones(&simulation.report)?;
    milestones.sort_by(|a, b| {
        a.percentiles
            .p50
            .end_date
            .partial_cmp(&b.percentiles.p50.end_date)
            .unwrap()
    });
    render_milestone_plot_png(output_path, &milestones)
}

fn milestone_label_for_y(value: f32, milestones: &[&WorkPackageSimulation]) -> String {
    const HALF_STEP: f32 = 0.5;
    const EPSILON: f32 = 0.001;

    let idx_as_float = value - HALF_STEP;
    let rounded = idx_as_float.round();
    if (idx_as_float - rounded).abs() > EPSILON {
        return String::new();
    }

    let idx = rounded as isize;
    if idx < 0 || idx as usize >= milestones.len() {
        return String::new();
    }

    milestones[idx as usize].id.clone()
}

fn collect_milestones(
    report: &SimulationReport,
) -> Result<Vec<&WorkPackageSimulation>, MilestonePlotError> {
    let work_packages = report
        .work_packages
        .as_ref()
        .ok_or(MilestonePlotError::NoWorkPackages)?;

    let milestones: Vec<&WorkPackageSimulation> = work_packages
        .iter()
        .filter(|item| item.is_milestone)
        .collect();
    if milestones.is_empty() {
        return Err(MilestonePlotError::NoMilestones);
    }

    Ok(milestones)
}

fn render_milestone_plot_png(
    output_path: &str,
    milestones: &[&WorkPackageSimulation],
) -> Result<(), MilestonePlotError> {
    let max_days = milestones
        .iter()
        .map(|item| item.percentiles.p100.days)
        .fold(0.0_f32, f32::max);
    let x_max = if max_days <= 0.0 {
        1.0
    } else {
        max_days * 1.1 + 1.0
    };

    let max_y = milestones.len() as f32;

    let root = BitMapBackend::new(output_path, (1200, 700)).into_drawing_area();
    root.fill(&WHITE)
        .map_err(|e| MilestonePlotError::Plot(e.to_string()))?;

    let mut chart = ChartBuilder::on(&root)
        .margin(20)
        .caption("Milestone Forecast Box Plot", ("sans-serif", 30))
        .x_label_area_size(80)
        .y_label_area_size(140)
        .build_cartesian_2d(0.0_f32..x_max, 0.0_f32..max_y)
        .map_err(|e| MilestonePlotError::Plot(e.to_string()))?;

    chart
        .configure_mesh()
        .disable_mesh()
        .x_desc("Duration in days")
        .y_desc("Milestones")
        .label_style(("sans-serif", 16))
        .axis_desc_style(("sans-serif", 20))
        .y_labels(milestones.len().saturating_mul(2).saturating_add(1))
        .y_label_formatter(&|value| milestone_label_for_y(*value, milestones))
        .draw()
        .map_err(|e| MilestonePlotError::Plot(e.to_string()))?;

    let mut series = BTreeMap::new();
    for (idx, item) in milestones.iter().enumerate() {
        let y_coord = idx as f32 + 0.5;

        let points = vec![
            (item.percentiles.p0.days, y_coord),
            (item.percentiles.p15.days, y_coord),
            (item.percentiles.p50.days, y_coord),
            (item.percentiles.p85.days, y_coord),
            (item.percentiles.p100.days, y_coord),
        ];

        series.insert(item.id.clone(), points);
    }

    for (_id, points) in series {
        chart
            .draw_series(LineSeries::new(points.clone(), &BLUE.mix(0.5)))
            .map_err(|e| MilestonePlotError::Plot(e.to_string()))?;

        chart
            .draw_series(PointSeries::of_element(
                points,
                5,
                &BLUE,
                &|coord, size, style| EmptyElement::at(coord) + Circle::new((0, 0), size, style),
            ))
            .map_err(|e| MilestonePlotError::Plot(e.to_string()))?;
    }

    root.present()
        .map_err(|e| MilestonePlotError::Plot(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::project_simulation::simulation_types::{
        SimulationPercentile, SimulationReport, WorkPackagePercentiles,
    };
    use assert_fs::prelude::*;
    use chrono::NaiveDate;
    use predicates::prelude::*;

    fn percentile(days: f32, year: i32, month: u32, day: u32) -> SimulationPercentile {
        SimulationPercentile {
            days,
            end_date: NaiveDate::from_ymd_opt(year, month, day).unwrap(),
        }
    }

    fn build_output(work_packages: Option<Vec<WorkPackageSimulation>>) -> SimulationOutput {
        let start_date = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();
        let report = SimulationReport {
            data_source: "unit".to_string(),
            start_date,
            velocity: Some(3.0),
            iterations: 100,
            simulated_items: 4,
            p0: percentile(4.0, 2026, 3, 5),
            p15: percentile(6.0, 2026, 3, 7),
            p50: percentile(10.0, 2026, 3, 11),
            p85: percentile(13.0, 2026, 3, 14),
            p100: percentile(17.0, 2026, 3, 18),
            work_packages,
        };

        SimulationOutput {
            report,
            results: vec![4.0, 6.0, 10.0, 13.0, 17.0],
        }
    }

    fn build_wp(
        id: &str,
        is_milestone: bool,
        p0: f32,
        p15: f32,
        p50: f32,
        p85: f32,
        p100: f32,
    ) -> WorkPackageSimulation {
        WorkPackageSimulation {
            id: id.to_string(),
            is_milestone,
            percentiles: WorkPackagePercentiles {
                p0: percentile(p0, 2026, 3, 1),
                p15: percentile(p15, 2026, 3, 2),
                p50: percentile(p50, 2026, 3, 3),
                p85: percentile(p85, 2026, 3, 4),
                p100: percentile(p100, 2026, 3, 5),
            },
        }
    }

    #[test]
    fn write_milestone_plot_png_writes_png() {
        let output_file = assert_fs::NamedTempFile::new("milestones.png").unwrap();
        let simulation = build_output(Some(vec![
            build_wp("M1", true, 2.0, 3.0, 5.0, 6.0, 8.0),
            build_wp("TASK", false, 1.0, 2.0, 4.0, 5.0, 7.0),
            build_wp("M2", true, 3.0, 5.0, 6.0, 9.0, 12.0),
        ]));

        write_milestone_plot_png(output_file.path().to_str().unwrap(), &simulation).unwrap();

        output_file.assert(predicate::path::exists());
        let metadata = std::fs::metadata(output_file.path()).unwrap();
        assert!(metadata.len() > 0);
    }

    #[test]
    fn write_milestone_plot_png_rejects_missing_work_packages() {
        let output_file = assert_fs::NamedTempFile::new("missing-work-packages.png").unwrap();
        let simulation = build_output(None);

        let error = write_milestone_plot_png(output_file.path().to_str().unwrap(), &simulation)
            .unwrap_err();

        assert!(matches!(error, MilestonePlotError::NoWorkPackages));
    }

    #[test]
    fn write_milestone_plot_png_rejects_reports_without_milestones() {
        let output_file = assert_fs::NamedTempFile::new("no-milestones.png").unwrap();
        let simulation = build_output(Some(vec![build_wp(
            "TASK-1", false, 1.0, 2.0, 4.0, 5.0, 7.0,
        )]));

        let error = write_milestone_plot_png(output_file.path().to_str().unwrap(), &simulation)
            .unwrap_err();

        assert!(matches!(error, MilestonePlotError::NoMilestones));
    }

    #[test]
    fn milestone_label_for_y_labels_half_step_positions_only() {
        let milestone_1 = build_wp("M1", true, 1.0, 2.0, 3.0, 4.0, 5.0);
        let milestone_2 = build_wp("M2", true, 2.0, 3.0, 4.0, 5.0, 6.0);
        let milestones = vec![&milestone_1, &milestone_2];

        assert_eq!(milestone_label_for_y(0.5, &milestones), "M1");
        assert_eq!(milestone_label_for_y(1.5, &milestones), "M2");
        assert_eq!(milestone_label_for_y(1.0, &milestones), "");
        assert_eq!(milestone_label_for_y(2.5, &milestones), "");
    }
}
