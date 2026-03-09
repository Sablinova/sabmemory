mod db;
mod server;
mod types;
mod dashboard;

use std::path::PathBuf;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "sabmemory", version, about = "Lightweight MCP memory server")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the MCP server over stdio
    Serve,
    /// Start the web dashboard
    Dashboard {
        /// Port to listen on
        #[arg(long, default_value = "3080")]
        port: u16,
        /// Host to bind to
        #[arg(long, default_value = "0.0.0.0")]
        host: String,
    },
    /// Migrate data from a forgetful-ai database
    Migrate {
        /// Path to the forgetful-ai SQLite database
        #[arg(long)]
        from: PathBuf,
    },
}

fn data_path() -> PathBuf {
    let data_dir = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("sabmemory");
    data_dir.join("sabmemory.db")
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Log to stderr only (stdout is MCP protocol)
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("sabmemory=info".parse()?)
        )
        .with_writer(std::io::stderr)
        .with_target(false)
        .init();

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Migrate { from }) => {
            migrate::run_migration(&from, &data_path())?;
        }
        Some(Commands::Dashboard { port, host }) => {
            dashboard::run_dashboard(&data_path(), &host, port).await?;
        }
        Some(Commands::Serve) | None => {
            run_server().await?;
        }
    }

    Ok(())
}

async fn run_server() -> anyhow::Result<()> {
    use rmcp::ServiceExt;

    let db_path = data_path();
    tracing::info!("Database: {}", db_path.display());

    let database = db::Database::new(&db_path)?;
    let server = server::SabMemoryServer::new(database);

    let transport = rmcp::transport::io::stdio();
    let ct: rmcp::service::RunningService<_, _> = server.serve(transport).await?;
    ct.waiting().await?;

    Ok(())
}

mod migrate {
    use std::path::Path;
    use anyhow::Result;
    use rusqlite::{params, Connection};
    use chrono::Utc;

    pub fn run_migration(from_path: &Path, to_path: &Path) -> Result<()> {
        if !from_path.exists() {
            anyhow::bail!("Source database not found: {}", from_path.display());
        }

        eprintln!("Migrating from: {}", from_path.display());
        eprintln!("Migrating to:   {}", to_path.display());

        // Open destination (creates schema via Database::new)
        let dest_db = crate::db::Database::new(&to_path.to_path_buf())?;

        // Open source directly
        let src = Connection::open(from_path)?;

        // Migrate projects
        let project_count = migrate_projects(&src, &dest_db)?;
        eprintln!("Migrated {} projects", project_count);

        // Migrate entities
        let entity_count = migrate_entities(&src, &dest_db)?;
        eprintln!("Migrated {} entities", entity_count);

        // Migrate memories
        let memory_count = migrate_memories(&src, &dest_db)?;
        eprintln!("Migrated {} memories", memory_count);

        // Migrate documents
        let doc_count = migrate_documents(&src, &dest_db)?;
        eprintln!("Migrated {} documents", doc_count);

        // Migrate code artifacts
        let code_count = migrate_code_artifacts(&src, &dest_db)?;
        eprintln!("Migrated {} code artifacts", code_count);

        // Migrate entity relationships
        let rel_count = migrate_relationships(&src, &dest_db)?;
        eprintln!("Migrated {} entity relationships", rel_count);

        // Migrate associations
        let assoc_count = migrate_associations(&src, &dest_db)?;
        eprintln!("Migrated {} associations", assoc_count);

        // Rebuild FTS indexes
        crate::db::schema::rebuild_fts(&dest_db)?;
        eprintln!("FTS indexes rebuilt");

        eprintln!("Migration complete!");
        Ok(())
    }

    fn migrate_projects(src: &Connection, dest_db: &crate::db::Database) -> Result<usize> {
        let dest = dest_db.conn.lock().unwrap();
        let mut stmt = src.prepare(
            "SELECT id, user_id, name, description, project_type, status, repo_name, notes, created_at, updated_at FROM projects"
        )?;

        let mut count = 0;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1).unwrap_or_else(|_| "default".to_string()),
                row.get::<_, String>(2)?,
                row.get::<_, String>(3).unwrap_or_default(),
                row.get::<_, Option<String>>(4)?,
                row.get::<_, String>(5).unwrap_or_else(|_| "active".to_string()),
                row.get::<_, Option<String>>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, String>(8)?,
                row.get::<_, String>(9)?,
            ))
        })?;

        for row in rows {
            let (id, _user_id, name, desc, ptype, status, repo, notes, created, updated) = row?;
            dest.execute(
                "INSERT OR IGNORE INTO projects (id, user_id, name, description, project_type, status, repo_name, notes, created_at, updated_at)
                 VALUES (?1, 'default', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![id, name, desc, ptype, status, repo, notes, created, updated],
            )?;
            count += 1;
        }
        Ok(count)
    }

    fn migrate_entities(src: &Connection, dest_db: &crate::db::Database) -> Result<usize> {
        let dest = dest_db.conn.lock().unwrap();

        // Check if source has entities table
        let has_entities: bool = src.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='entities'",
            [], |row| row.get::<_, i32>(0),
        ).map(|c| c > 0).unwrap_or(false);

        if !has_entities { return Ok(0); }

        let mut stmt = src.prepare(
            "SELECT id, user_id, name, entity_type, custom_type, notes, tags, aka, created_at, updated_at FROM entities"
        )?;

        let mut count = 0;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1).unwrap_or_else(|_| "default".to_string()),
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, String>(6).unwrap_or_else(|_| "[]".to_string()),
                row.get::<_, String>(7).unwrap_or_else(|_| "[]".to_string()),
                row.get::<_, String>(8)?,
                row.get::<_, String>(9)?,
            ))
        })?;

        for row in rows {
            let (id, _user_id, name, etype, ctype, notes, tags, aka, created, updated) = row?;
            dest.execute(
                "INSERT OR IGNORE INTO entities (id, user_id, name, entity_type, custom_type, notes, tags, aka, created_at, updated_at)
                 VALUES (?1, 'default', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![id, name, etype, ctype, notes, tags, aka, created, updated],
            )?;
            count += 1;
        }
        Ok(count)
    }

    fn migrate_memories(src: &Connection, dest_db: &crate::db::Database) -> Result<usize> {
        let dest = dest_db.conn.lock().unwrap();
        let _now = Utc::now().naive_utc();

        // forgetful-ai memories schema may differ — map what we can
        let mut stmt = src.prepare(
            "SELECT id, user_id, title, content, context, keywords, tags, importance,
             is_obsolete, obsolete_reason, superseded_by, obsoleted_at,
             source_repo, source_files, source_url, confidence, encoding_agent,
             created_at, updated_at
             FROM memories"
        )?;

        let mut count = 0;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1).unwrap_or_else(|_| "default".to_string()),
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4).unwrap_or_default(),
                row.get::<_, String>(5).unwrap_or_else(|_| "[]".to_string()),
                row.get::<_, String>(6).unwrap_or_else(|_| "[]".to_string()),
                row.get::<_, i32>(7).unwrap_or(5),
                row.get::<_, i32>(8).unwrap_or(0),
                row.get::<_, Option<String>>(9)?,
                row.get::<_, Option<i64>>(10)?,
                row.get::<_, Option<String>>(11)?,
                row.get::<_, Option<String>>(12)?,
                row.get::<_, Option<String>>(13)?,
                row.get::<_, Option<String>>(14)?,
                row.get::<_, Option<f64>>(15)?,
                row.get::<_, Option<String>>(16)?,
                row.get::<_, String>(17)?,
                row.get::<_, String>(18)?,
            ))
        })?;

        for row in rows {
            let (id, _uid, title, content, context, keywords, tags, importance,
                 is_obsolete, obs_reason, superseded, obs_at,
                 src_repo, src_files, src_url, confidence, enc_agent,
                 created, updated) = row?;

            dest.execute(
                "INSERT OR IGNORE INTO memories (id, user_id, title, content, context, keywords, tags, importance,
                 is_obsolete, obsolete_reason, superseded_by, obsoleted_at,
                 source_repo, source_files, source_url, confidence, encoding_agent,
                 version, is_latest, memory_type, created_at, updated_at)
                 VALUES (?1, 'default', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, 1, 1, 'fact', ?17, ?18)",
                params![id, title, content, context, keywords, tags, importance,
                    is_obsolete, obs_reason, superseded, obs_at,
                    src_repo, src_files, src_url, confidence, enc_agent,
                    created, updated],
            )?;
            count += 1;
        }
        Ok(count)
    }

    fn migrate_documents(src: &Connection, dest_db: &crate::db::Database) -> Result<usize> {
        let dest = dest_db.conn.lock().unwrap();

        let has_docs: bool = src.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='documents'",
            [], |row| row.get::<_, i32>(0),
        ).map(|c| c > 0).unwrap_or(false);

        if !has_docs { return Ok(0); }

        let mut stmt = src.prepare(
            "SELECT id, user_id, project_id, title, description, content, document_type, filename, size_bytes, tags, created_at, updated_at FROM documents"
        )?;

        let mut count = 0;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, Option<i64>>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4).unwrap_or_default(),
                row.get::<_, String>(5).unwrap_or_default(),
                row.get::<_, Option<String>>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, i64>(8).unwrap_or(0),
                row.get::<_, String>(9).unwrap_or_else(|_| "[]".to_string()),
                row.get::<_, String>(10)?,
                row.get::<_, String>(11)?,
            ))
        })?;

        for row in rows {
            let (id, project_id, title, desc, content, dtype, filename, size, tags, created, updated) = row?;
            dest.execute(
                "INSERT OR IGNORE INTO documents (id, user_id, project_id, title, description, content, document_type, filename, size_bytes, tags, created_at, updated_at)
                 VALUES (?1, 'default', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![id, project_id, title, desc, content, dtype, filename, size, tags, created, updated],
            )?;
            count += 1;
        }
        Ok(count)
    }

    fn migrate_code_artifacts(src: &Connection, dest_db: &crate::db::Database) -> Result<usize> {
        let dest = dest_db.conn.lock().unwrap();

        let has_table: bool = src.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='code_artifacts'",
            [], |row| row.get::<_, i32>(0),
        ).map(|c| c > 0).unwrap_or(false);

        if !has_table { return Ok(0); }

        let mut stmt = src.prepare(
            "SELECT id, project_id, title, description, code, language, tags, created_at, updated_at FROM code_artifacts"
        )?;

        let mut count = 0;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, Option<i64>>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3).unwrap_or_default(),
                row.get::<_, String>(4).unwrap_or_default(),
                row.get::<_, String>(5).unwrap_or_default(),
                row.get::<_, String>(6).unwrap_or_else(|_| "[]".to_string()),
                row.get::<_, String>(7)?,
                row.get::<_, String>(8)?,
            ))
        })?;

        for row in rows {
            let (id, project_id, title, desc, code, lang, tags, created, updated) = row?;
            dest.execute(
                "INSERT OR IGNORE INTO code_artifacts (id, user_id, project_id, title, description, code, language, tags, created_at, updated_at)
                 VALUES (?1, 'default', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![id, project_id, title, desc, code, lang, tags, created, updated],
            )?;
            count += 1;
        }
        Ok(count)
    }

    fn migrate_relationships(src: &Connection, dest_db: &crate::db::Database) -> Result<usize> {
        let dest = dest_db.conn.lock().unwrap();

        let has_table: bool = src.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='entity_relationships'",
            [], |row| row.get::<_, i32>(0),
        ).map(|c| c > 0).unwrap_or(false);

        if !has_table { return Ok(0); }

        let mut stmt = src.prepare(
            "SELECT source_entity_id, target_entity_id, relationship_type, strength, confidence, relationship_metadata, created_at, updated_at
             FROM entity_relationships"
        )?;

        let mut count = 0;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<f64>>(3)?,
                row.get::<_, Option<f64>>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
            ))
        })?;

        for row in rows {
            let (src_id, tgt_id, rtype, strength, confidence, meta, created, updated) = row?;
            dest.execute(
                "INSERT OR IGNORE INTO entity_relationships (user_id, source_entity_id, target_entity_id, relationship_type, strength, confidence, relationship_metadata, created_at, updated_at)
                 VALUES ('default', ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![src_id, tgt_id, rtype, strength, confidence, meta, created, updated],
            )?;
            count += 1;
        }
        Ok(count)
    }

    fn migrate_associations(src: &Connection, dest_db: &crate::db::Database) -> Result<usize> {
        let dest = dest_db.conn.lock().unwrap();
        let mut total = 0;

        // memory_project_association
        if table_exists(src, "memory_project_association") {
            let mut stmt = src.prepare("SELECT memory_id, project_id FROM memory_project_association")?;
            let rows = stmt.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)))?;
            for row in rows {
                let (mid, pid) = row?;
                let _ = dest.execute(
                    "INSERT OR IGNORE INTO memory_project_association (memory_id, project_id) VALUES (?1, ?2)",
                    params![mid, pid],
                );
                total += 1;
            }
        }

        // memory_entity_association
        if table_exists(src, "memory_entity_association") {
            let mut stmt = src.prepare("SELECT memory_id, entity_id FROM memory_entity_association")?;
            let rows = stmt.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)))?;
            for row in rows {
                let (mid, eid) = row?;
                let _ = dest.execute(
                    "INSERT OR IGNORE INTO memory_entity_association (memory_id, entity_id) VALUES (?1, ?2)",
                    params![mid, eid],
                );
                total += 1;
            }
        }

        // memory_document_association
        if table_exists(src, "memory_document_association") {
            let mut stmt = src.prepare("SELECT memory_id, document_id FROM memory_document_association")?;
            let rows = stmt.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)))?;
            for row in rows {
                let (mid, did) = row?;
                let _ = dest.execute(
                    "INSERT OR IGNORE INTO memory_document_association (memory_id, document_id) VALUES (?1, ?2)",
                    params![mid, did],
                );
                total += 1;
            }
        }

        // memory_code_artifact_association
        if table_exists(src, "memory_code_artifact_association") {
            let mut stmt = src.prepare("SELECT memory_id, code_artifact_id FROM memory_code_artifact_association")?;
            let rows = stmt.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)))?;
            for row in rows {
                let (mid, cid) = row?;
                let _ = dest.execute(
                    "INSERT OR IGNORE INTO memory_code_artifact_association (memory_id, code_artifact_id) VALUES (?1, ?2)",
                    params![mid, cid],
                );
                total += 1;
            }
        }

        // entity_project_association
        if table_exists(src, "entity_project_association") {
            let mut stmt = src.prepare("SELECT entity_id, project_id FROM entity_project_association")?;
            let rows = stmt.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)))?;
            for row in rows {
                let (eid, pid) = row?;
                let _ = dest.execute(
                    "INSERT OR IGNORE INTO entity_project_association (entity_id, project_id) VALUES (?1, ?2)",
                    params![eid, pid],
                );
                total += 1;
            }
        }

        // memory_links
        if table_exists(src, "memory_links") {
            let mut stmt = src.prepare("SELECT source_id, target_id FROM memory_links")?;
            let rows = stmt.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)))?;
            for row in rows {
                let (sid, tid) = row?;
                let _ = dest.execute(
                    "INSERT OR IGNORE INTO memory_links (user_id, source_id, target_id) VALUES ('default', ?1, ?2)",
                    params![sid, tid],
                );
                total += 1;
            }
        }

        Ok(total)
    }

    fn table_exists(conn: &Connection, table_name: &str) -> bool {
        conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
            params![table_name],
            |row| row.get::<_, i32>(0),
        ).map(|c| c > 0).unwrap_or(false)
    }
}
