use super::Database;
use crate::types::*;
use anyhow::Result;
use chrono::Utc;
use rusqlite::params;

impl Database {
    pub fn get_user(&self) -> Result<User> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, external_id, name, email, notes, created_at, updated_at FROM users WHERE id = 'default'",
            [],
            |row| {
                Ok(User {
                    id: row.get(0)?,
                    external_id: row.get(1)?,
                    name: row.get(2)?,
                    email: row.get(3)?,
                    notes: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            },
        ).map_err(|_| anyhow::anyhow!("Default user not found"))
    }

    pub fn update_user_notes(&self, notes: &str) -> Result<User> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().naive_utc();
        conn.execute(
            "UPDATE users SET notes = ?1, updated_at = ?2 WHERE id = 'default'",
            params![notes, now],
        )?;
        drop(conn);
        self.get_user()
    }
}
