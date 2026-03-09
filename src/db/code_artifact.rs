use super::Database;
use crate::types::*;
use anyhow::Result;
use chrono::Utc;
use rusqlite::params;

impl Database {
    pub fn create_code_artifact(&self, p: &CreateCodeArtifactParams) -> Result<CodeArtifact> {
        let conn = self.conn.lock().unwrap();
        let user_id = Self::default_user_id();
        let now = Utc::now().naive_utc();
        let tags_json = serde_json::to_string(&p.tags)?;

        conn.execute(
            "INSERT INTO code_artifacts (user_id, project_id, title, description, code, language, tags, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![user_id, p.project_id, p.title, p.description, p.code, p.language, tags_json, now, now],
        )?;
        let id = conn.last_insert_rowid();
        drop(conn);
        self.log_activity("code_artifact", id, "create", None)?;
        self.get_code_artifact_inner(id)
    }

    pub fn get_code_artifact(&self, id: i64) -> Result<CodeArtifact> {
        self.get_code_artifact_inner(id)
    }

    fn get_code_artifact_inner(&self, id: i64) -> Result<CodeArtifact> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, user_id, project_id, title, description, code, language, tags, created_at, updated_at
             FROM code_artifacts WHERE id = ?1",
            params![id],
            |row| {
                Ok(CodeArtifact {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    project_id: row.get(2)?,
                    title: row.get(3)?,
                    description: row.get(4)?,
                    code: row.get(5)?,
                    language: row.get(6)?,
                    tags: serde_json::from_str::<Vec<String>>(&row.get::<_, String>(7)?).unwrap_or_default(),
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            },
        ).map_err(|_| anyhow::anyhow!("Code artifact not found: {}", id))
    }

    pub fn list_code_artifacts(&self, project_id: Option<i64>, language: Option<&str>) -> Result<Vec<CodeArtifact>> {
        let conn = self.conn.lock().unwrap();
        let mut sql = "SELECT id, user_id, project_id, title, description, code, language, tags, created_at, updated_at FROM code_artifacts WHERE 1=1".to_string();
        let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        let mut idx = 1;

        if let Some(pid) = project_id {
            sql.push_str(&format!(" AND project_id = ?{}", idx));
            params_vec.push(Box::new(pid));
            idx += 1;
        }
        if let Some(lang) = language {
            sql.push_str(&format!(" AND language = ?{}", idx));
            params_vec.push(Box::new(lang.to_string()));
        }
        sql.push_str(" ORDER BY updated_at DESC");

        let refs: Vec<&dyn rusqlite::types::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
        let mut stmt = conn.prepare(&sql)?;
        let artifacts: Vec<CodeArtifact> = stmt.query_map(refs.as_slice(), |row| {
            Ok(CodeArtifact {
                id: row.get(0)?,
                user_id: row.get(1)?,
                project_id: row.get(2)?,
                title: row.get(3)?,
                description: row.get(4)?,
                code: row.get(5)?,
                language: row.get(6)?,
                tags: serde_json::from_str::<Vec<String>>(&row.get::<_, String>(7)?).unwrap_or_default(),
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })?.filter_map(|r| r.ok()).collect();

        Ok(artifacts)
    }

    pub fn update_code_artifact(&self, p: &UpdateCodeArtifactParams) -> Result<CodeArtifact> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().naive_utc();

        let mut sets = vec!["updated_at = ?1".to_string()];
        let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(now)];
        let mut idx = 2;

        if let Some(ref v) = p.title { sets.push(format!("title = ?{}", idx)); params_vec.push(Box::new(v.clone())); idx += 1; }
        if let Some(ref v) = p.description { sets.push(format!("description = ?{}", idx)); params_vec.push(Box::new(v.clone())); idx += 1; }
        if let Some(ref v) = p.code { sets.push(format!("code = ?{}", idx)); params_vec.push(Box::new(v.clone())); idx += 1; }
        if let Some(ref v) = p.language { sets.push(format!("language = ?{}", idx)); params_vec.push(Box::new(v.clone())); idx += 1; }
        if let Some(ref v) = p.tags { sets.push(format!("tags = ?{}", idx)); params_vec.push(Box::new(serde_json::to_string(v).unwrap_or_default())); idx += 1; }

        params_vec.push(Box::new(p.code_artifact_id));
        let sql = format!("UPDATE code_artifacts SET {} WHERE id = ?{}", sets.join(", "), idx);
        let refs: Vec<&dyn rusqlite::types::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, refs.as_slice())?;

        drop(conn);
        self.log_activity("code_artifact", p.code_artifact_id, "update", None)?;
        self.get_code_artifact_inner(p.code_artifact_id)
    }

    pub fn delete_code_artifact(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        self.log_activity_inner(&conn, "code_artifact", id, "delete", None)?;
        conn.execute("DELETE FROM code_artifacts WHERE id = ?1", params![id])?;
        Ok(())
    }
}
