use super::Database;
use crate::types::*;
use anyhow::Result;
use chrono::Utc;
use rusqlite::params;

impl Database {
    pub fn create_document(&self, p: &CreateDocumentParams) -> Result<Document> {
        let conn = self.conn.lock().unwrap();
        let user_id = Self::default_user_id();
        let now = Utc::now().naive_utc();
        let tags_json = serde_json::to_string(&p.tags)?;
        let size = p.content.len() as i64;

        conn.execute(
            "INSERT INTO documents (user_id, project_id, title, description, content, document_type, filename, size_bytes, tags, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![user_id, p.project_id, p.title, p.description, p.content, p.document_type, p.filename, size, tags_json, now, now],
        )?;
        let id = conn.last_insert_rowid();
        drop(conn);
        self.log_activity("document", id, "create", None)?;
        self.get_document_inner(id)
    }

    pub fn get_document(&self, document_id: i64) -> Result<Document> {
        self.get_document_inner(document_id)
    }

    fn get_document_inner(&self, document_id: i64) -> Result<Document> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, user_id, project_id, title, description, content, document_type, filename, size_bytes, tags, created_at, updated_at
             FROM documents WHERE id = ?1",
            params![document_id],
            |row| {
                Ok(Document {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    project_id: row.get(2)?,
                    title: row.get(3)?,
                    description: row.get(4)?,
                    content: row.get(5)?,
                    document_type: row.get(6)?,
                    filename: row.get(7)?,
                    size_bytes: row.get(8)?,
                    tags: serde_json::from_str::<Vec<String>>(&row.get::<_, String>(9)?).unwrap_or_default(),
                    created_at: row.get(10)?,
                    updated_at: row.get(11)?,
                })
            },
        ).map_err(|_| anyhow::anyhow!("Document not found: {}", document_id))
    }

    pub fn list_documents(&self, project_id: Option<i64>, document_type: Option<&str>) -> Result<Vec<Document>> {
        let conn = self.conn.lock().unwrap();
        let mut sql = "SELECT id, user_id, project_id, title, description, content, document_type, filename, size_bytes, tags, created_at, updated_at FROM documents WHERE 1=1".to_string();
        let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        let mut idx = 1;

        if let Some(pid) = project_id {
            sql.push_str(&format!(" AND project_id = ?{}", idx));
            params_vec.push(Box::new(pid));
            idx += 1;
        }
        if let Some(dt) = document_type {
            sql.push_str(&format!(" AND document_type = ?{}", idx));
            params_vec.push(Box::new(dt.to_string()));
        }
        sql.push_str(" ORDER BY updated_at DESC");

        let refs: Vec<&dyn rusqlite::types::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
        let mut stmt = conn.prepare(&sql)?;
        let docs: Vec<Document> = stmt.query_map(refs.as_slice(), |row| {
            Ok(Document {
                id: row.get(0)?,
                user_id: row.get(1)?,
                project_id: row.get(2)?,
                title: row.get(3)?,
                description: row.get(4)?,
                content: row.get(5)?,
                document_type: row.get(6)?,
                filename: row.get(7)?,
                size_bytes: row.get(8)?,
                tags: serde_json::from_str::<Vec<String>>(&row.get::<_, String>(9)?).unwrap_or_default(),
                created_at: row.get(10)?,
                updated_at: row.get(11)?,
            })
        })?.filter_map(|r| r.ok()).collect();

        Ok(docs)
    }

    pub fn update_document(&self, p: &UpdateDocumentParams) -> Result<Document> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().naive_utc();

        let mut sets = vec!["updated_at = ?1".to_string()];
        let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(now)];
        let mut idx = 2;

        if let Some(ref v) = p.title { sets.push(format!("title = ?{}", idx)); params_vec.push(Box::new(v.clone())); idx += 1; }
        if let Some(ref v) = p.description { sets.push(format!("description = ?{}", idx)); params_vec.push(Box::new(v.clone())); idx += 1; }
        if let Some(ref v) = p.content {
            sets.push(format!("content = ?{}", idx));
            params_vec.push(Box::new(v.clone()));
            idx += 1;
            sets.push(format!("size_bytes = ?{}", idx));
            params_vec.push(Box::new(v.len() as i64));
            idx += 1;
        }
        if let Some(ref v) = p.document_type { sets.push(format!("document_type = ?{}", idx)); params_vec.push(Box::new(v.clone())); idx += 1; }
        if let Some(ref v) = p.filename { sets.push(format!("filename = ?{}", idx)); params_vec.push(Box::new(v.clone())); idx += 1; }
        if let Some(ref v) = p.tags { sets.push(format!("tags = ?{}", idx)); params_vec.push(Box::new(serde_json::to_string(v).unwrap_or_default())); idx += 1; }

        params_vec.push(Box::new(p.document_id));
        let sql = format!("UPDATE documents SET {} WHERE id = ?{}", sets.join(", "), idx);
        let refs: Vec<&dyn rusqlite::types::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, refs.as_slice())?;

        drop(conn);
        self.log_activity("document", p.document_id, "update", None)?;
        self.get_document_inner(p.document_id)
    }

    pub fn delete_document(&self, document_id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        self.log_activity_inner(&conn, "document", document_id, "delete", None)?;
        conn.execute("DELETE FROM documents WHERE id = ?1", params![document_id])?;
        Ok(())
    }
}
