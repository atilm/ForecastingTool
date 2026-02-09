use crate::domain::epic::Epic;
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

/// Describes an interface for retrieving Epic and Issue information.
pub trait DataSource {
    async fn get_epic(&self, epic_id: &str) -> Result<Epic, DataSourceError>;
}
