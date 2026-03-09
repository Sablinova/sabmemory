use std::path::PathBuf;
use std::sync::Arc;
use axum::{
    Router,
    Json,
    extract::{State, Query},
    response::Html,
    http::{StatusCode, header},
    routing::get,
};
use tower_http::cors::CorsLayer;
use serde::Deserialize;

use crate::db::Database;

type AppState = Arc<Database>;

pub async fn run_dashboard(db_path: &PathBuf, host: &str, port: u16) -> anyhow::Result<()> {
    let database = Database::new(db_path)?;
    let state: AppState = Arc::new(database);

    let app = Router::new()
        // Frontend
        .route("/", get(index_page))
        .route("/style.css", get(css))
        .route("/app.js", get(js))
        // API
        .route("/api/stats", get(api_stats))
        .route("/api/memories", get(api_memories))
        .route("/api/memory/{id}", get(api_memory))
        .route("/api/entities", get(api_entities))
        .route("/api/relationships", get(api_relationships))
        .route("/api/projects", get(api_projects))
        .route("/api/documents", get(api_documents))
        .route("/api/graph", get(api_graph))
        .route("/api/search", get(api_search))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = format!("{}:{}", host, port);
    tracing::info!("Dashboard at http://{}", addr);
    eprintln!("sabmemory dashboard running at http://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

// ─── Frontend Routes ───

async fn index_page() -> Html<&'static str> {
    Html(include_str!("../dashboard/index.html"))
}

async fn css() -> (StatusCode, [(header::HeaderName, &'static str); 1], &'static str) {
    (StatusCode::OK, [(header::CONTENT_TYPE, "text/css")], include_str!("../dashboard/style.css"))
}

async fn js() -> (StatusCode, [(header::HeaderName, &'static str); 1], &'static str) {
    (StatusCode::OK, [(header::CONTENT_TYPE, "application/javascript")], include_str!("../dashboard/app.js"))
}

// ─── API Routes ───

async fn api_stats(State(db): State<AppState>) -> Result<Json<serde_json::Value>, StatusCode> {
    let conn = db.conn.lock().unwrap();

    let memory_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM memories WHERE user_id='default' AND is_obsolete=0 AND is_forgotten=0", [], |r| r.get(0)
    ).unwrap_or(0);
    let entity_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM entities WHERE user_id='default'", [], |r| r.get(0)
    ).unwrap_or(0);
    let project_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM projects WHERE user_id='default'", [], |r| r.get(0)
    ).unwrap_or(0);
    let document_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM documents WHERE user_id='default'", [], |r| r.get(0)
    ).unwrap_or(0);
    let code_artifact_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM code_artifacts WHERE user_id='default'", [], |r| r.get(0)
    ).unwrap_or(0);
    let relationship_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM entity_relationships WHERE user_id='default'", [], |r| r.get(0)
    ).unwrap_or(0);
    let link_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM memory_links WHERE user_id='default'", [], |r| r.get(0)
    ).unwrap_or(0);
    let obsolete_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM memories WHERE user_id='default' AND is_obsolete=1", [], |r| r.get(0)
    ).unwrap_or(0);
    let forgotten_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM memories WHERE user_id='default' AND is_forgotten=1", [], |r| r.get(0)
    ).unwrap_or(0);

    Ok(Json(serde_json::json!({
        "memories": memory_count,
        "entities": entity_count,
        "projects": project_count,
        "documents": document_count,
        "code_artifacts": code_artifact_count,
        "relationships": relationship_count,
        "memory_links": link_count,
        "obsolete": obsolete_count,
        "forgotten": forgotten_count
    })))
}

async fn api_memories(State(db): State<AppState>) -> Result<Json<serde_json::Value>, StatusCode> {
    let conn = db.conn.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, title, content, context, keywords, tags, importance, memory_type, container_tag,
                is_obsolete, is_forgotten, version, is_latest, created_at, updated_at
         FROM memories WHERE user_id='default'
         ORDER BY updated_at DESC"
    ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let rows: Vec<serde_json::Value> = stmt.query_map([], |row| {
        Ok(serde_json::json!({
            "id": row.get::<_, i64>(0)?,
            "title": row.get::<_, String>(1)?,
            "content": row.get::<_, String>(2)?,
            "context": row.get::<_, String>(3)?,
            "keywords": serde_json::from_str::<serde_json::Value>(&row.get::<_, String>(4)?).unwrap_or_default(),
            "tags": serde_json::from_str::<serde_json::Value>(&row.get::<_, String>(5)?).unwrap_or_default(),
            "importance": row.get::<_, i32>(6)?,
            "memory_type": row.get::<_, String>(7)?,
            "container_tag": row.get::<_, Option<String>>(8)?,
            "is_obsolete": row.get::<_, bool>(9)?,
            "is_forgotten": row.get::<_, bool>(10)?,
            "version": row.get::<_, i32>(11)?,
            "is_latest": row.get::<_, bool>(12)?,
            "created_at": row.get::<_, String>(13)?,
            "updated_at": row.get::<_, String>(14)?,
        }))
    }).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .filter_map(|r| r.ok())
    .collect();

    Ok(Json(serde_json::json!(rows)))
}

async fn api_memory(
    State(db): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let conn = db.conn.lock().unwrap();

    let mem = conn.query_row(
        "SELECT id, title, content, context, keywords, tags, importance, memory_type, container_tag,
                is_obsolete, obsolete_reason, is_forgotten, version, parent_memory_id, is_latest,
                relationship_type, source_repo, source_url, confidence, encoding_agent,
                created_at, updated_at
         FROM memories WHERE id=?1",
        [id],
        |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "title": row.get::<_, String>(1)?,
                "content": row.get::<_, String>(2)?,
                "context": row.get::<_, String>(3)?,
                "keywords": serde_json::from_str::<serde_json::Value>(&row.get::<_, String>(4)?).unwrap_or_default(),
                "tags": serde_json::from_str::<serde_json::Value>(&row.get::<_, String>(5)?).unwrap_or_default(),
                "importance": row.get::<_, i32>(6)?,
                "memory_type": row.get::<_, String>(7)?,
                "container_tag": row.get::<_, Option<String>>(8)?,
                "is_obsolete": row.get::<_, bool>(9)?,
                "obsolete_reason": row.get::<_, Option<String>>(10)?,
                "is_forgotten": row.get::<_, bool>(11)?,
                "version": row.get::<_, i32>(12)?,
                "parent_memory_id": row.get::<_, Option<i64>>(13)?,
                "is_latest": row.get::<_, bool>(14)?,
                "relationship_type": row.get::<_, Option<String>>(15)?,
                "source_repo": row.get::<_, Option<String>>(16)?,
                "source_url": row.get::<_, Option<String>>(17)?,
                "confidence": row.get::<_, Option<f64>>(18)?,
                "encoding_agent": row.get::<_, Option<String>>(19)?,
                "created_at": row.get::<_, String>(20)?,
                "updated_at": row.get::<_, String>(21)?,
            }))
        }
    ).map_err(|_| StatusCode::NOT_FOUND)?;

    // Get links
    let mut link_stmt = conn.prepare(
        "SELECT target_id FROM memory_links WHERE source_id=?1
         UNION SELECT source_id FROM memory_links WHERE target_id=?1"
    ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let links: Vec<i64> = link_stmt.query_map([id], |row| row.get(0))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .filter_map(|r| r.ok()).collect();

    // Get project IDs
    let mut proj_stmt = conn.prepare("SELECT project_id FROM memory_project_association WHERE memory_id=?1")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let projects: Vec<i64> = proj_stmt.query_map([id], |row| row.get(0))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .filter_map(|r| r.ok()).collect();

    // Get entity IDs
    let mut ent_stmt = conn.prepare("SELECT entity_id FROM memory_entity_association WHERE memory_id=?1")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let entities: Vec<i64> = ent_stmt.query_map([id], |row| row.get(0))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .filter_map(|r| r.ok()).collect();

    let mut result = mem;
    result["linked_memory_ids"] = serde_json::json!(links);
    result["project_ids"] = serde_json::json!(projects);
    result["entity_ids"] = serde_json::json!(entities);

    Ok(Json(result))
}

async fn api_entities(State(db): State<AppState>) -> Result<Json<serde_json::Value>, StatusCode> {
    let conn = db.conn.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, name, entity_type, custom_type, notes, tags, aka, created_at
         FROM entities WHERE user_id='default' ORDER BY name"
    ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let rows: Vec<serde_json::Value> = stmt.query_map([], |row| {
        Ok(serde_json::json!({
            "id": row.get::<_, i64>(0)?,
            "name": row.get::<_, String>(1)?,
            "entity_type": row.get::<_, String>(2)?,
            "custom_type": row.get::<_, Option<String>>(3)?,
            "notes": row.get::<_, Option<String>>(4)?,
            "tags": serde_json::from_str::<serde_json::Value>(&row.get::<_, String>(5)?).unwrap_or_default(),
            "aka": serde_json::from_str::<serde_json::Value>(&row.get::<_, String>(6)?).unwrap_or_default(),
            "created_at": row.get::<_, String>(7)?,
        }))
    }).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .filter_map(|r| r.ok())
    .collect();

    Ok(Json(serde_json::json!(rows)))
}

async fn api_relationships(State(db): State<AppState>) -> Result<Json<serde_json::Value>, StatusCode> {
    let conn = db.conn.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT er.id, er.source_entity_id, er.target_entity_id, er.relationship_type,
                er.strength, er.confidence, e1.name as source_name, e2.name as target_name
         FROM entity_relationships er
         JOIN entities e1 ON er.source_entity_id = e1.id
         JOIN entities e2 ON er.target_entity_id = e2.id
         WHERE er.user_id='default'"
    ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let rows: Vec<serde_json::Value> = stmt.query_map([], |row| {
        Ok(serde_json::json!({
            "id": row.get::<_, i64>(0)?,
            "source_entity_id": row.get::<_, i64>(1)?,
            "target_entity_id": row.get::<_, i64>(2)?,
            "relationship_type": row.get::<_, String>(3)?,
            "strength": row.get::<_, Option<f64>>(4)?,
            "confidence": row.get::<_, Option<f64>>(5)?,
            "source_name": row.get::<_, String>(6)?,
            "target_name": row.get::<_, String>(7)?,
        }))
    }).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .filter_map(|r| r.ok())
    .collect();

    Ok(Json(serde_json::json!(rows)))
}

async fn api_projects(State(db): State<AppState>) -> Result<Json<serde_json::Value>, StatusCode> {
    let conn = db.conn.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT p.id, p.name, p.description, p.project_type, p.status, p.repo_name,
                (SELECT COUNT(*) FROM memory_project_association WHERE project_id=p.id) as memory_count,
                p.created_at
         FROM projects p WHERE p.user_id='default' ORDER BY p.name"
    ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let rows: Vec<serde_json::Value> = stmt.query_map([], |row| {
        Ok(serde_json::json!({
            "id": row.get::<_, i64>(0)?,
            "name": row.get::<_, String>(1)?,
            "description": row.get::<_, String>(2)?,
            "project_type": row.get::<_, Option<String>>(3)?,
            "status": row.get::<_, String>(4)?,
            "repo_name": row.get::<_, Option<String>>(5)?,
            "memory_count": row.get::<_, i64>(6)?,
            "created_at": row.get::<_, String>(7)?,
        }))
    }).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .filter_map(|r| r.ok())
    .collect();

    Ok(Json(serde_json::json!(rows)))
}

async fn api_documents(State(db): State<AppState>) -> Result<Json<serde_json::Value>, StatusCode> {
    let conn = db.conn.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, title, description, document_type, size_bytes, tags, created_at
         FROM documents WHERE user_id='default' ORDER BY updated_at DESC"
    ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let rows: Vec<serde_json::Value> = stmt.query_map([], |row| {
        Ok(serde_json::json!({
            "id": row.get::<_, i64>(0)?,
            "title": row.get::<_, String>(1)?,
            "description": row.get::<_, String>(2)?,
            "document_type": row.get::<_, Option<String>>(3)?,
            "size_bytes": row.get::<_, i64>(4)?,
            "tags": serde_json::from_str::<serde_json::Value>(&row.get::<_, String>(5)?).unwrap_or_default(),
            "created_at": row.get::<_, String>(6)?,
        }))
    }).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .filter_map(|r| r.ok())
    .collect();

    Ok(Json(serde_json::json!(rows)))
}

// Knowledge graph data for d3-force visualization
async fn api_graph(State(db): State<AppState>) -> Result<Json<serde_json::Value>, StatusCode> {
    let conn = db.conn.lock().unwrap();
    let mut nodes: Vec<serde_json::Value> = Vec::new();
    let mut edges: Vec<serde_json::Value> = Vec::new();

    // Memory nodes
    let mut mem_stmt = conn.prepare(
        "SELECT id, title, importance, memory_type, is_obsolete, is_forgotten, tags
         FROM memories WHERE user_id='default'
         ORDER BY importance DESC"
    ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mems: Vec<_> = mem_stmt.query_map([], |row| {
        Ok(serde_json::json!({
            "id": format!("m{}", row.get::<_, i64>(0)?),
            "raw_id": row.get::<_, i64>(0)?,
            "label": row.get::<_, String>(1)?,
            "type": "memory",
            "importance": row.get::<_, i32>(2)?,
            "memory_type": row.get::<_, String>(3)?,
            "is_obsolete": row.get::<_, bool>(4)?,
            "is_forgotten": row.get::<_, bool>(5)?,
            "tags": serde_json::from_str::<serde_json::Value>(&row.get::<_, String>(6)?).unwrap_or_default(),
        }))
    }).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .filter_map(|r| r.ok())
    .collect();
    nodes.extend(mems);

    // Entity nodes
    let mut ent_stmt = conn.prepare(
        "SELECT id, name, entity_type FROM entities WHERE user_id='default'"
    ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let ents: Vec<_> = ent_stmt.query_map([], |row| {
        Ok(serde_json::json!({
            "id": format!("e{}", row.get::<_, i64>(0)?),
            "raw_id": row.get::<_, i64>(0)?,
            "label": row.get::<_, String>(1)?,
            "type": "entity",
            "entity_type": row.get::<_, String>(2)?,
        }))
    }).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .filter_map(|r| r.ok())
    .collect();
    nodes.extend(ents);

    // Project nodes
    let mut proj_stmt = conn.prepare(
        "SELECT id, name FROM projects WHERE user_id='default'"
    ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let projs: Vec<_> = proj_stmt.query_map([], |row| {
        Ok(serde_json::json!({
            "id": format!("p{}", row.get::<_, i64>(0)?),
            "raw_id": row.get::<_, i64>(0)?,
            "label": row.get::<_, String>(1)?,
            "type": "project",
        }))
    }).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .filter_map(|r| r.ok())
    .collect();
    nodes.extend(projs);

    // Memory-memory links
    let mut link_stmt = conn.prepare(
        "SELECT source_id, target_id FROM memory_links WHERE user_id='default'"
    ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mem_links: Vec<_> = link_stmt.query_map([], |row| {
        Ok(serde_json::json!({
            "source": format!("m{}", row.get::<_, i64>(0)?),
            "target": format!("m{}", row.get::<_, i64>(1)?),
            "type": "memory_link",
        }))
    }).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .filter_map(|r| r.ok())
    .collect();
    edges.extend(mem_links);

    // Memory-entity associations
    let mut me_stmt = conn.prepare(
        "SELECT memory_id, entity_id FROM memory_entity_association"
    ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let me_links: Vec<_> = me_stmt.query_map([], |row| {
        Ok(serde_json::json!({
            "source": format!("m{}", row.get::<_, i64>(0)?),
            "target": format!("e{}", row.get::<_, i64>(1)?),
            "type": "entity_assoc",
        }))
    }).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .filter_map(|r| r.ok())
    .collect();
    edges.extend(me_links);

    // Memory-project associations
    let mut mp_stmt = conn.prepare(
        "SELECT memory_id, project_id FROM memory_project_association"
    ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mp_links: Vec<_> = mp_stmt.query_map([], |row| {
        Ok(serde_json::json!({
            "source": format!("m{}", row.get::<_, i64>(0)?),
            "target": format!("p{}", row.get::<_, i64>(1)?),
            "type": "project_assoc",
        }))
    }).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .filter_map(|r| r.ok())
    .collect();
    edges.extend(mp_links);

    // Entity-entity relationships
    let mut er_stmt = conn.prepare(
        "SELECT source_entity_id, target_entity_id, relationship_type
         FROM entity_relationships WHERE user_id='default'"
    ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let er_links: Vec<_> = er_stmt.query_map([], |row| {
        Ok(serde_json::json!({
            "source": format!("e{}", row.get::<_, i64>(0)?),
            "target": format!("e{}", row.get::<_, i64>(1)?),
            "type": "relationship",
            "label": row.get::<_, String>(2)?,
        }))
    }).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .filter_map(|r| r.ok())
    .collect();
    edges.extend(er_links);

    // Entity-project associations
    let mut ep_stmt = conn.prepare(
        "SELECT entity_id, project_id FROM entity_project_association"
    ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let ep_links: Vec<_> = ep_stmt.query_map([], |row| {
        Ok(serde_json::json!({
            "source": format!("e{}", row.get::<_, i64>(0)?),
            "target": format!("p{}", row.get::<_, i64>(1)?),
            "type": "entity_project",
        }))
    }).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .filter_map(|r| r.ok())
    .collect();
    edges.extend(ep_links);

    Ok(Json(serde_json::json!({
        "nodes": nodes,
        "edges": edges
    })))
}

#[derive(Deserialize)]
struct SearchQuery {
    q: String,
}

async fn api_search(
    State(db): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let conn = db.conn.lock().unwrap();

    let query = params.q.trim();
    if query.is_empty() {
        return Ok(Json(serde_json::json!([])));
    }

    // Build FTS query — strip non-alphanumeric, OR-join tokens
    let tokens: Vec<String> = query
        .split_whitespace()
        .map(|w| w.chars().filter(|c| c.is_alphanumeric()).collect::<String>())
        .filter(|w| !w.is_empty())
        .map(|w| format!("\"{}\"", w))
        .collect();

    if tokens.is_empty() {
        return Ok(Json(serde_json::json!([])));
    }

    let fts_query = tokens.join(" OR ");

    let mut stmt = conn.prepare(
        "SELECT m.id, m.title, m.content, m.importance, m.memory_type, m.tags,
                bm25(memories_fts, 5.0, 3.0, 2.0, 1.0, 1.0) as rank
         FROM memories_fts fts
         JOIN memories m ON fts.rowid = m.id
         WHERE memories_fts MATCH ?1
           AND m.user_id='default' AND m.is_obsolete=0 AND m.is_forgotten=0 AND m.is_latest=1
         ORDER BY (rank * (1.0 + m.importance * 0.1))
         LIMIT 20"
    ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let rows: Vec<serde_json::Value> = stmt.query_map([&fts_query], |row| {
        Ok(serde_json::json!({
            "id": row.get::<_, i64>(0)?,
            "title": row.get::<_, String>(1)?,
            "content": row.get::<_, String>(2)?,
            "importance": row.get::<_, i32>(3)?,
            "memory_type": row.get::<_, String>(4)?,
            "tags": serde_json::from_str::<serde_json::Value>(&row.get::<_, String>(5)?).unwrap_or_default(),
        }))
    }).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .filter_map(|r| r.ok())
    .collect();

    Ok(Json(serde_json::json!(rows)))
}
