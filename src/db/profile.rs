use super::Database;
use crate::types::*;
use anyhow::Result;
use chrono::Utc;
use rusqlite::params;

impl Database {
    pub fn get_profile(&self) -> Result<UserProfile> {
        let conn = self.conn.lock().unwrap();
        let result = conn.query_row(
            "SELECT id, user_id, static_facts, dynamic_facts, generated_at, updated_at FROM user_profiles WHERE user_id = 'default' ORDER BY updated_at DESC LIMIT 1",
            [],
            |row| {
                Ok(UserProfile {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    static_facts: serde_json::from_str::<Vec<String>>(&row.get::<_, String>(2)?).unwrap_or_default(),
                    dynamic_facts: serde_json::from_str::<Vec<String>>(&row.get::<_, String>(3)?).unwrap_or_default(),
                    generated_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            },
        );

        match result {
            Ok(profile) => Ok(profile),
            Err(_) => {
                // Generate initial profile
                drop(conn);
                self.refresh_profile()
            }
        }
    }

    pub fn refresh_profile(&self) -> Result<UserProfile> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().naive_utc();

        // Gather static facts: high-importance memories of type 'fact'
        let mut stmt = conn.prepare(
            "SELECT title, content FROM memories
             WHERE is_obsolete = 0 AND is_forgotten = 0 AND importance >= 7 AND memory_type = 'fact'
             ORDER BY importance DESC, updated_at DESC
             LIMIT 50"
        )?;
        let static_facts: Vec<String> = stmt.query_map([], |row| {
            let title: String = row.get(0)?;
            let content: String = row.get(1)?;
            Ok(format!("{}: {}", title, truncate_str(&content, 200)))
        })?.filter_map(|r| r.ok()).collect();

        // Gather dynamic facts: preferences and recent episodes
        let mut stmt2 = conn.prepare(
            "SELECT title, content FROM memories
             WHERE is_obsolete = 0 AND is_forgotten = 0 AND memory_type IN ('preference', 'episode')
             ORDER BY updated_at DESC
             LIMIT 20"
        )?;
        let dynamic_facts: Vec<String> = stmt2.query_map([], |row| {
            let title: String = row.get(0)?;
            let content: String = row.get(1)?;
            Ok(format!("{}: {}", title, truncate_str(&content, 200)))
        })?.filter_map(|r| r.ok()).collect();

        let static_json = serde_json::to_string(&static_facts)?;
        let dynamic_json = serde_json::to_string(&dynamic_facts)?;

        // Upsert profile
        conn.execute(
            "DELETE FROM user_profiles WHERE user_id = 'default'"
            , [],
        )?;
        conn.execute(
            "INSERT INTO user_profiles (user_id, static_facts, dynamic_facts, generated_at, updated_at)
             VALUES ('default', ?1, ?2, ?3, ?4)",
            params![static_json, dynamic_json, now, now],
        )?;
        let id = conn.last_insert_rowid();

        Ok(UserProfile {
            id,
            user_id: "default".to_string(),
            static_facts,
            dynamic_facts,
            generated_at: now,
            updated_at: now,
        })
    }
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}
