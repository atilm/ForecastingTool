use std::io::{self, Write};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::services::util::histogram::Histogram;

#[derive(Serialize, Deserialize)]
struct HistogramRecord {
    min_value: f32,
    max_value: f32,
    bin_width: f32,
    bins: Vec<i32>,
}

#[derive(Error, Debug)]
pub enum HistogramYamlError {
    #[error("failed to read histogram yaml file: {0}")]
    Read(#[from] io::Error),
    #[error("failed to parse histogram yaml: {0}")]
    Parse(#[from] serde_yaml::Error),
}

pub fn serialize_histogram_to_yaml_file(
    path: &str,
    histogram: &Histogram,
) -> Result<(), HistogramYamlError> {
    let mut file = std::fs::File::create(path)?;
    serialize_histogram_to_yaml(&mut file, histogram)?;
    Ok(())
}

pub fn deserialize_histogram_from_yaml_file(path: &str) -> Result<Histogram, HistogramYamlError> {
    let contents = std::fs::read_to_string(path)?;
    deserialize_histogram_from_yaml_str(&contents)
}

pub fn serialize_histogram_to_yaml<W: Write>(
    writer: &mut W,
    histogram: &Histogram,
) -> Result<(), HistogramYamlError> {
    let record = HistogramRecord {
        min_value: histogram.min_value,
        max_value: histogram.max_value,
        bin_width: histogram.bin_width,
        bins: histogram.bins.clone(),
    };

    let yaml = serde_yaml::to_string(&record)?;
    writer.write_all(yaml.as_bytes())?;
    Ok(())
}

pub fn deserialize_histogram_from_yaml_str(input: &str) -> Result<Histogram, HistogramYamlError> {
    let record: HistogramRecord = serde_yaml::from_str(input)?;
    Ok(Histogram {
        min_value: record.min_value,
        max_value: record.max_value,
        bin_width: record.bin_width,
        bins: record.bins,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use assert_fs::prelude::*;

    fn assert_f32_eq(actual: f32, expected: f32) {
        assert!((actual - expected).abs() < 1e-6);
    }

    #[test]
    fn serializes_histogram_to_yaml_file() {
        let temp = assert_fs::TempDir::new().unwrap();
        let file = temp.child("histogram.yaml");

        let histogram = Histogram {
            min_value: 1.0,
            max_value: 9.0,
            bin_width: 4.0,
            bins: vec![2, 1, 3],
        };

        serialize_histogram_to_yaml_file(file.path().to_str().unwrap(), &histogram).unwrap();

        let contents = std::fs::read_to_string(file.path()).unwrap();
        assert!(contents.contains("min_value: 1.0"));
        assert!(contents.contains("max_value: 9.0"));
        assert!(contents.contains("bin_width: 4.0"));
        assert!(contents.contains("- 2"));
        assert!(contents.contains("- 1"));
        assert!(contents.contains("- 3"));
    }

    #[test]
    fn deserializes_histogram_from_yaml_file() {
        let temp = assert_fs::TempDir::new().unwrap();
        let file = temp.child("histogram.yaml");
        file.write_str(
            r#"min_value: 2.0
max_value: 12.0
bin_width: 5.0
bins:
  - 4
  - 0
  - 7
"#,
        )
        .unwrap();

        let histogram = deserialize_histogram_from_yaml_file(file.path().to_str().unwrap()).unwrap();

        assert_f32_eq(histogram.min_value, 2.0);
        assert_f32_eq(histogram.max_value, 12.0);
        assert_f32_eq(histogram.bin_width, 5.0);
        assert_eq!(histogram.bins, vec![4, 0, 7]);
    }

    #[test]
    fn serializes_and_deserializes_histogram_round_trip() {
        let temp = assert_fs::TempDir::new().unwrap();
        let file = temp.child("histogram.yaml");

        let original = Histogram {
            min_value: -3.5,
            max_value: 8.5,
            bin_width: 3.0,
            bins: vec![1, 4, 2, 5],
        };

        serialize_histogram_to_yaml_file(file.path().to_str().unwrap(), &original).unwrap();
        let deserialized = deserialize_histogram_from_yaml_file(file.path().to_str().unwrap()).unwrap();

        assert_f32_eq(deserialized.min_value, original.min_value);
        assert_f32_eq(deserialized.max_value, original.max_value);
        assert_f32_eq(deserialized.bin_width, original.bin_width);
        assert_eq!(deserialized.bins, original.bins);
    }
}