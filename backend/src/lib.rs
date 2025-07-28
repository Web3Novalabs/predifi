pub mod config;
pub mod controllers;
pub mod db;
pub mod error;
pub mod models;
pub mod routes;

pub use error::{AppError, AppResult};

use crate::db::database::Database;

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
}
