use crate::domain::{epic::Epic, issue::Issue};
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
#[async_trait::async_trait]
pub trait DataSource {
    async fn get_epic(&self, epic_id: &str) -> Result<Epic, DataSourceError>;
    async fn get_issues(&self, query: DataQuery) -> Result<Vec<Issue>, DataSourceError>;
}
