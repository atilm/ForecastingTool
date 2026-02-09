use std::io::{self, Write};
use crate::domain::throughput::Throughput;
use serde::Serialize;

#[derive(Serialize)]
struct ThroughputRecord {
    date: String,
    completed_issues: usize,
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
}
