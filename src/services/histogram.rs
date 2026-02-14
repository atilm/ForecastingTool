use plotters::prelude::*;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum HistogramError {
    #[error("failed to render histogram: {0}")]
    Render(String),
}

pub fn write_histogram_png(output_path: &str, results: &[f32]) -> Result<(), HistogramError> {
    render_histogram_png(output_path, results)
}

fn render_histogram_png(output_path: &str, results: &[f32]) -> Result<(), HistogramError> {
    if results.is_empty() {
        return Ok(());
    }

    let min_value = results
        .iter()
        .cloned()
        .fold(f32::INFINITY, f32::min);
    let max_value = results
    .iter()
    .cloned()
    .fold(f32::NEG_INFINITY, f32::max);

    let range = max_value - min_value;
    let square_root_of_n = (results.len() as f32).sqrt();
    let bin_width: f32 = range / square_root_of_n;

    let mut counts: std::collections::BTreeMap<i32, usize> = std::collections::BTreeMap::new();
    for value in results {
        let bucket = (*value / bin_width).round() as i32;
        *counts.entry(bucket).or_insert(0usize) += 1;
    }
    let max_count = *counts.values().max().unwrap_or(&1);

    let root = BitMapBackend::new(output_path, (800, 600)).into_drawing_area();
    root.fill(&WHITE)
        .map_err(|e| HistogramError::Render(e.to_string()))?;

    let min_bucket = (*counts.keys().next().unwrap_or(&0)) - 1;
    let max_bucket = (*counts.keys().next_back().unwrap_or(&0)) + 1;
    let max_x = if max_value - min_value < f32::EPSILON {
        min_bucket + 1
    } else {
        max_bucket
    };
    let mut chart = ChartBuilder::on(&root)
        .margin(20)
        .caption("Simulation Results", ("sans-serif", 30))
        .x_label_area_size(55)
        .y_label_area_size(65)
        .build_cartesian_2d(min_bucket..max_x, 0..(max_count + 1))
        .map_err(|e| HistogramError::Render(e.to_string()))?;

    chart
        .configure_mesh()
        .disable_mesh()
        .x_desc("Duration in days")
        .y_desc("Frequency")
        .label_style(("sans-serif", 18))
        .axis_desc_style(("sans-serif", 22))
        .x_label_formatter(&|value| format!("{:.2}", *value as f32 * bin_width))
        .draw()
        .map_err(|e| HistogramError::Render(e.to_string()))?;

    let bar_color = RGBColor(30, 122, 204);
    let bar_style = ShapeStyle::from(&bar_color).filled();
    chart
        .draw_series(counts.iter().map(|(value, count)| {
            Rectangle::new([(*value, 0), (*value + 1, *count)], bar_style)
        }))
        .map_err(|e| HistogramError::Render(e.to_string()))?;

    root.present()
        .map_err(|e| HistogramError::Render(e.to_string()))?;
    Ok(())
}
