use std::collections::BTreeMap;

use crate::domain::throughput::Throughput;
use crate::services::data_source::{DataQuery, DataSource, DataSourceError};
use chrono::{Datelike, NaiveDate};

pub struct DataConverter {
    data_source: Box<dyn DataSource>,
}

impl DataConverter {
    pub fn new(data_source: Box<dyn DataSource>) -> Self {
        Self { data_source }
    }

    pub async fn get_throughput_data(
        &self,
        data_query: DataQuery,
    ) -> Result<Vec<Throughput>, DataSourceError> {
        let issues = self.data_source.get_issues(data_query).await?;

        let done_dates: Vec<NaiveDate> =
            issues.iter().filter_map(|issue| issue.done_date).collect();
        let min_date = *done_dates.iter().min().ok_or(DataSourceError::NotFound)?;
        let max_date = *done_dates.iter().max().ok_or(DataSourceError::NotFound)?;

        let mut date_counts: BTreeMap<NaiveDate, usize> = BTreeMap::new();
        for date in done_dates {
            *date_counts.entry(date).or_insert(0) += 1;
        }

        fn is_weekend(date: NaiveDate) -> bool {
            matches!(date.weekday(), chrono::Weekday::Sat | chrono::Weekday::Sun)
        }

        let mut throughput_data = Vec::new();
        for date in min_date.iter_days().take_while(|&d| d <= max_date) {
            if is_weekend(date) {
                continue;
            }

            throughput_data.push(Throughput {
                date,
                completed_issues: *date_counts.get(&date).unwrap_or(&0),
            });
        }

        Ok(throughput_data)
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;
    use crate::domain::issue::Issue;
    use crate::services::data_source::{DataQuery, DataSourceError};

    struct MockDataSource {
        issues: Vec<Issue>,
    }

    #[async_trait::async_trait]
    impl DataSource for MockDataSource {
        async fn get_epic(
            &self,
            _epic_id: &str,
        ) -> Result<crate::domain::epic::Epic, DataSourceError> {
            Err(DataSourceError::Other("not used".to_string()))
        }

        async fn get_issues(&self, _query: DataQuery) -> Result<Vec<Issue>, DataSourceError> {
            Ok(self.issues.clone())
        }
    }

    #[tokio::test]
    async fn construct_throughput_from_issue_vector() {
        // Arrange
        let done_dates = vec![
            NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(), // Thursday
            NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2026, 1, 5).unwrap(), // Monday
            NaiveDate::from_ymd_opt(2026, 1, 5).unwrap(),
            NaiveDate::from_ymd_opt(2026, 1, 5).unwrap(),
            NaiveDate::from_ymd_opt(2026, 1, 7).unwrap(), // Wednesday
        ];

        let done_issues = done_dates
            .iter()
            .map(|&date| {
                let mut issue = Issue::new();
                issue.done_date = Some(date);
                issue
            })
            .collect();

        let data_source = Box::new(MockDataSource {
            issues: done_issues,
        });

        // Act
        let converter: DataConverter = DataConverter::new(data_source);
        let result = converter
            .get_throughput_data(DataQuery::StringQuery("dummy string".to_string()))
            .await
            .unwrap();

        // Assert
        let expected_throughput_data = vec![
            (NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(), 2), // Thursday
            (NaiveDate::from_ymd_opt(2026, 1, 2).unwrap(), 0),
            // Weekend
            (NaiveDate::from_ymd_opt(2026, 1, 5).unwrap(), 3), // Monday
            (NaiveDate::from_ymd_opt(2026, 1, 6).unwrap(), 0),
            (NaiveDate::from_ymd_opt(2026, 1, 7).unwrap(), 1),
        ];

        let expected_throughput = expected_throughput_data
            .into_iter()
            .map(|(date, completed_issues)| Throughput {
                date,
                completed_issues,
            })
            .collect::<Vec<_>>();
        assert_eq!(result, expected_throughput);
    }
}
