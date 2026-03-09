use chrono::{Duration, NaiveDate};
use plotters::coord::types::{RangedCoordf32, RangedCoordi32};
use plotters::prelude::*;

use crate::services::burndown_plot::{BurndownData, BurndownPlotError, ChartPoint};

pub(super) fn render_burndown_plot_png(
    output_path: &str,
    data: &BurndownData,
) -> Result<(), BurndownPlotError> {
    let day_span = (data.end_date - data.start_date).num_days().max(0) as i32;
    let x_max = day_span.max(1);
    let y_max = data.total_points.max(1.0);

    let root = BitMapBackend::new(output_path, (1200, 700)).into_drawing_area();
    root.fill(&WHITE)
        .map_err(|e| BurndownPlotError::Plot(e.to_string()))?;

    let mut chart = ChartBuilder::on(&root)
        .margin(20)
        .caption("Burndown Forecast", ("sans-serif", 30))
        .x_label_area_size(60)
        .y_label_area_size(80)
        .build_cartesian_2d(0..(x_max + 1), 0.0_f32..y_max)
        .map_err(|e| BurndownPlotError::Plot(e.to_string()))?;

    chart
        .configure_mesh()
        .x_desc("Date")
        .y_desc("Remaining story points")
        .x_labels((x_max as usize).min(10).max(1))
        .x_label_formatter(&|x| {
            (data.start_date + Duration::days(*x as i64))
                .format("%Y-%m-%d")
                .to_string()
        })
        .draw()
        .map_err(|e| BurndownPlotError::Plot(e.to_string()))?;

    let forecast_band_color = RGBColor(140, 190, 255);

    draw_forecast_band(&mut chart, data, forecast_band_color)?;
    draw_points(
        &mut chart,
        &data.done_points,
        data.start_date,
        RGBColor(130, 130, 130),
        5
    )?;
    draw_points(&mut chart, &data.p15_points, data.start_date, forecast_band_color, 2)?;
    draw_points(&mut chart, &data.p50_points, data.start_date, BLUE, 5)?;
    draw_points(&mut chart, &data.p85_points, data.start_date, forecast_band_color, 2)?;

    root.present()
        .map_err(|e| BurndownPlotError::Plot(e.to_string()))?;
    Ok(())
}

fn draw_forecast_band(
    chart: &mut ChartContext<BitMapBackend<'_>, Cartesian2d<RangedCoordi32, RangedCoordf32>>,
    data: &BurndownData,
    color: RGBColor
) -> Result<(), BurndownPlotError> {
    if data.p15_points.is_empty() || data.p85_points.is_empty() {
        return Ok(());
    }

    let mut polygon = data
        .p15_points
        .iter()
        .map(|point| {
            (
                (point.date - data.start_date).num_days() as i32,
                point.remaining,
            )
        })
        .collect::<Vec<_>>();
    polygon.extend(data.p85_points.iter().rev().map(|point| {
        (
            (point.date - data.start_date).num_days() as i32,
            point.remaining,
        )
    }));

    chart
        .draw_series(std::iter::once(Polygon::new(
            polygon,
            color.mix(0.3).filled(),
        )))
        .map_err(|e| BurndownPlotError::Plot(e.to_string()))?;
    Ok(())
}

fn draw_points(
    chart: &mut ChartContext<BitMapBackend<'_>, Cartesian2d<RangedCoordi32, RangedCoordf32>>,
    points: &[ChartPoint],
    start_date: NaiveDate,
    color: RGBColor,
    point_size: i32,
) -> Result<(), BurndownPlotError> {
    let coords = points
        .iter()
        .map(|point| ((point.date - start_date).num_days() as i32, point.remaining));
    chart
        .draw_series(PointSeries::of_element(
            coords,
            point_size,
            color.filled(),
            &|coord, size, style| EmptyElement::at(coord) + Circle::new((0, 0), size, style),
        ))
        .map_err(|e| BurndownPlotError::Plot(e.to_string()))?;
    Ok(())
}
