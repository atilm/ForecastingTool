use crate::domain::estimate::{Estimate, StoryPointEstimate};
use crate::domain::issue::Issue;

#[derive(Debug, Clone, PartialEq)]
pub struct Project {
    pub name: String,
    pub work_packages: Vec<Issue>,
}

impl Project {
    pub fn has_story_points(&self) -> bool {
        self.work_packages.iter().any(|issue| {
            matches!(
                issue.estimate,
                Some(Estimate::StoryPoint(StoryPointEstimate {
                    estimate: Some(_)
                }))
            )
        })
    }
}
