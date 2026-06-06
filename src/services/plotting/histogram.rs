use plotters::prelude::*;
use thiserror::Error;

use crate::services::util::histogram::Histogram;

#[derive(Error, Debug)]
pub enum HistogramError {
    #[error("failed to render histogram: {0}")]
    Render(String),
}

pub fn write_histogram_png(output_path: &str, histogram: &Histogram) -> Result<(), HistogramError> {
    render_histogram_png(output_path, histogram)
}

fn render_histogram_png(output_path: &str, histogram: &Histogram) -> Result<(), HistogramError> {
    let buckets: Vec<_> = histogram.iter().collect();
    if buckets.is_empty() {
        return Ok(());
    }

    let bucket_count = buckets.len() as i32;
    let max_count = buckets.iter().map(|bucket| bucket.count).max().unwrap_or(1);

    let root = BitMapBackend::new(output_path, (800, 600)).into_drawing_area();
    root.fill(&WHITE)
        .map_err(|e| HistogramError::Render(e.to_string()))?;

    let mut chart = ChartBuilder::on(&root)
        .margin(20)
        .caption("Simulation Results", ("sans-serif", 30))
        .x_label_area_size(55)
        .y_label_area_size(65)
        .build_cartesian_2d(0..bucket_count, 0..(max_count + 1))
        .map_err(|e| HistogramError::Render(e.to_string()))?;

    chart
        .configure_mesh()
        .disable_mesh()
        .x_desc("Duration in days")
        .y_desc("Frequency")
        .label_style(("sans-serif", 18))
        .axis_desc_style(("sans-serif", 22))
        .x_label_formatter(&|value| {
            if *value < 0 {
                return String::new();
            }

            let index = (*value as usize).min(buckets.len() - 1);
            format!("{:.2}", buckets[index].lower_bound)
        })
        .draw()
        .map_err(|e| HistogramError::Render(e.to_string()))?;

    let bar_color = RGBColor(30, 122, 204);
    let bar_style = ShapeStyle::from(&bar_color).filled();
    chart
        .draw_series(buckets.iter().enumerate().map(|(index, bucket)| {
            Rectangle::new(
                [((index as i32), 0), ((index as i32) + 1, bucket.count)],
                bar_style,
            )
        }))
        .map_err(|e| HistogramError::Render(e.to_string()))?;

    root.present()
        .map_err(|e| HistogramError::Render(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use assert_fs::prelude::*;

    #[test]
    fn write_histogram_png_writes_image_file_for_histogram() {
        let temp = assert_fs::TempDir::new().unwrap();
        let file = temp.child("histogram.png");
        let histogram = Histogram::create(&[1.0, 2.0, 3.0, 4.0]).unwrap();

        write_histogram_png(file.path().to_str().unwrap(), &histogram).unwrap();

        assert!(file.path().exists());
    }
}
