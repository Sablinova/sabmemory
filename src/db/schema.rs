use super::Database;
use anyhow::Result;

pub fn initialize(db: &Database) -> Result<()> {
    let conn = db.conn.lock().unwrap();

    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY NOT NULL,
            external_id TEXT NOT NULL DEFAULT 'default',
            name TEXT NOT NULL DEFAULT 'default',
            email TEXT NOT NULL DEFAULT '',
            idp_metadata TEXT,
            notes TEXT,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        );

        INSERT OR IGNORE INTO users (id, external_id, name) VALUES ('default', 'default', 'default');

        CREATE TABLE IF NOT EXISTS projects (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            name TEXT NOT NULL,
            description TEXT NOT NULL DEFAULT '',
            project_type TEXT,
            status TEXT NOT NULL DEFAULT 'active',
            repo_name TEXT,
            notes TEXT,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE INDEX IF NOT EXISTS idx_projects_user_id ON projects(user_id);
        CREATE INDEX IF NOT EXISTS idx_projects_status ON projects(status);

        CREATE TABLE IF NOT EXISTS memories (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            title TEXT NOT NULL,
            content TEXT NOT NULL,
            context TEXT NOT NULL DEFAULT '',
            keywords TEXT NOT NULL DEFAULT '[]',
            tags TEXT NOT NULL DEFAULT '[]',
            importance INTEGER NOT NULL DEFAULT 5,
            is_obsolete INTEGER NOT NULL DEFAULT 0,
            obsolete_reason TEXT,
            superseded_by INTEGER REFERENCES memories(id) ON DELETE SET NULL,
            obsoleted_at DATETIME,
            source_repo TEXT,
            source_files TEXT,
            source_url TEXT,
            confidence REAL,
            encoding_agent TEXT,
            encoding_version TEXT,
            version INTEGER NOT NULL DEFAULT 1,
            parent_memory_id INTEGER REFERENCES memories(id) ON DELETE SET NULL,
            is_latest INTEGER NOT NULL DEFAULT 1,
            relationship_type TEXT,
            forget_after DATETIME,
            is_forgotten INTEGER NOT NULL DEFAULT 0,
            forgotten_at DATETIME,
            memory_type TEXT NOT NULL DEFAULT 'fact',
            container_tag TEXT,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE INDEX IF NOT EXISTS idx_memories_user_id ON memories(user_id);
        CREATE INDEX IF NOT EXISTS idx_memories_importance ON memories(importance);
        CREATE INDEX IF NOT EXISTS idx_memories_is_obsolete ON memories(is_obsolete);
        CREATE INDEX IF NOT EXISTS idx_memories_is_forgotten ON memories(is_forgotten);
        CREATE INDEX IF NOT EXISTS idx_memories_is_latest ON memories(is_latest);
        CREATE INDEX IF NOT EXISTS idx_memories_parent_memory_id ON memories(parent_memory_id);
        CREATE INDEX IF NOT EXISTS idx_memories_container_tag ON memories(container_tag);
        CREATE INDEX IF NOT EXISTS idx_memories_forget_after ON memories(forget_after);
        CREATE INDEX IF NOT EXISTS idx_memories_confidence ON memories(confidence);

        CREATE TABLE IF NOT EXISTS entities (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            name TEXT NOT NULL,
            entity_type TEXT NOT NULL,
            custom_type TEXT,
            notes TEXT,
            tags TEXT NOT NULL DEFAULT '[]',
            aka TEXT NOT NULL DEFAULT '[]',
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE INDEX IF NOT EXISTS idx_entities_user_id ON entities(user_id);
        CREATE INDEX IF NOT EXISTS idx_entities_name ON entities(name);
        CREATE INDEX IF NOT EXISTS idx_entities_entity_type ON entities(entity_type);

        CREATE TABLE IF NOT EXISTS entity_relationships (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            source_entity_id INTEGER NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
            target_entity_id INTEGER NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
            relationship_type TEXT NOT NULL,
            strength REAL,
            confidence REAL,
            relationship_metadata TEXT,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(source_entity_id, target_entity_id, relationship_type)
        );
        CREATE INDEX IF NOT EXISTS idx_er_source ON entity_relationships(source_entity_id);
        CREATE INDEX IF NOT EXISTS idx_er_target ON entity_relationships(target_entity_id);
        CREATE INDEX IF NOT EXISTS idx_er_type ON entity_relationships(relationship_type);

        CREATE TABLE IF NOT EXISTS documents (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            project_id INTEGER REFERENCES projects(id) ON DELETE SET NULL,
            title TEXT NOT NULL,
            description TEXT NOT NULL DEFAULT '',
            content TEXT NOT NULL DEFAULT '',
            document_type TEXT,
            filename TEXT,
            size_bytes INTEGER NOT NULL DEFAULT 0,
            tags TEXT NOT NULL DEFAULT '[]',
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE INDEX IF NOT EXISTS idx_documents_project_id ON documents(project_id);

        CREATE TABLE IF NOT EXISTS code_artifacts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            project_id INTEGER REFERENCES projects(id) ON DELETE SET NULL,
            title TEXT NOT NULL,
            description TEXT NOT NULL DEFAULT '',
            code TEXT NOT NULL DEFAULT '',
            language TEXT NOT NULL DEFAULT '',
            tags TEXT NOT NULL DEFAULT '[]',
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE INDEX IF NOT EXISTS idx_code_artifacts_project_id ON code_artifacts(project_id);
        CREATE INDEX IF NOT EXISTS idx_code_artifacts_language ON code_artifacts(language);

        -- Association tables
        CREATE TABLE IF NOT EXISTS memory_project_association (
            memory_id INTEGER NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
            project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            PRIMARY KEY (memory_id, project_id)
        );

        CREATE TABLE IF NOT EXISTS memory_entity_association (
            memory_id INTEGER NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
            entity_id INTEGER NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
            PRIMARY KEY (memory_id, entity_id)
        );

        CREATE TABLE IF NOT EXISTS memory_document_association (
            memory_id INTEGER NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
            document_id INTEGER NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
            PRIMARY KEY (memory_id, document_id)
        );

        CREATE TABLE IF NOT EXISTS memory_code_artifact_association (
            memory_id INTEGER NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
            code_artifact_id INTEGER NOT NULL REFERENCES code_artifacts(id) ON DELETE CASCADE,
            PRIMARY KEY (memory_id, code_artifact_id)
        );

        CREATE TABLE IF NOT EXISTS entity_project_association (
            entity_id INTEGER NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
            project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            PRIMARY KEY (entity_id, project_id)
        );

        CREATE TABLE IF NOT EXISTS memory_links (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            source_id INTEGER NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
            target_id INTEGER NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(source_id, target_id)
        );
        CREATE INDEX IF NOT EXISTS idx_memory_links_target ON memory_links(target_id, source_id);

        -- Activity log
        CREATE TABLE IF NOT EXISTS activity_log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            entity_type TEXT NOT NULL,
            entity_id INTEGER NOT NULL,
            action TEXT NOT NULL,
            changes TEXT,
            snapshot TEXT NOT NULL DEFAULT '{}',
            actor TEXT NOT NULL DEFAULT 'agent',
            actor_id TEXT,
            metadata TEXT,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE INDEX IF NOT EXISTS idx_activity_user_entity ON activity_log(user_id, entity_type, entity_id);
        CREATE INDEX IF NOT EXISTS idx_activity_created ON activity_log(user_id, created_at);

        -- User profiles (supermemory)
        CREATE TABLE IF NOT EXISTS user_profiles (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            static_facts TEXT NOT NULL DEFAULT '[]',
            dynamic_facts TEXT NOT NULL DEFAULT '[]',
            generated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        );

        -- FTS5 virtual tables
        CREATE VIRTUAL TABLE IF NOT EXISTS memories_fts USING fts5(
            title, content, context, keywords, tags,
            content='memories',
            content_rowid='id',
            tokenize='porter unicode61'
        );

        CREATE VIRTUAL TABLE IF NOT EXISTS entities_fts USING fts5(
            name, notes, aka,
            content='entities',
            content_rowid='id',
            tokenize='porter unicode61'
        );

        CREATE VIRTUAL TABLE IF NOT EXISTS documents_fts USING fts5(
            title, description, content,
            content='documents',
            content_rowid='id',
            tokenize='porter unicode61'
        );

        CREATE VIRTUAL TABLE IF NOT EXISTS code_artifacts_fts USING fts5(
            title, description, code,
            content='code_artifacts',
            content_rowid='id',
            tokenize='porter unicode61'
        );

        -- Triggers to keep FTS in sync
        CREATE TRIGGER IF NOT EXISTS memories_ai AFTER INSERT ON memories BEGIN
            INSERT INTO memories_fts(rowid, title, content, context, keywords, tags)
            VALUES (new.id, new.title, new.content, new.context, new.keywords, new.tags);
        END;

        CREATE TRIGGER IF NOT EXISTS memories_ad AFTER DELETE ON memories BEGIN
            INSERT INTO memories_fts(memories_fts, rowid, title, content, context, keywords, tags)
            VALUES ('delete', old.id, old.title, old.content, old.context, old.keywords, old.tags);
        END;

        CREATE TRIGGER IF NOT EXISTS memories_au AFTER UPDATE ON memories BEGIN
            INSERT INTO memories_fts(memories_fts, rowid, title, content, context, keywords, tags)
            VALUES ('delete', old.id, old.title, old.content, old.context, old.keywords, old.tags);
            INSERT INTO memories_fts(rowid, title, content, context, keywords, tags)
            VALUES (new.id, new.title, new.content, new.context, new.keywords, new.tags);
        END;

        CREATE TRIGGER IF NOT EXISTS entities_ai AFTER INSERT ON entities BEGIN
            INSERT INTO entities_fts(rowid, name, notes, aka)
            VALUES (new.id, new.name, COALESCE(new.notes, ''), new.aka);
        END;

        CREATE TRIGGER IF NOT EXISTS entities_ad AFTER DELETE ON entities BEGIN
            INSERT INTO entities_fts(entities_fts, rowid, name, notes, aka)
            VALUES ('delete', old.id, old.name, COALESCE(old.notes, ''), old.aka);
        END;

        CREATE TRIGGER IF NOT EXISTS entities_au AFTER UPDATE ON entities BEGIN
            INSERT INTO entities_fts(entities_fts, rowid, name, notes, aka)
            VALUES ('delete', old.id, old.name, COALESCE(old.notes, ''), old.aka);
            INSERT INTO entities_fts(rowid, name, notes, aka)
            VALUES (new.id, new.name, COALESCE(new.notes, ''), new.aka);
        END;

        CREATE TRIGGER IF NOT EXISTS documents_ai AFTER INSERT ON documents BEGIN
            INSERT INTO documents_fts(rowid, title, description, content)
            VALUES (new.id, new.title, new.description, new.content);
        END;

        CREATE TRIGGER IF NOT EXISTS documents_ad AFTER DELETE ON documents BEGIN
            INSERT INTO documents_fts(documents_fts, rowid, title, description, content)
            VALUES ('delete', old.id, old.title, old.description, old.content);
        END;

        CREATE TRIGGER IF NOT EXISTS documents_au AFTER UPDATE ON documents BEGIN
            INSERT INTO documents_fts(documents_fts, rowid, title, description, content)
            VALUES ('delete', old.id, old.title, old.description, old.content);
            INSERT INTO documents_fts(rowid, title, description, content)
            VALUES (new.id, new.title, new.description, new.content);
        END;

        CREATE TRIGGER IF NOT EXISTS code_artifacts_ai AFTER INSERT ON code_artifacts BEGIN
            INSERT INTO code_artifacts_fts(rowid, title, description, code)
            VALUES (new.id, new.title, new.description, new.code);
        END;

        CREATE TRIGGER IF NOT EXISTS code_artifacts_ad AFTER DELETE ON code_artifacts BEGIN
            INSERT INTO code_artifacts_fts(code_artifacts_fts, rowid, title, description, code)
            VALUES ('delete', old.id, old.title, old.description, old.code);
        END;

        CREATE TRIGGER IF NOT EXISTS code_artifacts_au AFTER UPDATE ON code_artifacts BEGIN
            INSERT INTO code_artifacts_fts(code_artifacts_fts, rowid, title, description, code)
            VALUES ('delete', old.id, old.title, old.description, old.code);
            INSERT INTO code_artifacts_fts(rowid, title, description, code)
            VALUES (new.id, new.title, new.description, new.code);
        END;
        "
    )?;

    Ok(())
}

/// Rebuild FTS indexes from scratch (used after migration)
pub fn rebuild_fts(db: &Database) -> Result<()> {
    let conn = db.conn.lock().unwrap();
    conn.execute_batch(
        "
        INSERT INTO memories_fts(memories_fts) VALUES('rebuild');
        INSERT INTO entities_fts(entities_fts) VALUES('rebuild');
        INSERT INTO documents_fts(documents_fts) VALUES('rebuild');
        INSERT INTO code_artifacts_fts(code_artifacts_fts) VALUES('rebuild');
        "
    )?;
    Ok(())
}
