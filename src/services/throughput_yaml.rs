use std::io::{self, Write};
use crate::domain::throughput::Throughput;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Serialize)]
struct ThroughputRecord {
    date: String,
    completed_issues: usize,
}

#[derive(Deserialize)]
struct ThroughputRecordInput {
    date: String,
    completed_issues: usize,
}

#[derive(Error, Debug)]
pub enum ThroughputYamlError {
    #[error("failed to parse yaml: {0}")]
    Parse(#[from] serde_yaml::Error),
    #[error("invalid date format: {0}")]
    InvalidDate(String),
}

pub fn serialize_throughput_to_yaml<W: Write>(writer: &mut W, data: &[Throughput]) -> io::Result<()> {
    let records: Vec<ThroughputRecord> = data
        .iter()
        .map(|t| ThroughputRecord {
            date: t.date.format("%Y-%m-%d").to_string(),
            completed_issues: t.completed_issues,
        })
        .collect();

    let yaml = serde_yaml::to_string(&records)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    writer.write_all(yaml.as_bytes())
}

pub fn deserialize_throughput_from_yaml_str(input: &str) -> Result<Vec<Throughput>, ThroughputYamlError> {
    let records: Vec<ThroughputRecordInput> = serde_yaml::from_str(input)?;
    let mut result = Vec::with_capacity(records.len());
    for record in records {
        let date = chrono::NaiveDate::parse_from_str(&record.date, "%Y-%m-%d")
            .map_err(|_| ThroughputYamlError::InvalidDate(record.date.clone()))?;
        result.push(Throughput {
            date,
            completed_issues: record.completed_issues,
        });
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_serialize_throughput_to_yaml() {
        let data = vec![
            Throughput {
                date: NaiveDate::from_ymd_opt(2026, 2, 9).unwrap(),
                completed_issues: 5,
            },
            Throughput {
                date: NaiveDate::from_ymd_opt(2026, 2, 10).unwrap(),
                completed_issues: 3,
            },
        ];
        let mut buf = Vec::new();
        serialize_throughput_to_yaml(&mut buf, &data).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("2026-02-09"));
        assert!(output.contains("completed_issues: 5"));
        assert!(output.contains("2026-02-10"));
        assert!(output.contains("completed_issues: 3"));
    }

    #[test]
    fn test_deserialize_throughput_from_yaml_str() {
        let yaml = r#"- date: 2026-02-09
  completed_issues: 5
- date: 2026-02-10
  completed_issues: 3
"#;
        let result = deserialize_throughput_from_yaml_str(yaml).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].date, NaiveDate::from_ymd_opt(2026, 2, 9).unwrap());
        assert_eq!(result[0].completed_issues, 5);
        assert_eq!(result[1].date, NaiveDate::from_ymd_opt(2026, 2, 10).unwrap());
        assert_eq!(result[1].completed_issues, 3);
    }
}
