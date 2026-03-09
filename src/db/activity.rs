use super::Database;
use anyhow::Result;
use rusqlite::params;

impl Database {
    pub fn log_activity(&self, entity_type: &str, entity_id: i64, action: &str, changes: Option<&str>) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        self.log_activity_inner(&conn, entity_type, entity_id, action, changes)
    }

    pub fn log_activity_inner(&self, conn: &rusqlite::Connection, entity_type: &str, entity_id: i64, action: &str, changes: Option<&str>) -> Result<()> {
        let user_id = Self::default_user_id();
        conn.execute(
            "INSERT INTO activity_log (user_id, entity_type, entity_id, action, changes, snapshot, actor)
             VALUES (?1, ?2, ?3, ?4, ?5, '{}', 'agent')",
            params![user_id, entity_type, entity_id, action, changes],
        )?;
        Ok(())
    }
}
