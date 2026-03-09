use super::Database;
use crate::types::*;
use anyhow::Result;
use chrono::Utc;
use rusqlite::params;

impl Database {
    pub fn create_entity(&self, p: &CreateEntityParams) -> Result<Entity> {
        let conn = self.conn.lock().unwrap();
        let user_id = Self::default_user_id();
        let now = Utc::now().naive_utc();
        let tags_json = serde_json::to_string(&p.tags)?;
        let aka_json = serde_json::to_string(&p.aka)?;

        conn.execute(
            "INSERT INTO entities (user_id, name, entity_type, custom_type, notes, tags, aka, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![user_id, p.name, p.entity_type, p.custom_type, p.notes, tags_json, aka_json, now, now],
        )?;
        let id = conn.last_insert_rowid();
        drop(conn);
        self.log_activity("entity", id, "create", None)?;
        self.get_entity_inner(id)
    }

    pub fn get_entity(&self, entity_id: i64) -> Result<Entity> {
        self.get_entity_inner(entity_id)
    }

    fn get_entity_inner(&self, entity_id: i64) -> Result<Entity> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, user_id, name, entity_type, custom_type, notes, tags, aka, created_at, updated_at
             FROM entities WHERE id = ?1",
            params![entity_id],
            |row| {
                Ok(Entity {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    name: row.get(2)?,
                    entity_type: row.get(3)?,
                    custom_type: row.get(4)?,
                    notes: row.get(5)?,
                    tags: serde_json::from_str::<Vec<String>>(&row.get::<_, String>(6)?).unwrap_or_default(),
                    aka: serde_json::from_str::<Vec<String>>(&row.get::<_, String>(7)?).unwrap_or_default(),
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            },
        ).map_err(|_| anyhow::anyhow!("Entity not found: {}", entity_id))
    }

    pub fn list_entities(&self, project_ids: &[i64], entity_type: Option<&str>) -> Result<Vec<Entity>> {
        let conn = self.conn.lock().unwrap();
        let mut sql = "SELECT id, user_id, name, entity_type, custom_type, notes, tags, aka, created_at, updated_at FROM entities WHERE 1=1".to_string();

        if entity_type.is_some() {
            sql.push_str(" AND entity_type = ?1");
        }
        sql.push_str(" ORDER BY name");

        let mut stmt = conn.prepare(&sql)?;
        let entities: Vec<Entity> = if let Some(et) = entity_type {
            stmt.query_map(params![et], |row| {
                Ok(Entity {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    name: row.get(2)?,
                    entity_type: row.get(3)?,
                    custom_type: row.get(4)?,
                    notes: row.get(5)?,
                    tags: serde_json::from_str::<Vec<String>>(&row.get::<_, String>(6)?).unwrap_or_default(),
                    aka: serde_json::from_str::<Vec<String>>(&row.get::<_, String>(7)?).unwrap_or_default(),
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            })?.filter_map(|r| r.ok()).collect()
        } else {
            stmt.query_map([], |row| {
                Ok(Entity {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    name: row.get(2)?,
                    entity_type: row.get(3)?,
                    custom_type: row.get(4)?,
                    notes: row.get(5)?,
                    tags: serde_json::from_str::<Vec<String>>(&row.get::<_, String>(6)?).unwrap_or_default(),
                    aka: serde_json::from_str::<Vec<String>>(&row.get::<_, String>(7)?).unwrap_or_default(),
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            })?.filter_map(|r| r.ok()).collect()
        };

        // Post-filter by project if needed
        if !project_ids.is_empty() {
            let filtered: Vec<Entity> = entities.into_iter().filter(|e| {
                let pids = self.get_assoc_ids_inner(&conn, "entity_project_association", "entity_id", "project_id", e.id).unwrap_or_default();
                project_ids.iter().any(|pid| pids.contains(pid))
            }).collect();
            return Ok(filtered);
        }

        Ok(entities)
    }

    pub fn search_entities(&self, query: &str) -> Result<Vec<Entity>> {
        let conn = self.conn.lock().unwrap();
        let fts_query = query.split_whitespace()
            .map(|w| format!("\"{}\"", w.chars().filter(|c| c.is_alphanumeric()).collect::<String>()))
            .collect::<Vec<_>>()
            .join(" OR ");

        if fts_query.is_empty() {
            return Ok(Vec::new());
        }

        let mut stmt = conn.prepare(
            "SELECT e.id, e.user_id, e.name, e.entity_type, e.custom_type, e.notes, e.tags, e.aka, e.created_at, e.updated_at
             FROM entities_fts fts
             JOIN entities e ON e.id = fts.rowid
             WHERE entities_fts MATCH ?1
             ORDER BY bm25(entities_fts)
             LIMIT 20"
        )?;

        let entities: Vec<Entity> = stmt.query_map(params![fts_query], |row| {
            Ok(Entity {
                id: row.get(0)?,
                user_id: row.get(1)?,
                name: row.get(2)?,
                entity_type: row.get(3)?,
                custom_type: row.get(4)?,
                notes: row.get(5)?,
                tags: serde_json::from_str::<Vec<String>>(&row.get::<_, String>(6)?).unwrap_or_default(),
                aka: serde_json::from_str::<Vec<String>>(&row.get::<_, String>(7)?).unwrap_or_default(),
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })?.filter_map(|r| r.ok()).collect();

        Ok(entities)
    }

    pub fn update_entity(&self, p: &UpdateEntityParams) -> Result<Entity> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().naive_utc();

        let mut sets = vec!["updated_at = ?1".to_string()];
        let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(now)];
        let mut idx = 2;

        if let Some(ref v) = p.name { sets.push(format!("name = ?{}", idx)); params_vec.push(Box::new(v.clone())); idx += 1; }
        if let Some(ref v) = p.entity_type { sets.push(format!("entity_type = ?{}", idx)); params_vec.push(Box::new(v.clone())); idx += 1; }
        if let Some(ref v) = p.custom_type { sets.push(format!("custom_type = ?{}", idx)); params_vec.push(Box::new(v.clone())); idx += 1; }
        if let Some(ref v) = p.notes { sets.push(format!("notes = ?{}", idx)); params_vec.push(Box::new(v.clone())); idx += 1; }
        if let Some(ref v) = p.tags { sets.push(format!("tags = ?{}", idx)); params_vec.push(Box::new(serde_json::to_string(v).unwrap_or_default())); idx += 1; }
        if let Some(ref v) = p.aka { sets.push(format!("aka = ?{}", idx)); params_vec.push(Box::new(serde_json::to_string(v).unwrap_or_default())); idx += 1; }

        params_vec.push(Box::new(p.entity_id));
        let sql = format!("UPDATE entities SET {} WHERE id = ?{}", sets.join(", "), idx);
        let refs: Vec<&dyn rusqlite::types::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, refs.as_slice())?;

        drop(conn);
        self.log_activity("entity", p.entity_id, "update", None)?;
        self.get_entity_inner(p.entity_id)
    }

    pub fn delete_entity(&self, entity_id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        self.log_activity_inner(&conn, "entity", entity_id, "delete", None)?;
        conn.execute("DELETE FROM entities WHERE id = ?1", params![entity_id])?;
        Ok(())
    }

    pub fn link_entity_memory(&self, entity_id: i64, memory_id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO memory_entity_association (memory_id, entity_id) VALUES (?1, ?2)",
            params![memory_id, entity_id],
        )?;
        Ok(())
    }

    pub fn unlink_entity_memory(&self, entity_id: i64, memory_id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM memory_entity_association WHERE memory_id = ?1 AND entity_id = ?2",
            params![memory_id, entity_id],
        )?;
        Ok(())
    }

    pub fn link_entity_project(&self, entity_id: i64, project_id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO entity_project_association (entity_id, project_id) VALUES (?1, ?2)",
            params![entity_id, project_id],
        )?;
        Ok(())
    }

    pub fn unlink_entity_project(&self, entity_id: i64, project_id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM entity_project_association WHERE entity_id = ?1 AND project_id = ?2",
            params![entity_id, project_id],
        )?;
        Ok(())
    }

    pub fn get_entity_memories(&self, entity_id: i64) -> Result<Vec<i64>> {
        let conn = self.conn.lock().unwrap();
        self.get_assoc_ids_inner(&conn, "memory_entity_association", "entity_id", "memory_id", entity_id)
    }

    pub fn create_relationship(&self, p: &CreateRelationshipParams) -> Result<EntityRelationship> {
        let conn = self.conn.lock().unwrap();
        let user_id = Self::default_user_id();
        let now = Utc::now().naive_utc();
        let meta_json = p.metadata.as_ref().map(|m| serde_json::to_string(m).unwrap_or_default());

        conn.execute(
            "INSERT INTO entity_relationships (user_id, source_entity_id, target_entity_id, relationship_type, strength, confidence, relationship_metadata, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![user_id, p.source_entity_id, p.target_entity_id, p.relationship_type, p.strength, p.confidence, meta_json, now, now],
        )?;
        let id = conn.last_insert_rowid();
        drop(conn);
        self.get_relationship_inner(id)
    }

    fn get_relationship_inner(&self, rel_id: i64) -> Result<EntityRelationship> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, user_id, source_entity_id, target_entity_id, relationship_type, strength, confidence, relationship_metadata, created_at, updated_at
             FROM entity_relationships WHERE id = ?1",
            params![rel_id],
            |row| {
                Ok(EntityRelationship {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    source_entity_id: row.get(2)?,
                    target_entity_id: row.get(3)?,
                    relationship_type: row.get(4)?,
                    strength: row.get(5)?,
                    confidence: row.get(6)?,
                    relationship_metadata: row.get::<_, Option<String>>(7)?
                        .and_then(|s| serde_json::from_str(&s).ok()),
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            },
        ).map_err(|_| anyhow::anyhow!("Relationship not found: {}", rel_id))
    }

    pub fn get_relationships(&self, entity_id: i64, direction: &str) -> Result<Vec<EntityRelationship>> {
        let conn = self.conn.lock().unwrap();
        let sql = match direction {
            "outgoing" => "SELECT id, user_id, source_entity_id, target_entity_id, relationship_type, strength, confidence, relationship_metadata, created_at, updated_at FROM entity_relationships WHERE source_entity_id = ?1",
            "incoming" => "SELECT id, user_id, source_entity_id, target_entity_id, relationship_type, strength, confidence, relationship_metadata, created_at, updated_at FROM entity_relationships WHERE target_entity_id = ?1",
            _ => "SELECT id, user_id, source_entity_id, target_entity_id, relationship_type, strength, confidence, relationship_metadata, created_at, updated_at FROM entity_relationships WHERE source_entity_id = ?1 OR target_entity_id = ?1",
        };

        let mut stmt = conn.prepare(sql)?;
        let rels: Vec<EntityRelationship> = stmt.query_map(params![entity_id], |row| {
            Ok(EntityRelationship {
                id: row.get(0)?,
                user_id: row.get(1)?,
                source_entity_id: row.get(2)?,
                target_entity_id: row.get(3)?,
                relationship_type: row.get(4)?,
                strength: row.get(5)?,
                confidence: row.get(6)?,
                relationship_metadata: row.get::<_, Option<String>>(7)?
                    .and_then(|s| serde_json::from_str(&s).ok()),
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })?.filter_map(|r| r.ok()).collect();

        Ok(rels)
    }

    pub fn update_relationship(&self, p: &UpdateRelationshipParams) -> Result<EntityRelationship> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().naive_utc();

        let mut sets = vec!["updated_at = ?1".to_string()];
        let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(now)];
        let mut idx = 2;

        if let Some(ref v) = p.relationship_type { sets.push(format!("relationship_type = ?{}", idx)); params_vec.push(Box::new(v.clone())); idx += 1; }
        if let Some(v) = p.strength { sets.push(format!("strength = ?{}", idx)); params_vec.push(Box::new(v)); idx += 1; }
        if let Some(v) = p.confidence { sets.push(format!("confidence = ?{}", idx)); params_vec.push(Box::new(v)); idx += 1; }
        if let Some(ref v) = p.metadata { sets.push(format!("relationship_metadata = ?{}", idx)); params_vec.push(Box::new(serde_json::to_string(v).unwrap_or_default())); idx += 1; }

        params_vec.push(Box::new(p.relationship_id));
        let sql = format!("UPDATE entity_relationships SET {} WHERE id = ?{}", sets.join(", "), idx);
        let refs: Vec<&dyn rusqlite::types::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, refs.as_slice())?;

        drop(conn);
        self.get_relationship_inner(p.relationship_id)
    }

    pub fn delete_relationship(&self, rel_id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM entity_relationships WHERE id = ?1", params![rel_id])?;
        Ok(())
    }
}
