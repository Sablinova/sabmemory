pub mod schema;
pub mod memory;
pub mod entity;
pub mod project;
pub mod document;
pub mod code_artifact;
pub mod user;
pub mod profile;
pub mod activity;

use std::path::PathBuf;
use std::sync::Mutex;
use rusqlite::Connection;
use anyhow::Result;

pub struct Database {
    pub conn: Mutex<Connection>,
}

impl Database {
    pub fn new(path: &PathBuf) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; PRAGMA busy_timeout=5000;")?;
        let db = Database { conn: Mutex::new(conn) };
        schema::initialize(&db)?;
        Ok(db)
    }

    pub fn default_user_id() -> String {
        "default".to_string()
    }
}
