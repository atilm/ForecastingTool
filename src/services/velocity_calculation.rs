use thiserror::Error;

use crate::domain::{
    calendar::TeamCalendar,
    issue::{Issue, IssueStatus},
    project::Project,
};

#[derive(Error, Debug)]
pub enum VelocityCalculationError {
    #[error("missing dates for velocity calculation")]
    MissingVelocityDates,
    #[error("no completed issues with story point estimates")]
    MissingVelocityData,
    #[error("invalid velocity duration")]
    InvalidVelocityDuration,
    #[error("invalid velocity value")]
    InvalidVelocityValue,
}

pub fn calculate_project_velocity(
    project: &Project,
    calendar: &TeamCalendar,
) -> Result<f32, VelocityCalculationError> {
    let mut completed: Vec<&Issue> = project
        .work_packages
        .iter()
        .filter(|issue| issue.status == Some(IssueStatus::Done))
        .filter(|issue| issue.story_point_value().is_some())
        .filter(|issue| issue.start_date.is_some() && issue.done_date.is_some())
        .collect();

    if completed.is_empty() {
        return Err(VelocityCalculationError::MissingVelocityData);
    }

    completed.sort_by_key(|issue| issue.done_date);
    let selected = if completed.len() > 30 {
        &completed[completed.len() - 30..]
    } else {
        completed.as_slice()
    };

    let first = selected
        .first()
        .ok_or(VelocityCalculationError::MissingVelocityData)?;
    let last = selected
        .last()
        .ok_or(VelocityCalculationError::MissingVelocityData)?;
    let start_date = first
        .start_date
        .ok_or(VelocityCalculationError::MissingVelocityDates)?;
    let end_date = last
        .done_date
        .ok_or(VelocityCalculationError::MissingVelocityDates)?;

    let summed_capacity = summed_capacity_in_period(calendar, start_date, end_date);
    if summed_capacity <= 0.0 {
        return Err(VelocityCalculationError::InvalidVelocityDuration);
    }

    let total_points: f32 = selected
        .iter()
        .filter_map(|issue| issue.story_point_value())
        .sum();

    let velocity = total_points / summed_capacity as f32;
    if velocity <= 0.0 {
        return Err(VelocityCalculationError::InvalidVelocityValue);
    }

    Ok(velocity)
}

fn summed_capacity_in_period(
    calendar: &TeamCalendar,
    start: chrono::NaiveDate,
    end: chrono::NaiveDate,
) -> f32 {
    let mut total_capacity = 0.0;
    let mut current_date = start;
    while current_date <= end {
        total_capacity += calendar.get_capacity(current_date);
        current_date += chrono::Duration::days(1);
    }
    total_capacity
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::calendar::{self, Calendar, TeamCalendar};
    use crate::test_support::build_done_issue;
    use chrono::NaiveDate;

    #[test]
    fn calculate_velocity_from_done_story_points() {
        let base = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let mut issues = Vec::new();
        for idx in 0..30 {
            let start = base + chrono::Duration::days(idx);
            let done = start + chrono::Duration::days(1);
            issues.push(build_done_issue(&format!("ABC-{idx}"), 2.0, start, done));
        }
        let project = Project {
            name: "Demo".to_string(),
            work_packages: issues,
        };
        let no_free_days_calendar = TeamCalendar {
            calendars: vec![Calendar {
                free_weekdays: vec![],
                free_date_ranges: vec![],
            }],
        };

        let velocity = calculate_project_velocity(&project, &no_free_days_calendar).unwrap();
        // The 30 issues span an inclusive period of 31 days (from first start_date to last done_date).
        assert!((velocity - 2.0 * 30.0 / 31.0).abs() < f32::EPSILON);
    }

    #[test]
    fn calculate_velocity_uses_last_thirty_issues() {
        let base = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let mut issues = Vec::new();
        for idx in 0..31 {
            let start = base + chrono::Duration::days(idx);
            let done = start + chrono::Duration::days(1);
            issues.push(build_done_issue(&format!("ABC-{idx}"), 1.0, start, done));
        }
        let project = Project {
            name: "Demo".to_string(),
            work_packages: issues,
        };
        let no_free_days_calendar = TeamCalendar {
            calendars: vec![Calendar {
                free_weekdays: vec![],
                free_date_ranges: vec![],
            }],
        };

        let velocity = calculate_project_velocity(&project, &no_free_days_calendar).unwrap();
        // The 30 selected issues span an inclusive period of 31 days.
        let expected = 30.0 / 31.0;
        assert!((velocity - expected).abs() < f32::EPSILON);
    }

    fn on_date(year: i32, month: u32, day: u32) -> chrono::NaiveDate {
        chrono::NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    #[test]
    fn calculate_velocity_takes_fee_days_into_account() {
        use chrono::Weekday;

        let issues = vec![
            // The period used for velocity calculation is from 2026-02-13 to 2026-02-23, which contains 2 weekends and 7 working days
            build_done_issue("ABC-0", 2.0, on_date(2026, 2, 13), on_date(2026, 2, 16)), // Mon
            build_done_issue("ABC-1", 2.0, on_date(2026, 2, 13), on_date(2026, 2, 17)), // Tue
            build_done_issue("ABC-2", 2.0, on_date(2026, 2, 13), on_date(2026, 2, 18)), // Wed
            build_done_issue("ABC-3", 2.0, on_date(2026, 2, 13), on_date(2026, 2, 19)), // Thu
            build_done_issue("ABC-4", 2.0, on_date(2026, 2, 13), on_date(2026, 2, 20)), // Fri
            build_done_issue("ABC-5", 2.0, on_date(2026, 2, 13), on_date(2026, 2, 23)), // Next Mon
        ];

        let half_capacity_calendar = TeamCalendar {
            calendars: vec![
                Calendar {
                    free_weekdays: vec![Weekday::Sat, Weekday::Sun],
                    free_date_ranges: vec![],
                },
                Calendar {
                    free_weekdays: vec![Weekday::Sat, Weekday::Sun],
                    free_date_ranges: vec![calendar::FreeDateRange {
                        start_date: on_date(2026, 2, 13),
                        end_date: on_date(2026, 2, 23),
                    }],
                },
            ],
        };

        let project = Project {
            name: "Demo".to_string(),
            work_packages: issues,
        };

        let velocity = calculate_project_velocity(&project, &half_capacity_calendar).unwrap();
        let expected = 12.0 / 7.0 * 2.0; // 12 points over 7 working days with half capacity is double the velocity compared to full capacity
        assert!((velocity - expected).abs() < f32::EPSILON);
    }
}
