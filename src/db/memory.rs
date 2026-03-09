use super::Database;
use crate::types::*;
use anyhow::Result;
use chrono::{NaiveDateTime, Utc};
use rusqlite::params;

impl Database {
    pub fn create_memory(&self, p: &CreateMemoryParams) -> Result<MemoryWithLinks> {
        let conn = self.conn.lock().unwrap();
        let user_id = Self::default_user_id();
        let now = Utc::now().naive_utc();
        let keywords_json = serde_json::to_string(&p.keywords)?;
        let tags_json = serde_json::to_string(&p.tags)?;
        let source_files_json = p.source_files.as_ref().map(|f| serde_json::to_string(f).unwrap_or_default());
        let forget_after: Option<NaiveDateTime> = p.forget_after.as_ref().and_then(|s| {
            NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
                .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S"))
                .or_else(|_| {
                    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
                        .map(|d| d.and_hms_opt(0, 0, 0).unwrap())
                })
                .ok()
        });

        conn.execute(
            "INSERT INTO memories (user_id, title, content, context, keywords, tags, importance,
             source_repo, source_files, source_url, confidence, encoding_agent,
             memory_type, container_tag, forget_after, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
            params![
                user_id, p.title, p.content, p.context, keywords_json, tags_json, p.importance,
                p.source_repo, source_files_json, p.source_url, p.confidence, p.encoding_agent,
                p.memory_type, p.container_tag, forget_after, now, now
            ],
        )?;
        let memory_id = conn.last_insert_rowid();

        // Create associations
        for pid in &p.project_ids {
            conn.execute(
                "INSERT OR IGNORE INTO memory_project_association (memory_id, project_id) VALUES (?1, ?2)",
                params![memory_id, pid],
            )?;
        }
        for eid in &p.entity_ids {
            conn.execute(
                "INSERT OR IGNORE INTO memory_entity_association (memory_id, entity_id) VALUES (?1, ?2)",
                params![memory_id, eid],
            )?;
        }
        for did in &p.document_ids {
            conn.execute(
                "INSERT OR IGNORE INTO memory_document_association (memory_id, document_id) VALUES (?1, ?2)",
                params![memory_id, did],
            )?;
        }
        for cid in &p.code_artifact_ids {
            conn.execute(
                "INSERT OR IGNORE INTO memory_code_artifact_association (memory_id, code_artifact_id) VALUES (?1, ?2)",
                params![memory_id, cid],
            )?;
        }

        drop(conn);

        // Auto-link to similar memories
        self.auto_link_memory(memory_id)?;

        // Log activity
        self.log_activity("memory", memory_id, "create", None)?;

        self.get_memory_with_links(memory_id)
    }

    pub fn get_memory_with_links(&self, memory_id: i64) -> Result<MemoryWithLinks> {
        self.get_memory_with_links_inner(memory_id)
    }

    pub fn get_memory_with_links_inner(&self, memory_id: i64) -> Result<MemoryWithLinks> {
        let conn = self.conn.lock().unwrap();

        let memory = conn.query_row(
            "SELECT id, user_id, title, content, context, keywords, tags, importance,
             is_obsolete, obsolete_reason, superseded_by, obsoleted_at,
             source_repo, source_files, source_url, confidence, encoding_agent, encoding_version,
             version, parent_memory_id, is_latest, relationship_type,
             forget_after, is_forgotten, forgotten_at, memory_type, container_tag,
             created_at, updated_at
             FROM memories WHERE id = ?1",
            params![memory_id],
            |row| {
                Ok(Memory {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    title: row.get(2)?,
                    content: row.get(3)?,
                    context: row.get(4)?,
                    keywords: serde_json::from_str::<Vec<String>>(&row.get::<_, String>(5)?).unwrap_or_default(),
                    tags: serde_json::from_str::<Vec<String>>(&row.get::<_, String>(6)?).unwrap_or_default(),
                    importance: row.get(7)?,
                    is_obsolete: row.get::<_, i32>(8)? != 0,
                    obsolete_reason: row.get(9)?,
                    superseded_by: row.get(10)?,
                    obsoleted_at: row.get(11)?,
                    source_repo: row.get(12)?,
                    source_files: row.get::<_, Option<String>>(13)?
                        .and_then(|s| serde_json::from_str(&s).ok()),
                    source_url: row.get(14)?,
                    confidence: row.get(15)?,
                    encoding_agent: row.get(16)?,
                    encoding_version: row.get(17)?,
                    version: row.get(18)?,
                    parent_memory_id: row.get(19)?,
                    is_latest: row.get::<_, i32>(20)? != 0,
                    relationship_type: row.get(21)?,
                    forget_after: row.get(22)?,
                    is_forgotten: row.get::<_, i32>(23)? != 0,
                    forgotten_at: row.get(24)?,
                    memory_type: row.get(25)?,
                    container_tag: row.get(26)?,
                    created_at: row.get(27)?,
                    updated_at: row.get(28)?,
                })
            },
        ).map_err(|_| anyhow::anyhow!("Memory not found: {}", memory_id))?;

        let linked_ids = self.get_linked_ids_inner(&conn, memory_id)?;
        let project_ids = self.get_assoc_ids_inner(&conn, "memory_project_association", "memory_id", "project_id", memory_id)?;
        let entity_ids = self.get_assoc_ids_inner(&conn, "memory_entity_association", "memory_id", "entity_id", memory_id)?;
        let document_ids = self.get_assoc_ids_inner(&conn, "memory_document_association", "memory_id", "document_id", memory_id)?;
        let code_artifact_ids = self.get_assoc_ids_inner(&conn, "memory_code_artifact_association", "memory_id", "code_artifact_id", memory_id)?;

        Ok(MemoryWithLinks {
            memory,
            linked_memory_ids: linked_ids,
            project_ids,
            entity_ids,
            document_ids,
            code_artifact_ids,
        })
    }

    fn get_linked_ids_inner(&self, conn: &rusqlite::Connection, memory_id: i64) -> Result<Vec<i64>> {
        let mut stmt = conn.prepare(
            "SELECT target_id FROM memory_links WHERE source_id = ?1
             UNION
             SELECT source_id FROM memory_links WHERE target_id = ?1"
        )?;
        let ids: Vec<i64> = stmt.query_map(params![memory_id], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(ids)
    }

    pub fn get_assoc_ids_inner(&self, conn: &rusqlite::Connection, table: &str, key_col: &str, val_col: &str, key: i64) -> Result<Vec<i64>> {
        let sql = format!("SELECT {} FROM {} WHERE {} = ?1", val_col, table, key_col);
        let mut stmt = conn.prepare(&sql)?;
        let ids: Vec<i64> = stmt.query_map(params![key], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(ids)
    }

    pub fn query_memories(&self, p: &QueryMemoryParams) -> Result<SearchResult> {
        let conn = self.conn.lock().unwrap();

        // Build FTS5 query from natural language
        let fts_query = build_fts_query(&p.query, p.query_context.as_deref());

        // Run FTS5 search with BM25 ranking
        let sql = format!(
            "SELECT m.id, m.user_id, m.title, m.content, m.context, m.keywords, m.tags,
             m.importance, m.is_obsolete, m.obsolete_reason, m.superseded_by, m.obsoleted_at,
             m.source_repo, m.source_files, m.source_url, m.confidence, m.encoding_agent, m.encoding_version,
             m.version, m.parent_memory_id, m.is_latest, m.relationship_type,
             m.forget_after, m.is_forgotten, m.forgotten_at, m.memory_type, m.container_tag,
             m.created_at, m.updated_at,
             bm25(memories_fts, 5.0, 3.0, 2.0, 1.0, 1.0) as rank
             FROM memories_fts fts
             JOIN memories m ON m.id = fts.rowid
             WHERE memories_fts MATCH ?1
             AND m.is_obsolete = 0 AND m.is_forgotten = 0
             {}{}{}
             ORDER BY (rank * (m.importance / 10.0)) ASC
             LIMIT ?2",
            if !p.project_ids.is_empty() {
                " AND m.id IN (SELECT memory_id FROM memory_project_association WHERE project_id IN (SELECT value FROM json_each(?3)))"
            } else { "" },
            if !p.tags.is_empty() {
                " AND EXISTS (SELECT 1 FROM json_each(m.tags) t, json_each(?4) f WHERE t.value = f.value)"
            } else { "" },
            if p.container_tag.is_some() {
                " AND m.container_tag = ?5"
            } else { "" },
        );

        let stmt = conn.prepare(&sql)?;
        let _param_idx = 1;
        let _fts_q_clone = fts_query.clone();

        // We need a simpler approach to handle variable params
        // Let's use a direct query approach
        drop(stmt);

        let memories = self.search_memories_inner(&conn, &fts_query, p)?;

        let mut primary_memories = Vec::new();
        let mut token_count = 0;
        let max_memories = p.k.min(20) as usize;
        let token_budget = p.token_budget as usize;

        for mem in memories.into_iter().take(max_memories) {
            let mem_tokens = estimate_tokens(&format!("{} {} {}", mem.memory.title, mem.memory.content, mem.memory.context));
            if token_count + mem_tokens > token_budget && !primary_memories.is_empty() {
                break;
            }
            token_count += mem_tokens;
            primary_memories.push(mem);
        }

        // Collect linked memories
        let mut linked_memories = Vec::new();
        if p.include_links {
            let mut seen_ids: std::collections::HashSet<i64> = primary_memories.iter().map(|m| m.memory.id).collect();
            for pm in &primary_memories {
                let mut link_count = 0;
                for linked_id in &pm.linked_memory_ids {
                    if link_count >= p.max_links_per_primary { break; }
                    if seen_ids.contains(linked_id) { continue; }

                    if let Ok(linked_mem) = self.get_memory_with_links_inner(*linked_id) {
                        if linked_mem.memory.is_obsolete || linked_mem.memory.is_forgotten { continue; }
                        let mem_tokens = estimate_tokens(&format!("{} {} {}",
                            linked_mem.memory.title, linked_mem.memory.content, linked_mem.memory.context));
                        if token_count + mem_tokens > token_budget { break; }
                        token_count += mem_tokens;
                        seen_ids.insert(*linked_id);
                        linked_memories.push(LinkedMemoryEntry {
                            link_source_id: pm.memory.id,
                            memory: linked_mem,
                        });
                        link_count += 1;
                    }
                }
            }
        }

        let total = primary_memories.len() + linked_memories.len();
        Ok(SearchResult {
            total_count: total,
            truncated: token_count >= token_budget,
            token_count,
            primary_memories,
            linked_memories,
        })
    }

    fn search_memories_inner(&self, conn: &rusqlite::Connection, fts_query: &str, p: &QueryMemoryParams) -> Result<Vec<MemoryWithLinks>> {
        // Simple FTS5 search — handle project/tag filtering in post-processing for simplicity
        let mut stmt = conn.prepare(
            "SELECT m.id, m.user_id, m.title, m.content, m.context, m.keywords, m.tags,
             m.importance, m.is_obsolete, m.obsolete_reason, m.superseded_by, m.obsoleted_at,
             m.source_repo, m.source_files, m.source_url, m.confidence, m.encoding_agent, m.encoding_version,
             m.version, m.parent_memory_id, m.is_latest, m.relationship_type,
             m.forget_after, m.is_forgotten, m.forgotten_at, m.memory_type, m.container_tag,
             m.created_at, m.updated_at,
             bm25(memories_fts, 5.0, 3.0, 2.0, 1.0, 1.0) as rank
             FROM memories_fts fts
             JOIN memories m ON m.id = fts.rowid
             WHERE memories_fts MATCH ?1
             AND m.is_obsolete = 0 AND m.is_forgotten = 0
             ORDER BY (rank * (CAST(m.importance AS REAL) / 10.0))
             LIMIT 50"
        )?;

        let rows: Vec<(Memory, f64)> = stmt.query_map(params![fts_query], |row| {
            Ok((Memory {
                id: row.get(0)?,
                user_id: row.get(1)?,
                title: row.get(2)?,
                content: row.get(3)?,
                context: row.get(4)?,
                keywords: serde_json::from_str::<Vec<String>>(&row.get::<_, String>(5)?).unwrap_or_default(),
                tags: serde_json::from_str::<Vec<String>>(&row.get::<_, String>(6)?).unwrap_or_default(),
                importance: row.get(7)?,
                is_obsolete: row.get::<_, i32>(8)? != 0,
                obsolete_reason: row.get(9)?,
                superseded_by: row.get(10)?,
                obsoleted_at: row.get(11)?,
                source_repo: row.get(12)?,
                source_files: row.get::<_, Option<String>>(13)?
                    .and_then(|s| serde_json::from_str(&s).ok()),
                source_url: row.get(14)?,
                confidence: row.get(15)?,
                encoding_agent: row.get(16)?,
                encoding_version: row.get(17)?,
                version: row.get(18)?,
                parent_memory_id: row.get(19)?,
                is_latest: row.get::<_, i32>(20)? != 0,
                relationship_type: row.get(21)?,
                forget_after: row.get(22)?,
                is_forgotten: row.get::<_, i32>(23)? != 0,
                forgotten_at: row.get(24)?,
                memory_type: row.get(25)?,
                container_tag: row.get(26)?,
                created_at: row.get(27)?,
                updated_at: row.get(28)?,
            }, row.get::<_, f64>(29)?))
        })?.filter_map(|r| r.ok()).collect();

        drop(stmt);

        let mut results = Vec::new();
        for (memory, _rank) in rows {
            let mid = memory.id;

            // Apply post-filters
            if !p.project_ids.is_empty() {
                let pids = self.get_assoc_ids_inner(conn, "memory_project_association", "memory_id", "project_id", mid)?;
                if !p.project_ids.iter().any(|pid| pids.contains(pid)) {
                    continue;
                }
            }
            if let Some(min_imp) = p.min_importance {
                if memory.importance < min_imp { continue; }
            }
            if let Some(ref ct) = p.container_tag {
                if memory.container_tag.as_ref() != Some(ct) { continue; }
            }
            if !p.tags.is_empty() {
                if !p.tags.iter().any(|t| memory.tags.contains(t)) { continue; }
            }

            // Check expiry
            if let Some(ref fa) = memory.forget_after {
                if *fa < Utc::now().naive_utc() { continue; }
            }

            let linked_ids = self.get_linked_ids_inner(conn, mid)?;
            let project_ids = self.get_assoc_ids_inner(conn, "memory_project_association", "memory_id", "project_id", mid)?;
            let entity_ids = self.get_assoc_ids_inner(conn, "memory_entity_association", "memory_id", "entity_id", mid)?;
            let document_ids = self.get_assoc_ids_inner(conn, "memory_document_association", "memory_id", "document_id", mid)?;
            let code_artifact_ids = self.get_assoc_ids_inner(conn, "memory_code_artifact_association", "memory_id", "code_artifact_id", mid)?;

            results.push(MemoryWithLinks {
                memory,
                linked_memory_ids: linked_ids,
                project_ids,
                entity_ids,
                document_ids,
                code_artifact_ids,
            });
        }

        Ok(results)
    }

    pub fn update_memory(&self, p: &UpdateMemoryParams) -> Result<MemoryWithLinks> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().naive_utc();

        let mut sets = vec!["updated_at = ?1".to_string()];
        let mut param_count = 2;

        macro_rules! add_field {
            ($field:ident, $col:expr) => {
                if p.$field.is_some() {
                    sets.push(format!("{} = ?{}", $col, param_count));
                    param_count += 1;
                }
            };
        }
        add_field!(title, "title");
        add_field!(content, "content");
        add_field!(context, "context");
        add_field!(importance, "importance");
        add_field!(source_repo, "source_repo");
        add_field!(source_url, "source_url");
        add_field!(confidence, "confidence");
        add_field!(container_tag, "container_tag");

        // Handle JSON fields separately
        let keywords_json = p.keywords.as_ref().map(|k| serde_json::to_string(k).unwrap_or_default());
        let tags_json = p.tags.as_ref().map(|t| serde_json::to_string(t).unwrap_or_default());
        let source_files_json = p.source_files.as_ref().map(|f| serde_json::to_string(f).unwrap_or_default());

        if keywords_json.is_some() { sets.push(format!("keywords = ?{}", param_count)); param_count += 1; }
        if tags_json.is_some() { sets.push(format!("tags = ?{}", param_count)); param_count += 1; }
        if source_files_json.is_some() { sets.push(format!("source_files = ?{}", param_count)); param_count += 1; }

        let sql = format!("UPDATE memories SET {} WHERE id = ?{}", sets.join(", "), param_count);

        // Build params dynamically using rusqlite::types::ToSql
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        params.push(Box::new(now));
        if let Some(ref v) = p.title { params.push(Box::new(v.clone())); }
        if let Some(ref v) = p.content { params.push(Box::new(v.clone())); }
        if let Some(ref v) = p.context { params.push(Box::new(v.clone())); }
        if let Some(v) = p.importance { params.push(Box::new(v)); }
        if let Some(ref v) = p.source_repo { params.push(Box::new(v.clone())); }
        if let Some(ref v) = p.source_url { params.push(Box::new(v.clone())); }
        if let Some(v) = p.confidence { params.push(Box::new(v)); }
        if let Some(ref v) = p.container_tag { params.push(Box::new(v.clone())); }
        if let Some(ref v) = keywords_json { params.push(Box::new(v.clone())); }
        if let Some(ref v) = tags_json { params.push(Box::new(v.clone())); }
        if let Some(ref v) = source_files_json { params.push(Box::new(v.clone())); }
        params.push(Box::new(p.memory_id));

        let params_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, params_refs.as_slice())?;

        drop(conn);
        self.log_activity("memory", p.memory_id, "update", None)?;
        self.get_memory_with_links_inner(p.memory_id)
    }

    pub fn delete_memory(&self, memory_id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        self.log_activity_inner(&conn, "memory", memory_id, "delete", None)?;
        conn.execute("DELETE FROM memories WHERE id = ?1", params![memory_id])?;
        Ok(())
    }

    pub fn link_memories(&self, memory_id: i64, related_ids: &[i64]) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let user_id = Self::default_user_id();
        for rid in related_ids {
            conn.execute(
                "INSERT OR IGNORE INTO memory_links (user_id, source_id, target_id) VALUES (?1, ?2, ?3)",
                params![user_id, memory_id, rid],
            )?;
            // Also insert reverse
            conn.execute(
                "INSERT OR IGNORE INTO memory_links (user_id, source_id, target_id) VALUES (?1, ?2, ?3)",
                params![user_id, rid, memory_id],
            )?;
        }
        Ok(())
    }

    pub fn unlink_memories(&self, memory_id: i64, related_ids: &[i64]) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        for rid in related_ids {
            conn.execute(
                "DELETE FROM memory_links WHERE (source_id = ?1 AND target_id = ?2) OR (source_id = ?2 AND target_id = ?1)",
                params![memory_id, rid],
            )?;
        }
        Ok(())
    }

    pub fn mark_obsolete(&self, memory_id: i64, reason: &str, superseded_by: Option<i64>) -> Result<MemoryWithLinks> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().naive_utc();
        conn.execute(
            "UPDATE memories SET is_obsolete = 1, obsolete_reason = ?1, superseded_by = ?2, obsoleted_at = ?3, updated_at = ?3 WHERE id = ?4",
            params![reason, superseded_by, now, memory_id],
        )?;
        drop(conn);
        self.log_activity("memory", memory_id, "mark_obsolete", None)?;
        self.get_memory_with_links_inner(memory_id)
    }

    pub fn get_recent_memories(&self, limit: i32, project_ids: &[i64], container_tag: Option<&str>) -> Result<Vec<MemoryWithLinks>> {
        let conn = self.conn.lock().unwrap();
        let mut sql = "SELECT id FROM memories WHERE is_obsolete = 0 AND is_forgotten = 0".to_string();

        if container_tag.is_some() {
            sql.push_str(" AND container_tag = ?2");
        }

        sql.push_str(" ORDER BY updated_at DESC LIMIT ?1");

        let mut stmt = conn.prepare(&sql)?;
        let ids: Vec<i64> = if let Some(ct) = container_tag {
            stmt.query_map(params![limit, ct], |row| row.get(0))?
                .filter_map(|r| r.ok())
                .collect()
        } else {
            stmt.query_map(params![limit], |row| row.get(0))?
                .filter_map(|r| r.ok())
                .collect()
        };
        drop(stmt);
        drop(conn);

        let mut results = Vec::new();
        for id in ids {
            if let Ok(mem) = self.get_memory_with_links_inner(id) {
                // Filter by project if needed
                if !project_ids.is_empty() {
                    if !project_ids.iter().any(|pid| mem.project_ids.contains(pid)) {
                        continue;
                    }
                }
                results.push(mem);
            }
        }
        Ok(results)
    }

    pub fn create_version(&self, p: &CreateVersionParams) -> Result<MemoryWithLinks> {
        // Get parent version info
        let conn = self.conn.lock().unwrap();
        let (parent_version, parent_user_id): (i32, String) = conn.query_row(
            "SELECT version, user_id FROM memories WHERE id = ?1",
            params![p.parent_memory_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).map_err(|_| anyhow::anyhow!("Parent memory not found: {}", p.parent_memory_id))?;

        let now = Utc::now().naive_utc();
        let new_version = parent_version + 1;
        let keywords_json = serde_json::to_string(&p.keywords)?;
        let tags_json = serde_json::to_string(&p.tags)?;

        // Mark parent as not latest
        conn.execute(
            "UPDATE memories SET is_latest = 0, updated_at = ?1 WHERE id = ?2",
            params![now, p.parent_memory_id],
        )?;

        // Create new version
        conn.execute(
            "INSERT INTO memories (user_id, title, content, context, keywords, tags, importance,
             version, parent_memory_id, is_latest, relationship_type, memory_type,
             created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 1, ?10, ?11, ?12, ?13)",
            params![
                parent_user_id, p.title, p.content, p.context, keywords_json, tags_json, p.importance,
                new_version, p.parent_memory_id, p.relationship_type, p.memory_type, now, now
            ],
        )?;
        let new_id = conn.last_insert_rowid();

        // Copy associations from parent
        conn.execute(
            "INSERT OR IGNORE INTO memory_project_association (memory_id, project_id)
             SELECT ?1, project_id FROM memory_project_association WHERE memory_id = ?2",
            params![new_id, p.parent_memory_id],
        )?;
        conn.execute(
            "INSERT OR IGNORE INTO memory_entity_association (memory_id, entity_id)
             SELECT ?1, entity_id FROM memory_entity_association WHERE memory_id = ?2",
            params![new_id, p.parent_memory_id],
        )?;

        drop(conn);
        self.log_activity("memory", new_id, "create_version", None)?;
        self.get_memory_with_links_inner(new_id)
    }

    pub fn get_version_chain(&self, memory_id: i64) -> Result<Vec<MemoryWithLinks>> {
        let conn = self.conn.lock().unwrap();

        // Find root of chain (walk up parent_memory_id)
        let mut root_id = memory_id;
        loop {
            let parent: Option<i64> = conn.query_row(
                "SELECT parent_memory_id FROM memories WHERE id = ?1",
                params![root_id],
                |row| row.get(0),
            ).map_err(|_| anyhow::anyhow!("Memory not found: {}", root_id))?;
            match parent {
                Some(pid) => root_id = pid,
                None => break,
            }
        }

        // Walk down from root using recursive CTE
        let mut stmt = conn.prepare(
            "WITH RECURSIVE chain(id) AS (
                SELECT ?1
                UNION ALL
                SELECT m.id FROM memories m JOIN chain c ON m.parent_memory_id = c.id
            )
            SELECT id FROM chain ORDER BY id"
        )?;
        let ids: Vec<i64> = stmt.query_map(params![root_id], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();
        drop(stmt);
        drop(conn);

        let mut chain = Vec::new();
        for id in ids {
            if let Ok(mem) = self.get_memory_with_links_inner(id) {
                chain.push(mem);
            }
        }
        Ok(chain)
    }

    pub fn forget_memory(&self, memory_id: i64, reason: Option<&str>) -> Result<MemoryWithLinks> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().naive_utc();
        conn.execute(
            "UPDATE memories SET is_forgotten = 1, forgotten_at = ?1, obsolete_reason = ?2, updated_at = ?1 WHERE id = ?3",
            params![now, reason, memory_id],
        )?;
        drop(conn);
        self.log_activity("memory", memory_id, "forget", None)?;
        self.get_memory_with_links_inner(memory_id)
    }

    pub fn search_similar(&self, memory_id: i64, k: i32) -> Result<Vec<MemoryWithLinks>> {
        // Get the memory's text for FTS search
        let mem = self.get_memory_with_links_inner(memory_id)?;
        let search_text = format!("{} {} {}", mem.memory.title, mem.memory.content, mem.memory.keywords.join(" "));
        let fts_query = build_fts_query(&search_text, None);

        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT m.id FROM memories_fts fts
             JOIN memories m ON m.id = fts.rowid
             WHERE memories_fts MATCH ?1
             AND m.is_obsolete = 0 AND m.is_forgotten = 0
             AND m.id != ?2
             ORDER BY bm25(memories_fts) * (CAST(m.importance AS REAL) / 10.0)
             LIMIT ?3"
        )?;
        let ids: Vec<i64> = stmt.query_map(params![fts_query, memory_id, k], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();
        drop(stmt);
        drop(conn);

        let mut results = Vec::new();
        for id in ids {
            if let Ok(mem) = self.get_memory_with_links_inner(id) {
                results.push(mem);
            }
        }
        Ok(results)
    }

    fn auto_link_memory(&self, memory_id: i64) -> Result<()> {
        // Find top 3 similar memories and create bidirectional links
        let similar = self.search_similar(memory_id, 3)?;
        if !similar.is_empty() {
            let ids: Vec<i64> = similar.iter().map(|m| m.memory.id).collect();
            self.link_memories(memory_id, &ids)?;
        }
        Ok(())
    }

    /// Process auto-forget: mark expired memories as forgotten
    pub fn process_auto_forget(&self) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().naive_utc();
        let count = conn.execute(
            "UPDATE memories SET is_forgotten = 1, forgotten_at = ?1, updated_at = ?1
             WHERE forget_after IS NOT NULL AND forget_after <= ?1 AND is_forgotten = 0",
            params![now],
        )?;
        Ok(count)
    }
}

/// Build an FTS5 query from natural language input
fn build_fts_query(query: &str, context: Option<&str>) -> String {
    let mut terms: Vec<String> = Vec::new();

    // Extract meaningful words (3+ chars), skip common stop words
    let stop_words = ["the", "and", "for", "are", "but", "not", "you", "all",
        "can", "had", "her", "was", "one", "our", "out", "has", "have",
        "with", "this", "that", "from", "they", "been", "said", "each",
        "which", "their", "will", "other", "about", "many", "then",
        "them", "these", "some", "would", "make", "like", "into",
        "could", "time", "very", "when", "what", "your", "how"];

    let combined = if let Some(ctx) = context {
        format!("{} {}", query, ctx)
    } else {
        query.to_string()
    };

    for word in combined.split_whitespace() {
        let clean: String = word.chars()
            .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
            .collect();
        let lower = clean.to_lowercase();
        if lower.len() >= 2 && !stop_words.contains(&lower.as_str()) {
            terms.push(format!("\"{}\"", clean));
        }
    }

    if terms.is_empty() {
        // Fallback: use original query words
        for word in query.split_whitespace() {
            let clean: String = word.chars().filter(|c| c.is_alphanumeric()).collect();
            if !clean.is_empty() {
                terms.push(format!("\"{}\"", clean));
            }
        }
    }

    if terms.is_empty() {
        return "\"*\"".to_string();
    }

    // Use OR to be more permissive (better recall)
    terms.join(" OR ")
}
