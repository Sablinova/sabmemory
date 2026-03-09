use super::Database;
use crate::types::*;
use anyhow::Result;
use chrono::Utc;
use rusqlite::params;

impl Database {
    pub fn create_project(&self, p: &CreateProjectParams) -> Result<Project> {
        let conn = self.conn.lock().unwrap();
        let user_id = Self::default_user_id();
        let now = Utc::now().naive_utc();

        conn.execute(
            "INSERT INTO projects (user_id, name, description, project_type, status, repo_name, notes, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, 'active', ?5, ?6, ?7, ?8)",
            params![user_id, p.name, p.description, p.project_type, p.repo_name, p.notes, now, now],
        )?;
        let id = conn.last_insert_rowid();
        drop(conn);
        self.log_activity("project", id, "create", None)?;
        self.get_project_inner(id)
    }

    pub fn get_project(&self, project_id: i64) -> Result<Project> {
        self.get_project_inner(project_id)
    }

    fn get_project_inner(&self, project_id: i64) -> Result<Project> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, user_id, name, description, project_type, status, repo_name, notes, created_at, updated_at
             FROM projects WHERE id = ?1",
            params![project_id],
            |row| {
                Ok(Project {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    name: row.get(2)?,
                    description: row.get(3)?,
                    project_type: row.get(4)?,
                    status: row.get(5)?,
                    repo_name: row.get(6)?,
                    notes: row.get(7)?,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            },
        ).map_err(|_| anyhow::anyhow!("Project not found: {}", project_id))
    }

    pub fn list_projects(&self, repo_name: Option<&str>, status: Option<&str>) -> Result<Vec<Project>> {
        let conn = self.conn.lock().unwrap();

        let mut sql = "SELECT id, user_id, name, description, project_type, status, repo_name, notes, created_at, updated_at FROM projects WHERE 1=1".to_string();
        let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        let mut idx = 1;

        if let Some(rn) = repo_name {
            sql.push_str(&format!(" AND repo_name = ?{}", idx));
            params_vec.push(Box::new(rn.to_string()));
            idx += 1;
        }
        if let Some(st) = status {
            sql.push_str(&format!(" AND status = ?{}", idx));
            params_vec.push(Box::new(st.to_string()));
        }
        sql.push_str(" ORDER BY name");

        let refs: Vec<&dyn rusqlite::types::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
        let mut stmt = conn.prepare(&sql)?;
        let projects: Vec<Project> = stmt.query_map(refs.as_slice(), |row| {
            Ok(Project {
                id: row.get(0)?,
                user_id: row.get(1)?,
                name: row.get(2)?,
                description: row.get(3)?,
                project_type: row.get(4)?,
                status: row.get(5)?,
                repo_name: row.get(6)?,
                notes: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })?.filter_map(|r| r.ok()).collect();

        // Add memory_count for each project
        Ok(projects)
    }

    pub fn update_project(&self, p: &UpdateProjectParams) -> Result<Project> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().naive_utc();

        let mut sets = vec!["updated_at = ?1".to_string()];
        let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(now)];
        let mut idx = 2;

        if let Some(ref v) = p.name { sets.push(format!("name = ?{}", idx)); params_vec.push(Box::new(v.clone())); idx += 1; }
        if let Some(ref v) = p.description { sets.push(format!("description = ?{}", idx)); params_vec.push(Box::new(v.clone())); idx += 1; }
        if let Some(ref v) = p.project_type { sets.push(format!("project_type = ?{}", idx)); params_vec.push(Box::new(v.clone())); idx += 1; }
        if let Some(ref v) = p.status { sets.push(format!("status = ?{}", idx)); params_vec.push(Box::new(v.clone())); idx += 1; }
        if let Some(ref v) = p.repo_name { sets.push(format!("repo_name = ?{}", idx)); params_vec.push(Box::new(v.clone())); idx += 1; }
        if let Some(ref v) = p.notes { sets.push(format!("notes = ?{}", idx)); params_vec.push(Box::new(v.clone())); idx += 1; }

        params_vec.push(Box::new(p.project_id));
        let sql = format!("UPDATE projects SET {} WHERE id = ?{}", sets.join(", "), idx);
        let refs: Vec<&dyn rusqlite::types::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, refs.as_slice())?;

        drop(conn);
        self.log_activity("project", p.project_id, "update", None)?;
        self.get_project_inner(p.project_id)
    }

    pub fn delete_project(&self, project_id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        self.log_activity_inner(&conn, "project", project_id, "delete", None)?;
        conn.execute("DELETE FROM projects WHERE id = ?1", params![project_id])?;
        Ok(())
    }
}
