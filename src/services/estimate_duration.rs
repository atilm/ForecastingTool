use thiserror::Error;

use crate::domain::estimate::Estimate;
use crate::domain::issue::Issue;
use crate::domain::project::Project;
use crate::services::project_simulation::beta_pert_sampler::{pert_expected_value, PertError};

#[derive(Error, Debug, PartialEq)]
pub enum EstimateDurationError {
    #[error("issue '{0}' has no estimate")]
    MissingEstimate(String),
    #[error("issue '{0}' has incomplete three-point estimate")]
    IncompleteThreePoint(String),
    #[error("issue '{id}' has invalid PERT parameters: {source}")]
    InvalidPert {
        id: String,
        #[source]
        source: PertError,
    },
    #[error("issue '{0}' uses an unsupported estimate type for duration calculation")]
    UnsupportedEstimateType(String),
    #[error("issue '{0}' has no issue id")]
    MissingIssueId(String),
}

/// A computed duration for a single work package.
#[derive(Debug, Clone, PartialEq)]
pub struct WorkPackageDuration {
    pub id: String,
    pub summary: Option<String>,
    pub expected_days: f32,
}

/// Computes expected durations (in days) for all work packages in a project,
/// using the PERT expected value formula on three-point estimates.
pub fn compute_expected_durations(
    project: &Project,
) -> Result<Vec<WorkPackageDuration>, EstimateDurationError> {
    project
        .work_packages
        .iter()
        .map(compute_issue_duration)
        .collect()
}

fn compute_issue_duration(issue: &Issue) -> Result<WorkPackageDuration, EstimateDurationError> {
    let id = issue
        .issue_id
        .as_ref()
        .map(|i| i.id.clone())
        .unwrap_or_default();

    if id.is_empty() {
        return Err(EstimateDurationError::MissingIssueId(
            issue.summary.clone().unwrap_or_default(),
        ));
    }

    let estimate = issue
        .estimate
        .as_ref()
        .ok_or_else(|| EstimateDurationError::MissingEstimate(id.clone()))?;

    let expected_days = match estimate {
        Estimate::ThreePoint(tp) => {
            let optimistic = tp
                .optimistic
                .ok_or_else(|| EstimateDurationError::IncompleteThreePoint(id.clone()))?;
            let most_likely = tp
                .most_likely
                .ok_or_else(|| EstimateDurationError::IncompleteThreePoint(id.clone()))?;
            let pessimistic = tp
                .pessimistic
                .ok_or_else(|| EstimateDurationError::IncompleteThreePoint(id.clone()))?;
            pert_expected_value(optimistic, most_likely, pessimistic).map_err(|e| {
                EstimateDurationError::InvalidPert {
                    id: id.clone(),
                    source: e,
                }
            })?
        }
        Estimate::StoryPoint(sp) => sp.estimate.unwrap_or(0.0),
        Estimate::Reference(r) => {
            if let Some(cached) = &r.cached_estimate {
                let optimistic = cached
                    .optimistic
                    .ok_or_else(|| EstimateDurationError::IncompleteThreePoint(id.clone()))?;
                let most_likely = cached
                    .most_likely
                    .ok_or_else(|| EstimateDurationError::IncompleteThreePoint(id.clone()))?;
                let pessimistic = cached
                    .pessimistic
                    .ok_or_else(|| EstimateDurationError::IncompleteThreePoint(id.clone()))?;
                pert_expected_value(optimistic, most_likely, pessimistic).map_err(|e| {
                    EstimateDurationError::InvalidPert {
                        id: id.clone(),
                        source: e,
                    }
                })?
            } else {
                return Err(EstimateDurationError::UnsupportedEstimateType(id));
            }
        }
    };

    Ok(WorkPackageDuration {
        id,
        summary: issue.summary.clone(),
        expected_days,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::estimate::{StoryPointEstimate, ThreePointEstimate};
    use crate::domain::issue::{Issue, IssueId};

    fn build_three_point_issue(id: &str, opt: f32, ml: f32, pes: f32) -> Issue {
        let mut issue = Issue::new();
        issue.issue_id = Some(IssueId {
            id: id.to_string(),
        });
        issue.summary = Some(format!("Task {id}"));
        issue.estimate = Some(Estimate::ThreePoint(ThreePointEstimate {
            optimistic: Some(opt),
            most_likely: Some(ml),
            pessimistic: Some(pes),
        }));
        issue
    }

    fn build_story_point_issue(id: &str, points: f32) -> Issue {
        let mut issue = Issue::new();
        issue.issue_id = Some(IssueId {
            id: id.to_string(),
        });
        issue.summary = Some(format!("Task {id}"));
        issue.estimate = Some(Estimate::StoryPoint(StoryPointEstimate {
            estimate: Some(points),
        }));
        issue
    }

    #[test]
    fn computes_expected_duration_for_three_point_estimate() {
        let project = Project {
            name: "Test".to_string(),
            work_packages: vec![build_three_point_issue("A", 2.0, 3.0, 10.0)],
        };
        let durations = compute_expected_durations(&project).unwrap();
        assert_eq!(durations.len(), 1);
        assert_eq!(durations[0].id, "A");
        // (2 + 4*3 + 10) / 6 = 4.0
        assert!((durations[0].expected_days - 4.0).abs() < f32::EPSILON);
    }

    #[test]
    fn computes_expected_duration_for_story_points() {
        let project = Project {
            name: "Test".to_string(),
            work_packages: vec![build_story_point_issue("B", 5.0)],
        };
        let durations = compute_expected_durations(&project).unwrap();
        assert_eq!(durations[0].expected_days, 5.0);
    }

    #[test]
    fn computes_multiple_work_packages() {
        let project = Project {
            name: "Test".to_string(),
            work_packages: vec![
                build_three_point_issue("A", 1.0, 2.0, 3.0),
                build_three_point_issue("B", 3.0, 5.0, 7.0),
            ],
        };
        let durations = compute_expected_durations(&project).unwrap();
        assert_eq!(durations.len(), 2);
        // (1 + 8 + 3) / 6 = 2.0
        assert!((durations[0].expected_days - 2.0).abs() < f32::EPSILON);
        // (3 + 20 + 7) / 6 = 5.0
        assert!((durations[1].expected_days - 5.0).abs() < f32::EPSILON);
    }

    #[test]
    fn rejects_missing_estimate() {
        let mut issue = Issue::new();
        issue.issue_id = Some(IssueId {
            id: "X".to_string(),
        });
        let project = Project {
            name: "Test".to_string(),
            work_packages: vec![issue],
        };
        let err = compute_expected_durations(&project).unwrap_err();
        assert!(matches!(err, EstimateDurationError::MissingEstimate(_)));
    }

    #[test]
    fn rejects_incomplete_three_point_estimate() {
        let mut issue = Issue::new();
        issue.issue_id = Some(IssueId {
            id: "Y".to_string(),
        });
        issue.estimate = Some(Estimate::ThreePoint(ThreePointEstimate {
            optimistic: Some(1.0),
            most_likely: None,
            pessimistic: Some(5.0),
        }));
        let project = Project {
            name: "Test".to_string(),
            work_packages: vec![issue],
        };
        let err = compute_expected_durations(&project).unwrap_err();
        assert!(matches!(
            err,
            EstimateDurationError::IncompleteThreePoint(_)
        ));
    }

    #[test]
    fn zero_duration_three_point() {
        let project = Project {
            name: "Test".to_string(),
            work_packages: vec![build_three_point_issue("M", 0.0, 0.0, 0.0)],
        };
        let durations = compute_expected_durations(&project).unwrap();
        assert!((durations[0].expected_days - 0.0).abs() < f32::EPSILON);
    }
}
