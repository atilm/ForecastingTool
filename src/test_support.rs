use chrono::NaiveDate;

use crate::domain::estimate::{Estimate, StoryPointEstimate, ThreePointEstimate};
use crate::domain::issue::{Issue, IssueId, IssueStatus};
use crate::services::beta_pert_sampler::ThreePointSampler;

// A mock ThreePointSampler that always returns the most likely value
pub struct MockSampler;
impl ThreePointSampler for MockSampler {
    fn sample(&mut self, _optimistic: f32, most_likely: f32, _pessimistic: f32) -> Result<f32, ()> {
        Ok(most_likely)
    }
}

pub fn on_date(year: i32, month: u32, day: u32) -> chrono::NaiveDate {
    chrono::NaiveDate::from_ymd_opt(year, month, day).unwrap()
}

pub fn create_calendar_without_any_free_days() -> crate::domain::calendar::TeamCalendar {
    crate::domain::calendar::TeamCalendar {
        calendars: vec![crate::domain::calendar::Calendar {
            free_weekdays: vec![],
            free_date_ranges: vec![],
        }],
    }
}

pub fn build_done_issue(id: &str, points: f32, start: NaiveDate, done: NaiveDate) -> Issue {
    let mut issue = Issue::new();
    issue.issue_id = Some(IssueId { id: id.to_string() });
    issue.status = Some(IssueStatus::Done);
    issue.start_date = Some(start);
    issue.done_date = Some(done);
    issue.estimate = Some(Estimate::StoryPoint(StoryPointEstimate {
        estimate: Some(points),
    }));
    issue
}

pub fn build_three_point_issue(id: &str, days: f32, deps: &[&str]) -> Issue {
    let mut issue = Issue::new();
    issue.issue_id = Some(IssueId { id: id.to_string() });
    issue.estimate = Some(Estimate::ThreePoint(ThreePointEstimate {
        optimistic: Some(days),
        most_likely: Some(days),
        pessimistic: Some(days),
    }));
    issue.dependencies = if deps.is_empty() {
        None
    } else {
        Some(
            deps.iter()
                .map(|dep| IssueId {
                    id: (*dep).to_string(),
                })
                .collect(),
        )
    };
    issue
}

pub fn build_story_point_issue(id: &str, points: f32, deps: &[&str]) -> Issue {
    let mut issue = Issue::new();
    issue.issue_id = Some(IssueId { id: id.to_string() });
    issue.status = Some(IssueStatus::ToDo);
    issue.estimate = Some(Estimate::StoryPoint(StoryPointEstimate {
        estimate: Some(points),
    }));
    issue.dependencies = if deps.is_empty() {
        None
    } else {
        Some(
            deps.iter()
                .map(|dep| IssueId {
                    id: (*dep).to_string(),
                })
                .collect(),
        )
    };
    issue
}
