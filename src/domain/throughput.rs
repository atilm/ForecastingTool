use chrono::NaiveDate;

pub struct Throughput {
    pub date: NaiveDate,
    pub completed_issues: usize,
}

