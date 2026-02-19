use assert_fs::prelude::*;
use chrono::NaiveDate;

use forecasts::services::team_calendar_yaml::load_team_calendar_from_yaml_dir;

#[test]
fn loads_team_calendar_from_yaml_directory() {
    let temp = assert_fs::TempDir::new().unwrap();

    temp.child("calendar_1.yaml")
        .write_str(
            r#"free_weekdays: [Mon, Tue]
free_date_ranges:
  - start_date: 2026-02-27
    end_date: 2026-02-27
"#,
        )
        .unwrap();

    temp.child("calendar_2.yaml")
        .write_str(
            r#"free_weekdays: [Wed]
free_date_ranges:
  - start_date: 2026-02-19
    end_date: 2026-02-20
"#,
        )
        .unwrap();

    let team_calendar = load_team_calendar_from_yaml_dir(temp.path()).unwrap();
    assert_eq!(team_calendar.calendars.len(), 2);

    let mon = NaiveDate::from_ymd_opt(2026, 2, 16).unwrap();
    let tue = NaiveDate::from_ymd_opt(2026, 2, 17).unwrap();
    let wed = NaiveDate::from_ymd_opt(2026, 2, 18).unwrap();
    let thu = NaiveDate::from_ymd_opt(2026, 2, 19).unwrap();
    let fri = NaiveDate::from_ymd_opt(2026, 2, 20).unwrap();
    let fri_27 = NaiveDate::from_ymd_opt(2026, 2, 27).unwrap();

    assert_eq!(team_calendar.get_capacity(mon), 0.5);
    assert_eq!(team_calendar.get_capacity(tue), 0.5);
    assert_eq!(team_calendar.get_capacity(wed), 0.5);
    assert_eq!(team_calendar.get_capacity(thu), 0.5);
    assert_eq!(team_calendar.get_capacity(fri), 0.5);
    assert_eq!(team_calendar.get_capacity(fri_27), 0.5);
}
