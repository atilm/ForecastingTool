use crate::domain::issue::Issue;

#[derive(Debug, Clone, PartialEq)]
pub struct Project {
    pub name: String,
    pub work_packages: Vec<Issue>,
}
