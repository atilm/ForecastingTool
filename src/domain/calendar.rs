use chrono::Datelike;
use chrono::NaiveDate;
use chrono::Weekday;

#[derive(Debug, Clone)]
pub struct FreeDateRange {
    pub start_date: chrono::NaiveDate,
    pub end_date: chrono::NaiveDate,
}

#[derive(Debug, Clone)]
pub struct Calendar {
    pub free_weekdays: Vec<Weekday>,
    pub free_date_ranges: Vec<FreeDateRange>,
}

impl Calendar {
    pub fn new() -> Self {
        Self {
            free_weekdays: Vec::new(),
            free_date_ranges: Vec::new(),
        }
    }

    pub fn get_capacity(&self, date: NaiveDate) -> f32 {
        if self.free_weekdays.contains(&date.weekday()) {
            return 0.0;
        }

        for free_date_range in &self.free_date_ranges {
            if date >= free_date_range.start_date && date <= free_date_range.end_date {
                return 0.0;
            }
        }

        1.0
    }
}

#[derive(Debug, Clone)]
pub struct TeamCalendar {
    pub calendars: Vec<Calendar>,
}

impl TeamCalendar {
    pub fn new() -> Self {
        Self {
            calendars: Vec::new(),
        }
    }

    pub fn get_capacity(&self, date: NaiveDate) -> f32 {
        if self.calendars.is_empty() {
            return self.get_default_capacity(date);
        }

        let capacity_sum: f32 = self.calendars.iter().map(|c| c.get_capacity(date)).sum();
        let max_capacity = self.calendars.len() as f32;
        
        capacity_sum / max_capacity
    }

    pub fn get_default_capacity(&self, date: NaiveDate) -> f32 {
        if date.weekday() == Weekday::Sat || date.weekday() == Weekday::Sun {
            return 0.0;
        }

        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_default_team_calendar_has_capacity_1_on_weekdays_and_0_on_weekends() {
        let test_cases = vec![
            (NaiveDate::from_ymd_opt(2026, 2, 16).unwrap(), 1.0), // Monday
            (NaiveDate::from_ymd_opt(2026, 2, 17).unwrap(), 1.0), // Tuesday
            (NaiveDate::from_ymd_opt(2026, 2, 18).unwrap(), 1.0), // Wednesday
            (NaiveDate::from_ymd_opt(2026, 2, 19).unwrap(), 1.0), // Thursday
            (NaiveDate::from_ymd_opt(2026, 2, 20).unwrap(), 1.0), // Friday
            (NaiveDate::from_ymd_opt(2026, 2, 21).unwrap(), 0.0), // Saturday
            (NaiveDate::from_ymd_opt(2026, 2, 22).unwrap(), 0.0), // Sunday
        ];

        let team_calendar = TeamCalendar::new();

        for (date, expected_capacity) in test_cases {
            let capacity = team_calendar.get_capacity(date);
            assert_eq!(
                capacity, expected_capacity,
                "Expected capacity of {} on {}, but got {}",
                expected_capacity, date, capacity
            );
        }
    }

    #[test]
    fn a_team_calendar_with_one_calendar_returns_capacity_correctly() {
        let mut team_calendar = TeamCalendar::new();
        let calendar = Calendar {
            free_weekdays: vec![Weekday::Mon, Weekday::Tue],
            free_date_ranges: vec![
                FreeDateRange {
                    start_date: NaiveDate::from_ymd_opt(2026, 2, 16).unwrap(),
                    end_date: NaiveDate::from_ymd_opt(2026, 2, 20).unwrap(),
                },
                FreeDateRange {
                    start_date: NaiveDate::from_ymd_opt(2026, 2, 27).unwrap(),
                    end_date: NaiveDate::from_ymd_opt(2026, 2, 27).unwrap(),
                },
            ],
        };
        team_calendar.calendars.push(calendar);

        let test_cases = vec![
            (NaiveDate::from_ymd_opt(2026, 2, 15).unwrap(), 1.0), // Monday
            (NaiveDate::from_ymd_opt(2026, 2, 16).unwrap(), 0.0), // Monday
            (NaiveDate::from_ymd_opt(2026, 2, 17).unwrap(), 0.0), // Tuesday
            (NaiveDate::from_ymd_opt(2026, 2, 18).unwrap(), 0.0), // Wednesday
            (NaiveDate::from_ymd_opt(2026, 2, 19).unwrap(), 0.0), // Thursday
            (NaiveDate::from_ymd_opt(2026, 2, 20).unwrap(), 0.0), // Friday
            (NaiveDate::from_ymd_opt(2026, 2, 21).unwrap(), 1.0), // Saturday
            (NaiveDate::from_ymd_opt(2026, 2, 22).unwrap(), 1.0), // Sunday
            (NaiveDate::from_ymd_opt(2026, 2, 23).unwrap(), 0.0), // Monday
            (NaiveDate::from_ymd_opt(2026, 2, 24).unwrap(), 0.0), // Tuesday
            (NaiveDate::from_ymd_opt(2026, 2, 25).unwrap(), 1.0), // Wednesday
            (NaiveDate::from_ymd_opt(2026, 2, 26).unwrap(), 1.0), // Thursday
            (NaiveDate::from_ymd_opt(2026, 2, 27).unwrap(), 0.0), // Friday
            (NaiveDate::from_ymd_opt(2026, 2, 28).unwrap(), 1.0), // Saturday
            (NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(), 1.0),  // Sunday
        ];

        for (date, expected_capacity) in test_cases {
            let capacity = team_calendar.get_capacity(date);
            assert_eq!(
                capacity, expected_capacity,
                "Expected capacity of {} on {}, but got {}",
                expected_capacity, date, capacity
            );
        }
    }

    #[test]
    fn three_calendars_are_combined_correctly() {
        let mut team_calendar = TeamCalendar::new();

        let calendar1 = Calendar {
            free_weekdays: vec![Weekday::Tue, Weekday::Wed, Weekday::Thu],
            free_date_ranges: vec![],
        };
        team_calendar.calendars.push(calendar1);

        let calendar2 = Calendar {
            free_weekdays: vec![Weekday::Wed, Weekday::Thu],
            free_date_ranges: vec![],
        };
        team_calendar.calendars.push(calendar2);

        let calendar3 = Calendar {
            free_weekdays: vec![],
            free_date_ranges: vec![FreeDateRange {
                start_date: NaiveDate::from_ymd_opt(2026, 2, 19).unwrap(),
                end_date: NaiveDate::from_ymd_opt(2026, 2, 20).unwrap(),
            }],
        };
        team_calendar.calendars.push(calendar3);

        let test_cases = vec![
            (NaiveDate::from_ymd_opt(2026, 2, 16).unwrap(), 1.0), // Monday
            (NaiveDate::from_ymd_opt(2026, 2, 17).unwrap(), 2.0 / 3.0), // Tuesday
            (NaiveDate::from_ymd_opt(2026, 2, 18).unwrap(), 1.0 / 3.0), // Wednesday
            (NaiveDate::from_ymd_opt(2026, 2, 19).unwrap(), 0.0), // Thursday
            (NaiveDate::from_ymd_opt(2026, 2, 20).unwrap(), 2.0 / 3.0), // Friday
            (NaiveDate::from_ymd_opt(2026, 2, 21).unwrap(), 1.0), // Saturday
            (NaiveDate::from_ymd_opt(2026, 2, 22).unwrap(), 1.0), // Sunday
        ];

        for (date, expected_capacity) in test_cases {
            let capacity = team_calendar.get_capacity(date);
            assert_eq!(
                capacity, expected_capacity,
                "Expected capacity of {} on {}, but got {}",
                expected_capacity, date, capacity
            );
        }
    }
}
