use std::sync::Arc;
use rmcp::{
    ErrorData as McpError,
    ServerHandler, 
    handler::server::router::tool::ToolRouter,
    model::*,
    tool, tool_handler, tool_router,
};
use rmcp::handler::server::wrapper::Parameters;

use crate::db::Database;
use crate::types::*;

#[derive(Clone)]
pub struct SabMemoryServer {
    db: Arc<Database>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl SabMemoryServer {
    pub fn new(db: Database) -> Self {
        let db = Arc::new(db);
        Self {
            db,
            tool_router: Self::tool_router(),
        }
    }

    // ═══════════════════════════════════════════════════
    // MEMORY TOOLS (13)
    // ═══════════════════════════════════════════════════

    /// Create a new atomic memory. Memories should represent a single concept, decision, or fact.
    /// Auto-links to similar existing memories via FTS5 text similarity.
    /// Fields: title (max 200 chars), content (max 2000 chars), context (max 500 chars, explains WHY this matters),
    /// keywords (max 10, for search clustering), tags (max 10, for categorization), importance (1-10).
    /// Optional: project_ids, entity_ids, document_ids, code_artifact_ids for associations.
    /// Optional: source_repo, source_files, source_url, confidence, encoding_agent for provenance.
    /// Optional: memory_type (fact/preference/episode), container_tag (for scoping), forget_after (ISO date for auto-expiry).
    #[tool(name = "create_memory")]
    async fn create_memory(&self, Parameters(p): Parameters<CreateMemoryParams>) -> Result<CallToolResult, McpError> {
        match self.db.create_memory(&p) {
            Ok(mem) => ok_json(&mem),
            Err(e) => tool_error(&format!("Failed to create memory: {}", e)),
        }
    }

    /// Search memories using natural language queries. Uses FTS5 full-text search with BM25 ranking,
    /// boosted by importance score. Results are bounded by a token budget (default 8000).
    /// Supports filtering by project_ids, tags, min_importance, and container_tag.
    /// When include_links is true, linked memories are included (up to max_links_per_primary per result).
    #[tool(name = "query_memory")]
    async fn query_memory(&self, Parameters(p): Parameters<QueryMemoryParams>) -> Result<CallToolResult, McpError> {
        // Process auto-forget before searching
        let _ = self.db.process_auto_forget();
        match self.db.query_memories(&p) {
            Ok(results) => ok_json(&results),
            Err(e) => tool_error(&format!("Failed to query memories: {}", e)),
        }
    }

    /// Get a specific memory by ID, including all linked memory IDs, project IDs, entity IDs,
    /// document IDs, and code artifact IDs.
    #[tool(name = "get_memory")]
    async fn get_memory(&self, Parameters(p): Parameters<GetMemoryParams>) -> Result<CallToolResult, McpError> {
        match self.db.get_memory_with_links_inner(p.memory_id) {
            Ok(mem) => ok_json(&mem),
            Err(e) => tool_error(&format!("Memory not found: {}", e)),
        }
    }

    /// Update specific fields of an existing memory. Only provided fields are updated (PATCH semantics).
    /// Updates the FTS index automatically via triggers.
    #[tool(name = "update_memory")]
    async fn update_memory(&self, Parameters(p): Parameters<UpdateMemoryParams>) -> Result<CallToolResult, McpError> {
        match self.db.update_memory(&p) {
            Ok(mem) => ok_json(&mem),
            Err(e) => tool_error(&format!("Failed to update memory: {}", e)),
        }
    }

    /// Permanently delete a memory and all its associations. This action cannot be undone.
    /// Consider using mark_obsolete or forget_memory for soft deletion instead.
    #[tool(name = "delete_memory")]
    async fn delete_memory(&self, Parameters(p): Parameters<DeleteMemoryParams>) -> Result<CallToolResult, McpError> {
        match self.db.delete_memory(p.memory_id) {
            Ok(()) => ok_text(&format!("Memory {} deleted", p.memory_id)),
            Err(e) => tool_error(&format!("Failed to delete memory: {}", e)),
        }
    }

    /// Create bidirectional links between memories. Links are used for knowledge graph traversal
    /// and are included when querying with include_links=true.
    #[tool(name = "link_memories")]
    async fn link_memories(&self, Parameters(p): Parameters<LinkMemoriesParams>) -> Result<CallToolResult, McpError> {
        match self.db.link_memories(p.memory_id, &p.related_ids) {
            Ok(()) => ok_text(&format!("Linked memory {} to {:?}", p.memory_id, p.related_ids)),
            Err(e) => tool_error(&format!("Failed to link memories: {}", e)),
        }
    }

    /// Remove bidirectional links between memories.
    #[tool(name = "unlink_memories")]
    async fn unlink_memories(&self, Parameters(p): Parameters<UnlinkMemoriesParams>) -> Result<CallToolResult, McpError> {
        match self.db.unlink_memories(p.memory_id, &p.related_ids) {
            Ok(()) => ok_text(&format!("Unlinked memory {} from {:?}", p.memory_id, p.related_ids)),
            Err(e) => tool_error(&format!("Failed to unlink memories: {}", e)),
        }
    }

    /// Mark a memory as obsolete (soft delete with audit trail). The memory remains in the database
    /// but is excluded from search results. Optionally specify superseded_by to link to the replacement.
    #[tool(name = "mark_obsolete")]
    async fn mark_obsolete(&self, Parameters(p): Parameters<MarkObsoleteParams>) -> Result<CallToolResult, McpError> {
        match self.db.mark_obsolete(p.memory_id, &p.reason, p.superseded_by) {
            Ok(mem) => ok_json(&mem),
            Err(e) => tool_error(&format!("Failed to mark obsolete: {}", e)),
        }
    }

    /// Get recently updated memories, ordered by update time (newest first).
    /// Useful for reviewing what was captured in previous sessions.
    /// Supports filtering by project_ids and container_tag.
    #[tool(name = "get_recent")]
    async fn get_recent(&self, Parameters(p): Parameters<GetRecentParams>) -> Result<CallToolResult, McpError> {
        let _ = self.db.process_auto_forget();
        match self.db.get_recent_memories(p.limit, &p.project_ids, p.container_tag.as_deref()) {
            Ok(mems) => ok_json(&mems),
            Err(e) => tool_error(&format!("Failed to get recent memories: {}", e)),
        }
    }

    /// Create a new version of an existing memory, forming a version chain (supermemory-inspired).
    /// The parent memory is marked as is_latest=false, and the new version gets is_latest=true.
    /// relationship_type must be one of: updates (contradicts/replaces), extends (enriches), derives (system-inferred).
    /// Associations (project, entity links) are copied from the parent.
    #[tool(name = "create_version")]
    async fn create_version(&self, Parameters(p): Parameters<CreateVersionParams>) -> Result<CallToolResult, McpError> {
        match self.db.create_version(&p) {
            Ok(mem) => ok_json(&mem),
            Err(e) => tool_error(&format!("Failed to create version: {}", e)),
        }
    }

    /// Get the complete version chain for a memory. Walks up to the root (first version)
    /// and then returns all versions in order, showing how the knowledge evolved over time.
    #[tool(name = "get_version_chain")]
    async fn get_version_chain(&self, Parameters(p): Parameters<GetVersionChainParams>) -> Result<CallToolResult, McpError> {
        match self.db.get_version_chain(p.memory_id) {
            Ok(chain) => ok_json(&chain),
            Err(e) => tool_error(&format!("Failed to get version chain: {}", e)),
        }
    }

    /// Soft-forget a memory (supermemory-inspired). Different from mark_obsolete:
    /// - obsolete = information is outdated/replaced
    /// - forgotten = information is no longer relevant/needed
    /// Forgotten memories are excluded from search but remain in the database for audit.
    #[tool(name = "forget_memory")]
    async fn forget_memory(&self, Parameters(p): Parameters<ForgetMemoryParams>) -> Result<CallToolResult, McpError> {
        match self.db.forget_memory(p.memory_id, p.reason.as_deref()) {
            Ok(mem) => ok_json(&mem),
            Err(e) => tool_error(&format!("Failed to forget memory: {}", e)),
        }
    }

    /// Find memories that are textually similar to a given memory using FTS5.
    /// Useful for manual linking or discovering related knowledge.
    #[tool(name = "search_similar")]
    async fn search_similar(&self, Parameters(p): Parameters<SearchSimilarParams>) -> Result<CallToolResult, McpError> {
        match self.db.search_similar(p.memory_id, p.k) {
            Ok(mems) => ok_json(&mems),
            Err(e) => tool_error(&format!("Failed to search similar: {}", e)),
        }
    }

    // ═══════════════════════════════════════════════════
    // ENTITY TOOLS (15)
    // ═══════════════════════════════════════════════════

    /// Create a new entity (person, organization, device, concept, etc.).
    /// Entities form the nodes of the knowledge graph. Use aka for aliases.
    #[tool(name = "create_entity")]
    async fn create_entity(&self, Parameters(p): Parameters<CreateEntityParams>) -> Result<CallToolResult, McpError> {
        match self.db.create_entity(&p) {
            Ok(ent) => ok_json(&ent),
            Err(e) => tool_error(&format!("Failed to create entity: {}", e)),
        }
    }

    /// Get a specific entity by ID.
    #[tool(name = "get_entity")]
    async fn get_entity(&self, Parameters(p): Parameters<GetEntityParams>) -> Result<CallToolResult, McpError> {
        match self.db.get_entity(p.entity_id) {
            Ok(ent) => ok_json(&ent),
            Err(e) => tool_error(&format!("Entity not found: {}", e)),
        }
    }

    /// List all entities, optionally filtered by project IDs or entity type.
    #[tool(name = "list_entities")]
    async fn list_entities(&self, Parameters(p): Parameters<ListEntitiesParams>) -> Result<CallToolResult, McpError> {
        match self.db.list_entities(&p.project_ids, p.entity_type.as_deref()) {
            Ok(ents) => ok_json(&ents),
            Err(e) => tool_error(&format!("Failed to list entities: {}", e)),
        }
    }

    /// Search entities by name and aliases using full-text search.
    #[tool(name = "search_entities")]
    async fn search_entities(&self, Parameters(p): Parameters<SearchEntitiesParams>) -> Result<CallToolResult, McpError> {
        match self.db.search_entities(&p.query) {
            Ok(ents) => ok_json(&ents),
            Err(e) => tool_error(&format!("Failed to search entities: {}", e)),
        }
    }

    /// Update specific fields of an existing entity (PATCH semantics).
    #[tool(name = "update_entity")]
    async fn update_entity(&self, Parameters(p): Parameters<UpdateEntityParams>) -> Result<CallToolResult, McpError> {
        match self.db.update_entity(&p) {
            Ok(ent) => ok_json(&ent),
            Err(e) => tool_error(&format!("Failed to update entity: {}", e)),
        }
    }

    /// Delete an entity and all its associations (relationships, memory links, project links).
    #[tool(name = "delete_entity")]
    async fn delete_entity(&self, Parameters(p): Parameters<DeleteEntityParams>) -> Result<CallToolResult, McpError> {
        match self.db.delete_entity(p.entity_id) {
            Ok(()) => ok_text(&format!("Entity {} deleted", p.entity_id)),
            Err(e) => tool_error(&format!("Failed to delete entity: {}", e)),
        }
    }

    /// Link an entity to a memory. Creates a bidirectional association.
    #[tool(name = "link_entity_memory")]
    async fn link_entity_memory(&self, Parameters(p): Parameters<LinkEntityMemoryParams>) -> Result<CallToolResult, McpError> {
        match self.db.link_entity_memory(p.entity_id, p.memory_id) {
            Ok(()) => ok_text(&format!("Linked entity {} to memory {}", p.entity_id, p.memory_id)),
            Err(e) => tool_error(&format!("Failed to link: {}", e)),
        }
    }

    /// Remove the link between an entity and a memory.
    #[tool(name = "unlink_entity_memory")]
    async fn unlink_entity_memory(&self, Parameters(p): Parameters<LinkEntityMemoryParams>) -> Result<CallToolResult, McpError> {
        match self.db.unlink_entity_memory(p.entity_id, p.memory_id) {
            Ok(()) => ok_text(&format!("Unlinked entity {} from memory {}", p.entity_id, p.memory_id)),
            Err(e) => tool_error(&format!("Failed to unlink: {}", e)),
        }
    }

    /// Link an entity to a project. Creates an organizational association.
    #[tool(name = "link_entity_project")]
    async fn link_entity_project(&self, Parameters(p): Parameters<LinkEntityProjectParams>) -> Result<CallToolResult, McpError> {
        match self.db.link_entity_project(p.entity_id, p.project_id) {
            Ok(()) => ok_text(&format!("Linked entity {} to project {}", p.entity_id, p.project_id)),
            Err(e) => tool_error(&format!("Failed to link: {}", e)),
        }
    }

    /// Remove the link between an entity and a project.
    #[tool(name = "unlink_entity_project")]
    async fn unlink_entity_project(&self, Parameters(p): Parameters<LinkEntityProjectParams>) -> Result<CallToolResult, McpError> {
        match self.db.unlink_entity_project(p.entity_id, p.project_id) {
            Ok(()) => ok_text(&format!("Unlinked entity {} from project {}", p.entity_id, p.project_id)),
            Err(e) => tool_error(&format!("Failed to unlink: {}", e)),
        }
    }

    /// Get all memory IDs associated with an entity.
    #[tool(name = "get_entity_memories")]
    async fn get_entity_memories(&self, Parameters(p): Parameters<GetEntityMemoriesParams>) -> Result<CallToolResult, McpError> {
        match self.db.get_entity_memories(p.entity_id) {
            Ok(ids) => ok_json(&serde_json::json!({"memory_ids": ids, "count": ids.len()})),
            Err(e) => tool_error(&format!("Failed to get entity memories: {}", e)),
        }
    }

    /// Create a typed, directed relationship between two entities (e.g., works_for, owns, manages).
    /// Optional strength and confidence scores (0.0-1.0) and arbitrary metadata.
    #[tool(name = "create_relationship")]
    async fn create_relationship(&self, Parameters(p): Parameters<CreateRelationshipParams>) -> Result<CallToolResult, McpError> {
        match self.db.create_relationship(&p) {
            Ok(rel) => ok_json(&rel),
            Err(e) => tool_error(&format!("Failed to create relationship: {}", e)),
        }
    }

    /// Get relationships for an entity. Direction can be: outgoing, incoming, or both.
    #[tool(name = "get_relationships")]
    async fn get_relationships(&self, Parameters(p): Parameters<GetRelationshipsParams>) -> Result<CallToolResult, McpError> {
        match self.db.get_relationships(p.entity_id, &p.direction) {
            Ok(rels) => ok_json(&rels),
            Err(e) => tool_error(&format!("Failed to get relationships: {}", e)),
        }
    }

    /// Update a relationship's type, strength, confidence, or metadata.
    #[tool(name = "update_relationship")]
    async fn update_relationship(&self, Parameters(p): Parameters<UpdateRelationshipParams>) -> Result<CallToolResult, McpError> {
        match self.db.update_relationship(&p) {
            Ok(rel) => ok_json(&rel),
            Err(e) => tool_error(&format!("Failed to update relationship: {}", e)),
        }
    }

    /// Delete a relationship between entities.
    #[tool(name = "delete_relationship")]
    async fn delete_relationship(&self, Parameters(p): Parameters<DeleteRelationshipParams>) -> Result<CallToolResult, McpError> {
        match self.db.delete_relationship(p.relationship_id) {
            Ok(()) => ok_text(&format!("Relationship {} deleted", p.relationship_id)),
            Err(e) => tool_error(&format!("Failed to delete relationship: {}", e)),
        }
    }

    // ═══════════════════════════════════════════════════
    // PROJECT TOOLS (5)
    // ═══════════════════════════════════════════════════

    /// Create a new project container. Projects organize memories, entities, and artifacts.
    /// Set repo_name for Git repository association (e.g., "owner/repo").
    #[tool(name = "create_project")]
    async fn create_project(&self, Parameters(p): Parameters<CreateProjectParams>) -> Result<CallToolResult, McpError> {
        match self.db.create_project(&p) {
            Ok(proj) => ok_json(&proj),
            Err(e) => tool_error(&format!("Failed to create project: {}", e)),
        }
    }

    /// Get a specific project by ID, including its memory count.
    #[tool(name = "get_project")]
    async fn get_project(&self, Parameters(p): Parameters<GetProjectParams>) -> Result<CallToolResult, McpError> {
        match self.db.get_project(p.project_id) {
            Ok(proj) => ok_json(&proj),
            Err(e) => tool_error(&format!("Project not found: {}", e)),
        }
    }

    /// List all projects, optionally filtered by repo_name or status.
    #[tool(name = "list_projects")]
    async fn list_projects(&self, Parameters(p): Parameters<ListProjectsParams>) -> Result<CallToolResult, McpError> {
        match self.db.list_projects(p.repo_name.as_deref(), p.status.as_deref()) {
            Ok(projs) => ok_json(&projs),
            Err(e) => tool_error(&format!("Failed to list projects: {}", e)),
        }
    }

    /// Update specific fields of a project (PATCH semantics).
    #[tool(name = "update_project")]
    async fn update_project(&self, Parameters(p): Parameters<UpdateProjectParams>) -> Result<CallToolResult, McpError> {
        match self.db.update_project(&p) {
            Ok(proj) => ok_json(&proj),
            Err(e) => tool_error(&format!("Failed to update project: {}", e)),
        }
    }

    /// Delete a project. Memories associated with this project are NOT deleted, only the association is removed.
    #[tool(name = "delete_project")]
    async fn delete_project(&self, Parameters(p): Parameters<DeleteProjectParams>) -> Result<CallToolResult, McpError> {
        match self.db.delete_project(p.project_id) {
            Ok(()) => ok_text(&format!("Project {} deleted", p.project_id)),
            Err(e) => tool_error(&format!("Failed to delete project: {}", e)),
        }
    }

    // ═══════════════════════════════════════════════════
    // DOCUMENT TOOLS (5)
    // ═══════════════════════════════════════════════════

    /// Create a document for storing long-form content that exceeds the 2000-char memory limit.
    /// Link documents to memories via memory document_ids for navigation.
    #[tool(name = "create_document")]
    async fn create_document(&self, Parameters(p): Parameters<CreateDocumentParams>) -> Result<CallToolResult, McpError> {
        match self.db.create_document(&p) {
            Ok(doc) => ok_json(&doc),
            Err(e) => tool_error(&format!("Failed to create document: {}", e)),
        }
    }

    /// Get a specific document by ID, including its full content.
    #[tool(name = "get_document")]
    async fn get_document(&self, Parameters(p): Parameters<GetDocumentParams>) -> Result<CallToolResult, McpError> {
        match self.db.get_document(p.document_id) {
            Ok(doc) => ok_json(&doc),
            Err(e) => tool_error(&format!("Document not found: {}", e)),
        }
    }

    /// List documents, optionally filtered by project_id or document_type.
    #[tool(name = "list_documents")]
    async fn list_documents(&self, Parameters(p): Parameters<ListDocumentsParams>) -> Result<CallToolResult, McpError> {
        match self.db.list_documents(p.project_id, p.document_type.as_deref()) {
            Ok(docs) => ok_json(&docs),
            Err(e) => tool_error(&format!("Failed to list documents: {}", e)),
        }
    }

    /// Update specific fields of a document (PATCH semantics).
    #[tool(name = "update_document")]
    async fn update_document(&self, Parameters(p): Parameters<UpdateDocumentParams>) -> Result<CallToolResult, McpError> {
        match self.db.update_document(&p) {
            Ok(doc) => ok_json(&doc),
            Err(e) => tool_error(&format!("Failed to update document: {}", e)),
        }
    }

    /// Delete a document permanently.
    #[tool(name = "delete_document")]
    async fn delete_document(&self, Parameters(p): Parameters<DeleteDocumentParams>) -> Result<CallToolResult, McpError> {
        match self.db.delete_document(p.document_id) {
            Ok(()) => ok_text(&format!("Document {} deleted", p.document_id)),
            Err(e) => tool_error(&format!("Failed to delete document: {}", e)),
        }
    }

    // ═══════════════════════════════════════════════════
    // CODE ARTIFACT TOOLS (5)
    // ═══════════════════════════════════════════════════

    /// Create a code artifact for storing reusable code snippets with language metadata.
    /// Link to memories via memory code_artifact_ids.
    #[tool(name = "create_code_artifact")]
    async fn create_code_artifact(&self, Parameters(p): Parameters<CreateCodeArtifactParams>) -> Result<CallToolResult, McpError> {
        match self.db.create_code_artifact(&p) {
            Ok(art) => ok_json(&art),
            Err(e) => tool_error(&format!("Failed to create code artifact: {}", e)),
        }
    }

    /// Get a specific code artifact by ID.
    #[tool(name = "get_code_artifact")]
    async fn get_code_artifact(&self, Parameters(p): Parameters<GetCodeArtifactParams>) -> Result<CallToolResult, McpError> {
        match self.db.get_code_artifact(p.code_artifact_id) {
            Ok(art) => ok_json(&art),
            Err(e) => tool_error(&format!("Code artifact not found: {}", e)),
        }
    }

    /// List code artifacts, optionally filtered by project_id or language.
    #[tool(name = "list_code_artifacts")]
    async fn list_code_artifacts(&self, Parameters(p): Parameters<ListCodeArtifactsParams>) -> Result<CallToolResult, McpError> {
        match self.db.list_code_artifacts(p.project_id, p.language.as_deref()) {
            Ok(arts) => ok_json(&arts),
            Err(e) => tool_error(&format!("Failed to list code artifacts: {}", e)),
        }
    }

    /// Update specific fields of a code artifact (PATCH semantics).
    #[tool(name = "update_code_artifact")]
    async fn update_code_artifact(&self, Parameters(p): Parameters<UpdateCodeArtifactParams>) -> Result<CallToolResult, McpError> {
        match self.db.update_code_artifact(&p) {
            Ok(art) => ok_json(&art),
            Err(e) => tool_error(&format!("Failed to update code artifact: {}", e)),
        }
    }

    /// Delete a code artifact permanently.
    #[tool(name = "delete_code_artifact")]
    async fn delete_code_artifact(&self, Parameters(p): Parameters<DeleteCodeArtifactParams>) -> Result<CallToolResult, McpError> {
        match self.db.delete_code_artifact(p.code_artifact_id) {
            Ok(()) => ok_text(&format!("Code artifact {} deleted", p.code_artifact_id)),
            Err(e) => tool_error(&format!("Failed to delete code artifact: {}", e)),
        }
    }

    // ═══════════════════════════════════════════════════
    // USER TOOLS (2)
    // ═══════════════════════════════════════════════════

    /// Get the current user's information.
    #[tool(name = "get_user")]
    async fn get_user(&self) -> Result<CallToolResult, McpError> {
        match self.db.get_user() {
            Ok(user) => ok_json(&user),
            Err(e) => tool_error(&format!("Failed to get user: {}", e)),
        }
    }

    /// Update the user's notes field.
    #[tool(name = "update_user_notes")]
    async fn update_user_notes(&self, Parameters(p): Parameters<UpdateUserNotesParams>) -> Result<CallToolResult, McpError> {
        match self.db.update_user_notes(&p.notes) {
            Ok(user) => ok_json(&user),
            Err(e) => tool_error(&format!("Failed to update user notes: {}", e)),
        }
    }

    // ═══════════════════════════════════════════════════
    // PROFILE TOOLS (2) — Supermemory-inspired
    // ═══════════════════════════════════════════════════

    /// Get the auto-generated user profile. Contains static_facts (persistent knowledge like identity,
    /// preferences, infrastructure) and dynamic_facts (recent context, episodes).
    /// Generated from high-importance memories. Fast retrieval (~1ms).
    #[tool(name = "get_profile")]
    async fn get_profile(&self) -> Result<CallToolResult, McpError> {
        match self.db.get_profile() {
            Ok(profile) => ok_json(&profile),
            Err(e) => tool_error(&format!("Failed to get profile: {}", e)),
        }
    }

    /// Regenerate the user profile from current memories. Call this after significant memory changes.
    #[tool(name = "refresh_profile")]
    async fn refresh_profile(&self) -> Result<CallToolResult, McpError> {
        match self.db.refresh_profile() {
            Ok(profile) => ok_json(&profile),
            Err(e) => tool_error(&format!("Failed to refresh profile: {}", e)),
        }
    }

    // ═══════════════════════════════════════════════════
    // META TOOLS (2)
    // ═══════════════════════════════════════════════════

    /// List all available sabmemory tools with their descriptions, organized by category.
    /// Use this to discover what tools are available.
    #[tool(name = "list_tools")]
    async fn list_all_tools(&self) -> Result<CallToolResult, McpError> {
        let tools = serde_json::json!({
            "memory": [
                {"name": "create_memory", "description": "Create a new atomic memory with auto-linking"},
                {"name": "query_memory", "description": "Search memories using natural language (FTS5 + BM25)"},
                {"name": "get_memory", "description": "Get a specific memory by ID with all associations"},
                {"name": "update_memory", "description": "Update memory fields (PATCH)"},
                {"name": "delete_memory", "description": "Permanently delete a memory"},
                {"name": "link_memories", "description": "Create bidirectional links between memories"},
                {"name": "unlink_memories", "description": "Remove links between memories"},
                {"name": "mark_obsolete", "description": "Soft-delete: mark memory as outdated"},
                {"name": "get_recent", "description": "Get recently updated memories"},
                {"name": "create_version", "description": "Create a versioned evolution of a memory"},
                {"name": "get_version_chain", "description": "Get full version history for a memory"},
                {"name": "forget_memory", "description": "Soft-forget: mark memory as no longer needed"},
                {"name": "search_similar", "description": "Find textually similar memories"}
            ],
            "entity": [
                {"name": "create_entity", "description": "Create person/org/device/concept"},
                {"name": "get_entity", "description": "Get entity by ID"},
                {"name": "list_entities", "description": "List entities with filters"},
                {"name": "search_entities", "description": "Search entities by name/aliases"},
                {"name": "update_entity", "description": "Update entity fields"},
                {"name": "delete_entity", "description": "Delete entity and associations"},
                {"name": "link_entity_memory", "description": "Link entity to memory"},
                {"name": "unlink_entity_memory", "description": "Unlink entity from memory"},
                {"name": "link_entity_project", "description": "Link entity to project"},
                {"name": "unlink_entity_project", "description": "Unlink entity from project"},
                {"name": "get_entity_memories", "description": "Get all memories for entity"},
                {"name": "create_relationship", "description": "Create typed edge between entities"},
                {"name": "get_relationships", "description": "Get entity relationships"},
                {"name": "update_relationship", "description": "Update relationship"},
                {"name": "delete_relationship", "description": "Delete relationship"}
            ],
            "project": [
                {"name": "create_project", "description": "Create project container"},
                {"name": "get_project", "description": "Get project by ID"},
                {"name": "list_projects", "description": "List projects with filters"},
                {"name": "update_project", "description": "Update project fields"},
                {"name": "delete_project", "description": "Delete project"}
            ],
            "document": [
                {"name": "create_document", "description": "Create long-form document"},
                {"name": "get_document", "description": "Get document by ID"},
                {"name": "list_documents", "description": "List documents with filters"},
                {"name": "update_document", "description": "Update document fields"},
                {"name": "delete_document", "description": "Delete document"}
            ],
            "code_artifact": [
                {"name": "create_code_artifact", "description": "Create reusable code snippet"},
                {"name": "get_code_artifact", "description": "Get code artifact by ID"},
                {"name": "list_code_artifacts", "description": "List code artifacts with filters"},
                {"name": "update_code_artifact", "description": "Update code artifact fields"},
                {"name": "delete_code_artifact", "description": "Delete code artifact"}
            ],
            "user": [
                {"name": "get_user", "description": "Get current user info"},
                {"name": "update_user_notes", "description": "Update user notes"}
            ],
            "profile": [
                {"name": "get_profile", "description": "Get auto-generated user profile"},
                {"name": "refresh_profile", "description": "Regenerate profile from memories"}
            ]
        });
        ok_json(&tools)
    }

    /// Get detailed information about a specific tool, including its parameter schema and usage examples.
    #[tool(name = "tool_info")]
    async fn tool_info(&self, Parameters(p): Parameters<ToolInfoParams>) -> Result<CallToolResult, McpError> {
        // Return the tool's description from the router
        match self.tool_router.get(&p.tool_name) {
            Some(tool) => ok_json(&serde_json::json!({
                "name": tool.name,
                "description": tool.description,
                "input_schema": tool.schema_as_json_value(),
            })),
            None => tool_error(&format!("Unknown tool: {}", p.tool_name)),
        }
    }
}

#[tool_handler]
impl ServerHandler for SabMemoryServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .build(),
        )
        .with_server_info(Implementation::new("sabmemory", env!("CARGO_PKG_VERSION")))
    }
}

// ─── Helper functions ───

fn ok_json<T: serde::Serialize>(value: &T) -> Result<CallToolResult, McpError> {
    let json = serde_json::to_string_pretty(value)
        .map_err(|e| McpError::internal_error(format!("Serialization error: {}", e), None))?;
    Ok(CallToolResult::success(vec![Content::text(json)]))
}

fn ok_text(msg: &str) -> Result<CallToolResult, McpError> {
    Ok(CallToolResult::success(vec![Content::text(msg.to_string())]))
}

fn tool_error(msg: &str) -> Result<CallToolResult, McpError> {
    Ok(CallToolResult::error(vec![Content::text(msg.to_string())]))
}
