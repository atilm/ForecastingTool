use std::io;
use std::path::{Path, PathBuf};

use chrono::{NaiveDate, Weekday};
use serde::Deserialize;
use thiserror::Error;

use crate::domain::calendar::{Calendar, FreeDateRange, TeamCalendar};

#[derive(Error, Debug)]
pub enum TeamCalendarYamlError {
    #[error("calendar directory not found: {0}")]
    DirectoryNotFound(PathBuf),
    #[error("calendar directory contains no yaml files: {0}")]
    DirectoryEmpty(PathBuf),
    #[error("failed to list calendar directory {path}: {source}")]
    ReadDir {
        path: PathBuf,
        source: io::Error,
    },
    #[error("failed to read calendar yaml file {path}: {source}")]
    ReadFile {
        path: PathBuf,
        source: io::Error,
    },
    #[error("failed to parse calendar yaml file {path}: {source}")]
    Parse {
        path: PathBuf,
        source: serde_yaml::Error,
    },
    #[error("invalid weekday value in {path}: {value}")]
    InvalidWeekday { path: PathBuf, value: String },
    #[error("invalid date format in {path}: {value} (expected YYYY-MM-DD)")]
    InvalidDate { path: PathBuf, value: String },
    #[error(
        "invalid date range in {path}: start_date {start_date} is after end_date {end_date}"
    )]
    InvalidDateRange {
        path: PathBuf,
        start_date: NaiveDate,
        end_date: NaiveDate,
    },
}

#[derive(Debug, Deserialize)]
struct CalendarRecord {
    free_weekdays: Option<Vec<String>>,
    free_date_ranges: Option<Vec<FreeDateRangeRecord>>,
}

#[derive(Debug, Deserialize)]
struct FreeDateRangeRecord {
    start_date: String,
    end_date: String,
}

/// Loads all `*.yaml` / `*.yml` files in `dir_path`, parses each file into a [`Calendar`],
/// and composes them into a [`TeamCalendar`].
///
/// # Errors
/// - Returns an error when `dir_path` does not exist.
/// - Returns an error when no YAML files are present.
/// - Returns an error on I/O or parse failures, or when content is invalid.
pub fn load_team_calendar_from_yaml_dir<P: AsRef<Path>>(
    dir_path: P,
) -> Result<TeamCalendar, TeamCalendarYamlError> {
    let dir_path = dir_path.as_ref();
    if !dir_path.exists() {
        return Err(TeamCalendarYamlError::DirectoryNotFound(
            dir_path.to_path_buf(),
        ));
    }
    if !dir_path.is_dir() {
        return Err(TeamCalendarYamlError::DirectoryNotFound(
            dir_path.to_path_buf(),
        ));
    }

    let mut yaml_files = Vec::new();
    let read_dir = std::fs::read_dir(dir_path).map_err(|source| TeamCalendarYamlError::ReadDir {
        path: dir_path.to_path_buf(),
        source,
    })?;
    for entry in read_dir {
        let entry = entry.map_err(|source| TeamCalendarYamlError::ReadDir {
            path: dir_path.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        if path.is_file() && is_yaml_file(&path) {
            yaml_files.push(path);
        }
    }
    yaml_files.sort();
    if yaml_files.is_empty() {
        return Err(TeamCalendarYamlError::DirectoryEmpty(dir_path.to_path_buf()));
    }

    let mut team_calendar = TeamCalendar::new();
    team_calendar.calendars = yaml_files
        .iter()
        .map(|file_path| load_calendar_from_yaml_file(file_path))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(team_calendar)
}

fn is_yaml_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("yaml") | Some("yml")
    )
}

fn load_calendar_from_yaml_file(path: &Path) -> Result<Calendar, TeamCalendarYamlError> {
    let contents = std::fs::read_to_string(path).map_err(|source| TeamCalendarYamlError::ReadFile {
        path: path.to_path_buf(),
        source,
    })?;
    deserialize_calendar_from_yaml_str(&contents, path)
}

fn deserialize_calendar_from_yaml_str(
    input: &str,
    origin_path: &Path,
) -> Result<Calendar, TeamCalendarYamlError> {
    let record: CalendarRecord = serde_yaml::from_str(input).map_err(|source| {
        TeamCalendarYamlError::Parse {
            path: origin_path.to_path_buf(),
            source,
        }
    })?;

    let free_weekdays = record
        .free_weekdays
        .unwrap_or_default()
        .into_iter()
        .map(|value| {
            parse_weekday(&value).ok_or_else(|| TeamCalendarYamlError::InvalidWeekday {
                path: origin_path.to_path_buf(),
                value,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let free_date_ranges = record
        .free_date_ranges
        .unwrap_or_default()
        .into_iter()
        .map(|value| free_date_range_from_record(value, origin_path))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Calendar {
        free_weekdays,
        free_date_ranges,
    })
}

fn free_date_range_from_record(
    value: FreeDateRangeRecord,
    origin_path: &Path,
) -> Result<FreeDateRange, TeamCalendarYamlError> {
    let start_date = parse_date(&value.start_date, origin_path)?;
    let end_date = parse_date(&value.end_date, origin_path)?;
    if start_date > end_date {
        return Err(TeamCalendarYamlError::InvalidDateRange {
            path: origin_path.to_path_buf(),
            start_date,
            end_date,
        });
    }
    Ok(FreeDateRange {
        start_date,
        end_date,
    })
}

fn parse_date(value: &str, origin_path: &Path) -> Result<NaiveDate, TeamCalendarYamlError> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| TeamCalendarYamlError::InvalidDate {
            path: origin_path.to_path_buf(),
            value: value.to_string(),
        })
}

fn parse_weekday(value: &str) -> Option<Weekday> {
    match value.trim().to_ascii_lowercase().as_str() {
        "mon" | "monday" => Some(Weekday::Mon),
        "tue" | "tues" | "tuesday" => Some(Weekday::Tue),
        "wed" | "wednesday" => Some(Weekday::Wed),
        "thu" | "thur" | "thurs" | "thursday" => Some(Weekday::Thu),
        "fri" | "friday" => Some(Weekday::Fri),
        "sat" | "saturday" => Some(Weekday::Sat),
        "sun" | "sunday" => Some(Weekday::Sun),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use assert_fs::prelude::*;

    #[test]
    fn returns_error_when_directory_does_not_exist() {
        let temp = assert_fs::TempDir::new().unwrap();
        let missing = temp.path().join("does-not-exist");

        let err = load_team_calendar_from_yaml_dir(&missing).unwrap_err();
        assert!(matches!(err, TeamCalendarYamlError::DirectoryNotFound(p) if p == missing));
    }

    #[test]
    fn returns_error_when_directory_is_empty_of_yaml_files() {
        let temp = assert_fs::TempDir::new().unwrap();
        temp.child("readme.txt").write_str("hello").unwrap();

        let err = load_team_calendar_from_yaml_dir(temp.path()).unwrap_err();
        assert!(matches!(err, TeamCalendarYamlError::DirectoryEmpty(p) if p == temp.path()));
    }

    #[test]
    fn returns_error_on_invalid_yaml_syntax() {
        let temp = assert_fs::TempDir::new().unwrap();
        let file = temp.child("calendar.yaml");
        file.write_str("free_weekdays: [Mon\n").unwrap();

        let err = load_team_calendar_from_yaml_dir(temp.path()).unwrap_err();
        assert!(matches!(err, TeamCalendarYamlError::Parse { .. }));
    }

    #[test]
    fn returns_error_on_invalid_weekday_value() {
        let temp = assert_fs::TempDir::new().unwrap();
        let file = temp.child("calendar.yaml");
        file.write_str("free_weekdays: [Funday]\n").unwrap();

        let err = load_team_calendar_from_yaml_dir(temp.path()).unwrap_err();
        assert!(matches!(err, TeamCalendarYamlError::InvalidWeekday { .. }));
    }

    #[test]
    fn returns_error_on_invalid_date_format() {
        let temp = assert_fs::TempDir::new().unwrap();
        let file = temp.child("calendar.yaml");
        file.write_str(
            "free_date_ranges:\n  - start_date: 2026-02-xx\n    end_date: 2026-02-20\n",
        )
        .unwrap();

        let err = load_team_calendar_from_yaml_dir(temp.path()).unwrap_err();
        assert!(matches!(err, TeamCalendarYamlError::InvalidDate { .. }));
    }

    #[test]
    fn returns_error_on_invalid_date_range_when_start_after_end() {
        let temp = assert_fs::TempDir::new().unwrap();
        let file = temp.child("calendar.yaml");
        file.write_str(
            "free_date_ranges:\n  - start_date: 2026-02-21\n    end_date: 2026-02-20\n",
        )
        .unwrap();

        let err = load_team_calendar_from_yaml_dir(temp.path()).unwrap_err();
        assert!(matches!(err, TeamCalendarYamlError::InvalidDateRange { .. }));
    }

    #[test]
    fn loads_and_composes_multiple_calendar_files() {
        let temp = assert_fs::TempDir::new().unwrap();
        temp.child("a.yaml")
            .write_str("free_weekdays: [Mon]\n")
            .unwrap();
        temp.child("b.yml")
            .write_str(
                "free_date_ranges:\n  - start_date: 2026-02-19\n    end_date: 2026-02-20\n",
            )
            .unwrap();

        let team_calendar = load_team_calendar_from_yaml_dir(temp.path()).unwrap();
        assert_eq!(team_calendar.calendars.len(), 2);

        let monday = NaiveDate::from_ymd_opt(2026, 2, 16).unwrap();
        let wednesday = NaiveDate::from_ymd_opt(2026, 2, 18).unwrap();
        let thursday = NaiveDate::from_ymd_opt(2026, 2, 19).unwrap();
        assert_eq!(team_calendar.get_capacity(monday), 0.5);
        assert_eq!(team_calendar.get_capacity(wednesday), 1.0);
        assert_eq!(team_calendar.get_capacity(thursday), 0.5);
    }
}
