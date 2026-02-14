use crate::domain::{issue::Issue, project::Project};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DataSourceError {
    #[error("resource not found")]
    NotFound,
    #[error("connection error")]
    Connection,
    #[error("parse error")]
    Parse,
    #[error("unauthorized")]
    Unauthorized,
    #[error("{0}")]
    Other(String),
}

pub enum DataQuery {
    StringQuery(String),
}

/// Describes an interface for retrieving Epic and Issue information.
pub trait DataSource {
    fn get_issues(&self, query: DataQuery) -> Result<Vec<Issue>, DataSourceError>;
    fn get_project(&self, query: DataQuery) -> Result<Project, DataSourceError>;
}
