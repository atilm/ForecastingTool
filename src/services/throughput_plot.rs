use crate::domain::throughput::Throughput;
use crate::services::throughput_yaml::{deserialize_throughput_from_yaml_str, ThroughputYamlError};
use plotters::prelude::*;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ThroughputPlotError {
    #[error("failed to read throughput file: {0}")]
    ReadThroughput(#[from] std::io::Error),
    #[error("failed to parse throughput yaml: {0}")]
    ParseThroughput(#[from] ThroughputYamlError),
    #[error("throughput data is empty")]
    EmptyThroughput,
    #[error("failed to render throughput plot: {0}")]
    Plot(String),
}

pub async fn plot_throughput_from_yaml_file(
    input_path: &str,
    output_path: &str,
) -> Result<(), ThroughputPlotError> {
    let throughput_yaml = tokio::fs::read_to_string(input_path).await?;
    let throughput = deserialize_throughput_from_yaml_str(&throughput_yaml)?;
    if throughput.is_empty() {
        return Err(ThroughputPlotError::EmptyThroughput);
    }
    write_plot_png(output_path, &throughput).await?;
    Ok(())
}

async fn write_plot_png(
    output_path: &str,
    throughput: &[Throughput],
) -> Result<(), ThroughputPlotError> {
    let output_path = output_path.to_string();
    let throughput = throughput.to_vec();
    tokio::task::spawn_blocking(move || render_plot_png(&output_path, &throughput))
        .await
        .map_err(|e| ThroughputPlotError::Plot(e.to_string()))??;
    Ok(())
}

fn render_plot_png(
    output_path: &str,
    throughput: &[Throughput],
) -> Result<(), ThroughputPlotError> {
    if throughput.is_empty() {
        return Ok(());
    }

    let max_completed = throughput
        .iter()
        .map(|item| item.completed_issues)
        .max()
        .unwrap_or(0);
    let max_y = max_completed.saturating_add(1).max(1) as i32;
    let max_x = throughput.len().max(1) as i32;

    let root = BitMapBackend::new(output_path, (900, 600)).into_drawing_area();
    root.fill(&WHITE)
        .map_err(|e| ThroughputPlotError::Plot(e.to_string()))?;

    let mut chart = ChartBuilder::on(&root)
        .margin(20)
        .caption("Throughput Over Time", ("sans-serif", 30))
        .x_label_area_size(55)
        .y_label_area_size(65)
        .build_cartesian_2d(0..max_x, 0..max_y)
        .map_err(|e| ThroughputPlotError::Plot(e.to_string()))?;

    let label_count = throughput.len().min(10).max(1);
    chart
        .configure_mesh()
        .disable_mesh()
        .x_desc("Date")
        .y_desc("Completed issues")
        .label_style(("sans-serif", 18))
        .axis_desc_style(("sans-serif", 22))
        .x_labels(label_count)
        .x_label_formatter(&|index| {
            if *index < 0 {
                return String::new();
            }
            let idx = *index as usize;
            throughput
                .get(idx)
                .map(|item| item.date.format("%Y-%m-%d").to_string())
                .unwrap_or_default()
        })
        .draw()
        .map_err(|e| ThroughputPlotError::Plot(e.to_string()))?;

    let bar_color = RGBColor(30, 122, 204);
    let bar_style = ShapeStyle::from(&bar_color).filled().stroke_width(1);
    chart
        .draw_series(throughput.iter().enumerate().map(|(idx, item)| {
            Rectangle::new(
                [(idx as i32, 0), (idx as i32 + 1, item.completed_issues as i32)],
                bar_style.clone(),
            )
        }))
        .map_err(|e| ThroughputPlotError::Plot(e.to_string()))?;

    root.present()
        .map_err(|e| ThroughputPlotError::Plot(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::prelude::*;
    use predicates::prelude::*;

    #[tokio::test]
    async fn plot_throughput_from_yaml_file_writes_png() {
        let throughput_yaml = "- date: 2026-01-26\n  completed_issues: 2\n- date: 2026-01-27\n  completed_issues: 0\n- date: 2026-01-28\n  completed_issues: 3\n";

        let input_file = assert_fs::NamedTempFile::new("throughput.yaml").unwrap();
        input_file.write_str(throughput_yaml).unwrap();
        let output_file = assert_fs::NamedTempFile::new("throughput.png").unwrap();

        plot_throughput_from_yaml_file(
            input_file.path().to_str().unwrap(),
            output_file.path().to_str().unwrap(),
        )
        .await
        .unwrap();

        output_file.assert(predicate::path::exists());
        let metadata = std::fs::metadata(output_file.path()).unwrap();
        assert!(metadata.len() > 0);
    }

    #[tokio::test]
    async fn plot_throughput_from_yaml_file_rejects_empty_data() {
        let input_file = assert_fs::NamedTempFile::new("empty.yaml").unwrap();
        input_file.write_str("[]").unwrap();
        let output_file = assert_fs::NamedTempFile::new("empty.png").unwrap();

        let error = plot_throughput_from_yaml_file(
            input_file.path().to_str().unwrap(),
            output_file.path().to_str().unwrap(),
        )
        .await
        .expect_err("expected empty throughput error");

        assert!(matches!(error, ThroughputPlotError::EmptyThroughput));
    }
}
