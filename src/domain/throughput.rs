use chrono::NaiveDate;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Throughput {
    pub date: NaiveDate,
    pub completed_issues: usize,
}

