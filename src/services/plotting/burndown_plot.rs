use chrono::{Duration, NaiveDate};
use thiserror::Error;

use crate::domain::estimate::{Estimate, StoryPointEstimate};
use crate::services::parsing::simulation_report_yaml::{
    ReportParseError, load_simulation_report_from_file,
};
use crate::services::parsing::team_calendar_yaml::{
    TeamCalendarYamlError, load_team_calendar_if_provided,
};
use crate::services::plotting::burndown_plot_rendering::render_burndown_plot_png;
use crate::services::project_simulation::simulation_types::{
    SimulationReport, WorkPackageSimulation, WorkPackageType,
};

#[derive(Error, Debug)]
pub enum BurndownPlotError {
    #[error("failed to parse simulation report yaml: {0}")]
    ParseReport(#[from] ReportParseError),
    #[error("simulation report has no done work packages")]
    NoDoneIssues,
    #[error("simulation report has no todo, in-progress, or dynamically created work packages")]
    NoForecastIssues,
    #[error("done work package '{id}' is missing done_date")]
    MissingDoneDate { id: String },
    #[error("simulation report has no work package data")]
    MissingSimulationWorkPackages,
    #[error("failed to parse team calendar yaml: {0}")]
    ParseCalendar(#[from] TeamCalendarYamlError),
    #[error("work package '{id}' has unsupported estimate type for burndown")]
    UnsupportedEstimateType { id: String },
    #[error("failed to render burndown plot: {0}")]
    Plot(String),
}

#[derive(Clone)]
struct DoneIssue {
    points: f32,
    done_date: NaiveDate,
}

#[derive(Clone)]
struct ForecastIssue {
    points: f32,
    work_package_type: WorkPackageType,
    p15: NaiveDate,
    p50: NaiveDate,
    p85: NaiveDate,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ChartPoint {
    pub(crate) date: NaiveDate,
    pub(crate) remaining: f32,
}

#[derive(Debug, Clone)]
pub(crate) struct ClassifiedChartPoint {
    pub(crate) point: ChartPoint,
    pub(crate) work_package_type: WorkPackageType,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct CapacityRange {
    pub(crate) start_date: NaiveDate,
    pub(crate) end_date: NaiveDate,
    pub(crate) capacity: f32,
}

#[derive(Debug)]
pub(crate) struct BurndownData {
    pub(crate) start_date: NaiveDate,
    pub(crate) end_date: NaiveDate,
    pub(crate) total_points: f32,
    pub(crate) capacity_ranges: Vec<CapacityRange>,
    pub(crate) done_points: Vec<ChartPoint>,
    pub(crate) p15_points: Vec<ChartPoint>,
    pub(crate) p85_points: Vec<ChartPoint>,
    pub(crate) forecast_p50_points: Vec<ClassifiedChartPoint>,
}

pub fn plot_burndown_from_yaml_files(
    report_path: &str,
    output_path: &str,
    calendar_path: Option<&str>,
) -> Result<(), BurndownPlotError> {
    let report = load_simulation_report_from_file(report_path)?;
    let calendar = load_team_calendar_if_provided(calendar_path)?;
    let data = build_burndown_data(&report, calendar_path.map(|_| &calendar))?;
    render_burndown_plot_png(output_path, &data)
}

fn build_burndown_data(
    report: &SimulationReport,
    calendar: Option<&crate::domain::calendar::TeamCalendar>,
) -> Result<BurndownData, BurndownPlotError> {
    let work_packages = report
        .work_packages
        .as_deref()
        .ok_or(BurndownPlotError::MissingSimulationWorkPackages)?;

    let mut done_issues = Vec::new();
    let mut forecast_issues = Vec::new();

    for work_package in work_packages {
        if work_package.work_package_type == WorkPackageType::Milestone {
            continue;
        }

        let points = story_points_for_burndown(work_package)?;
        if work_package.work_package_type == WorkPackageType::Done {
            let done_date =
                work_package
                    .done_date
                    .ok_or_else(|| BurndownPlotError::MissingDoneDate {
                        id: work_package.id.clone(),
                    })?;
            done_issues.push(DoneIssue { points, done_date });
            continue;
        }

        forecast_issues.push(ForecastIssue {
            points,
            work_package_type: work_package.work_package_type.clone(),
            p15: work_package.percentiles.p15.end_date,
            p50: work_package.percentiles.p50.end_date,
            p85: work_package.percentiles.p85.end_date,
        });
    }

    if done_issues.is_empty() {
        return Err(BurndownPlotError::NoDoneIssues);
    }
    if forecast_issues.is_empty() {
        return Err(BurndownPlotError::NoForecastIssues);
    }

    done_issues.sort_by_key(|item| item.done_date);

    let start_date = done_issues
        .iter()
        .map(|item| item.done_date)
        .min()
        .ok_or(BurndownPlotError::NoDoneIssues)?;
    let end_date = forecast_issues
        .iter()
        .map(|item| item.p85)
        .max()
        .unwrap_or(start_date);

    let total_points = done_issues.iter().map(|item| item.points).sum::<f32>()
        + forecast_issues.iter().map(|item| item.points).sum::<f32>();

    let capacity_ranges = calendar
        .map(|team_calendar| build_capacity_ranges(start_date, end_date, team_calendar))
        .unwrap_or_default();

    let done_events: Vec<(NaiveDate, f32)> = done_issues
        .iter()
        .map(|item| (item.done_date, item.points))
        .collect();

    let done_points = build_done_points(&done_issues, total_points);
    let p15_points = build_forecast_points(total_points, &done_events, &forecast_issues, |i| i.p15);
    let forecast_p50_points =
        build_classified_forecast_points(total_points, &done_events, &forecast_issues, |i| i.p50);
    let p85_points = build_forecast_points(total_points, &done_events, &forecast_issues, |i| i.p85);

    Ok(BurndownData {
        start_date,
        end_date,
        total_points,
        capacity_ranges,
        done_points,
        p15_points,
        p85_points,
        forecast_p50_points,
    })
}

fn build_capacity_ranges(
    start_date: NaiveDate,
    end_date: NaiveDate,
    calendar: &crate::domain::calendar::TeamCalendar,
) -> Vec<CapacityRange> {
    let mut ranges = Vec::new();
    let mut current_date = start_date;
    let mut active_start = None;
    let mut active_capacity = 1.0;
    let mut active_end = start_date;

    while current_date <= end_date {
        let capacity = calendar.get_capacity_from_free_ranges_only(current_date);
        if capacity < 1.0 {
            match active_start {
                Some(_) if same_capacity(active_capacity, capacity) => {
                    active_end = current_date;
                }
                Some(start_date) => {
                    ranges.push(CapacityRange {
                        start_date,
                        end_date: active_end,
                        capacity: active_capacity,
                    });
                    active_start = Some(current_date);
                    active_capacity = capacity;
                    active_end = current_date;
                }
                None => {
                    active_start = Some(current_date);
                    active_capacity = capacity;
                    active_end = current_date;
                }
            }
        } else if let Some(start_date) = active_start.take() {
            ranges.push(CapacityRange {
                start_date,
                end_date: active_end,
                capacity: active_capacity,
            });
        }

        current_date += Duration::days(1);
    }

    if let Some(start_date) = active_start {
        ranges.push(CapacityRange {
            start_date,
            end_date: active_end,
            capacity: active_capacity,
        });
    }

    ranges
}

fn same_capacity(left: f32, right: f32) -> bool {
    (left - right).abs() < 0.000_1
}

fn build_done_points(done_issues: &[DoneIssue], total_points: f32) -> Vec<ChartPoint> {
    let mut remaining = total_points;
    done_issues
        .iter()
        .map(|item| {
            remaining -= item.points;
            ChartPoint {
                date: item.done_date,
                remaining,
            }
        })
        .collect()
}

fn build_forecast_points<F>(
    total_points: f32,
    done_events: &[(NaiveDate, f32)],
    forecast_issues: &[ForecastIssue],
    date_selector: F,
) -> Vec<ChartPoint>
where
    F: Fn(&ForecastIssue) -> NaiveDate,
{
    let mut forecast_events: Vec<(NaiveDate, f32)> = forecast_issues
        .iter()
        .map(|item| (date_selector(item), item.points))
        .collect();
    forecast_events.sort_by_key(|item| item.0);

    let mut points = Vec::with_capacity(forecast_events.len());
    let mut done_idx = 0;
    let mut done_sum = 0.0;
    let mut forecast_sum = 0.0;

    for (date, issue_points) in forecast_events {
        // skip done events that happened before the forecast date
        while done_idx < done_events.len() && done_events[done_idx].0 <= date {
            done_sum += done_events[done_idx].1;
            done_idx += 1;
        }

        forecast_sum += issue_points;
        points.push(ChartPoint {
            date,
            remaining: total_points - done_sum - forecast_sum,
        });
    }

    points
}

fn build_classified_forecast_points<F>(
    total_points: f32,
    done_events: &[(NaiveDate, f32)],
    forecast_issues: &[ForecastIssue],
    date_selector: F,
) -> Vec<ClassifiedChartPoint>
where
    F: Fn(&ForecastIssue) -> NaiveDate,
{
    let mut forecast_events: Vec<(&ForecastIssue, NaiveDate)> = forecast_issues
        .iter()
        .map(|item| (item, date_selector(item)))
        .collect();
    forecast_events.sort_by_key(|(_, date)| *date);

    let mut done_idx = 0;
    let mut done_sum = 0.0;
    let mut forecast_sum = 0.0;
    forecast_events
        .into_iter()
        .map(|(issue, date)| {
            while done_idx < done_events.len() && done_events[done_idx].0 <= date {
                done_sum += done_events[done_idx].1;
                done_idx += 1;
            }
            forecast_sum += issue.points;
            ClassifiedChartPoint {
                point: ChartPoint {
                    date,
                    remaining: total_points - done_sum - forecast_sum,
                },
                work_package_type: issue.work_package_type.clone(),
            }
        })
        .collect()
}

fn story_points_for_burndown(
    work_package: &WorkPackageSimulation,
) -> Result<f32, BurndownPlotError> {
    match work_package.estimate.as_ref() {
        None => Ok(1.0),
        Some(Estimate::StoryPoint(StoryPointEstimate { estimate })) => Ok(estimate.unwrap_or(1.0)),
        _ => Err(BurndownPlotError::UnsupportedEstimateType {
            id: work_package.id.clone(),
        }),
    }
}

#[cfg(test)]
#[path = "burndown_plot_tests.rs"]
mod burndown_plot_tests;
