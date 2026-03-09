use chrono::NaiveDateTime;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// ─── Memory ───

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub id: i64,
    pub user_id: String,
    pub title: String,
    pub content: String,
    pub context: String,
    pub keywords: Vec<String>,
    pub tags: Vec<String>,
    pub importance: i32,
    pub is_obsolete: bool,
    pub obsolete_reason: Option<String>,
    pub superseded_by: Option<i64>,
    pub obsoleted_at: Option<NaiveDateTime>,
    pub source_repo: Option<String>,
    pub source_files: Option<Vec<String>>,
    pub source_url: Option<String>,
    pub confidence: Option<f64>,
    pub encoding_agent: Option<String>,
    pub encoding_version: Option<String>,
    // Supermemory fields
    pub version: i32,
    pub parent_memory_id: Option<i64>,
    pub is_latest: bool,
    pub relationship_type: Option<String>,
    pub forget_after: Option<NaiveDateTime>,
    pub is_forgotten: bool,
    pub forgotten_at: Option<NaiveDateTime>,
    pub memory_type: String,
    pub container_tag: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryWithLinks {
    #[serde(flatten)]
    pub memory: Memory,
    pub linked_memory_ids: Vec<i64>,
    pub project_ids: Vec<i64>,
    pub entity_ids: Vec<i64>,
    pub document_ids: Vec<i64>,
    pub code_artifact_ids: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub primary_memories: Vec<MemoryWithLinks>,
    pub linked_memories: Vec<LinkedMemoryEntry>,
    pub total_count: usize,
    pub token_count: usize,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkedMemoryEntry {
    pub memory: MemoryWithLinks,
    pub link_source_id: i64,
}

// ─── Entity ───

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: i64,
    pub user_id: String,
    pub name: String,
    pub entity_type: String,
    pub custom_type: Option<String>,
    pub notes: Option<String>,
    pub tags: Vec<String>,
    pub aka: Vec<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityRelationship {
    pub id: i64,
    pub user_id: String,
    pub source_entity_id: i64,
    pub target_entity_id: i64,
    pub relationship_type: String,
    pub strength: Option<f64>,
    pub confidence: Option<f64>,
    pub relationship_metadata: Option<serde_json::Value>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

// ─── Project ───

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: i64,
    pub user_id: String,
    pub name: String,
    pub description: String,
    pub project_type: Option<String>,
    pub status: String,
    pub repo_name: Option<String>,
    pub notes: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

// ─── Document ───

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: i64,
    pub user_id: String,
    pub project_id: Option<i64>,
    pub title: String,
    pub description: String,
    pub content: String,
    pub document_type: Option<String>,
    pub filename: Option<String>,
    pub size_bytes: i64,
    pub tags: Vec<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

// ─── Code Artifact ───

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeArtifact {
    pub id: i64,
    pub user_id: String,
    pub project_id: Option<i64>,
    pub title: String,
    pub description: String,
    pub code: String,
    pub language: String,
    pub tags: Vec<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

// ─── User ───

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub external_id: String,
    pub name: String,
    pub email: String,
    pub notes: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

// ─── User Profile (Supermemory) ───

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub id: i64,
    pub user_id: String,
    pub static_facts: Vec<String>,
    pub dynamic_facts: Vec<String>,
    pub generated_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

// ─── Activity Log ───

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityLog {
    pub id: i64,
    pub user_id: String,
    pub entity_type: String,
    pub entity_id: i64,
    pub action: String,
    pub changes: Option<serde_json::Value>,
    pub snapshot: serde_json::Value,
    pub actor: String,
    pub actor_id: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: NaiveDateTime,
}

// ─── Tool Parameter Types ───

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateMemoryParams {
    /// Short, searchable title (max 200 chars)
    pub title: String,
    /// Single concept content (max 2000 chars)
    pub content: String,
    /// WHY this matters (max 500 chars)
    pub context: String,
    /// Keywords for search clustering (max 10)
    pub keywords: Vec<String>,
    /// Tags for categorization (max 10)
    pub tags: Vec<String>,
    /// Importance score 1-10 (higher = more important)
    pub importance: i32,
    /// Project IDs to associate with
    #[serde(default)]
    pub project_ids: Vec<i64>,
    /// Entity IDs to associate with
    #[serde(default)]
    pub entity_ids: Vec<i64>,
    /// Document IDs to associate with
    #[serde(default)]
    pub document_ids: Vec<i64>,
    /// Code artifact IDs to associate with
    #[serde(default)]
    pub code_artifact_ids: Vec<i64>,
    /// Source repository (e.g., "owner/repo")
    #[serde(default)]
    pub source_repo: Option<String>,
    /// Source file paths
    #[serde(default)]
    pub source_files: Option<Vec<String>>,
    /// Source URL
    #[serde(default)]
    pub source_url: Option<String>,
    /// Confidence score 0.0-1.0
    #[serde(default)]
    pub confidence: Option<f64>,
    /// Agent that created this memory
    #[serde(default)]
    pub encoding_agent: Option<String>,
    /// Memory type: fact, preference, or episode
    #[serde(default = "default_memory_type")]
    pub memory_type: String,
    /// Container tag for scoping
    #[serde(default)]
    pub container_tag: Option<String>,
    /// Auto-forget date (ISO 8601)
    #[serde(default)]
    pub forget_after: Option<String>,
}

fn default_memory_type() -> String {
    "fact".to_string()
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct QueryMemoryParams {
    /// Natural language search query
    pub query: String,
    /// Why you're searching (improves ranking context)
    #[serde(default)]
    pub query_context: Option<String>,
    /// Maximum number of results (default 10)
    #[serde(default = "default_k")]
    pub k: i32,
    /// Include linked memories in results
    #[serde(default = "default_true")]
    pub include_links: bool,
    /// Maximum linked memories per primary result
    #[serde(default = "default_max_links")]
    pub max_links_per_primary: i32,
    /// Filter by project IDs
    #[serde(default)]
    pub project_ids: Vec<i64>,
    /// Filter by tags
    #[serde(default)]
    pub tags: Vec<String>,
    /// Minimum importance score
    #[serde(default)]
    pub min_importance: Option<i32>,
    /// Token budget for results (default 8000)
    #[serde(default = "default_token_budget")]
    pub token_budget: i32,
    /// Filter by container tag
    #[serde(default)]
    pub container_tag: Option<String>,
}

fn default_k() -> i32 { 10 }
fn default_true() -> bool { true }
fn default_max_links() -> i32 { 5 }
fn default_token_budget() -> i32 { 8000 }

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetMemoryParams {
    /// Memory ID to retrieve
    pub memory_id: i64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateMemoryParams {
    /// Memory ID to update
    pub memory_id: i64,
    /// New title
    #[serde(default)]
    pub title: Option<String>,
    /// New content
    #[serde(default)]
    pub content: Option<String>,
    /// New context
    #[serde(default)]
    pub context: Option<String>,
    /// New keywords
    #[serde(default)]
    pub keywords: Option<Vec<String>>,
    /// New tags
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    /// New importance score
    #[serde(default)]
    pub importance: Option<i32>,
    /// New source repo
    #[serde(default)]
    pub source_repo: Option<String>,
    /// New source files
    #[serde(default)]
    pub source_files: Option<Vec<String>>,
    /// New source URL
    #[serde(default)]
    pub source_url: Option<String>,
    /// New confidence score
    #[serde(default)]
    pub confidence: Option<f64>,
    /// New container tag
    #[serde(default)]
    pub container_tag: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DeleteMemoryParams {
    /// Memory ID to delete
    pub memory_id: i64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LinkMemoriesParams {
    /// Source memory ID
    pub memory_id: i64,
    /// Target memory IDs to link
    pub related_ids: Vec<i64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UnlinkMemoriesParams {
    /// Source memory ID
    pub memory_id: i64,
    /// Target memory IDs to unlink
    pub related_ids: Vec<i64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MarkObsoleteParams {
    /// Memory ID to mark obsolete
    pub memory_id: i64,
    /// Reason for obsolescence
    pub reason: String,
    /// ID of memory that supersedes this one
    #[serde(default)]
    pub superseded_by: Option<i64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetRecentParams {
    /// Maximum number of results (default 10)
    #[serde(default = "default_k")]
    pub limit: i32,
    /// Filter by project IDs
    #[serde(default)]
    pub project_ids: Vec<i64>,
    /// Filter by container tag
    #[serde(default)]
    pub container_tag: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateVersionParams {
    /// Parent memory ID to create a version from
    pub parent_memory_id: i64,
    /// Relationship type: updates, extends, or derives
    pub relationship_type: String,
    /// New title
    pub title: String,
    /// New content
    pub content: String,
    /// New context
    pub context: String,
    /// New keywords
    pub keywords: Vec<String>,
    /// New tags
    pub tags: Vec<String>,
    /// New importance score
    pub importance: i32,
    /// Memory type
    #[serde(default = "default_memory_type")]
    pub memory_type: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetVersionChainParams {
    /// Memory ID to get version chain for
    pub memory_id: i64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ForgetMemoryParams {
    /// Memory ID to forget
    pub memory_id: i64,
    /// Reason for forgetting
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchSimilarParams {
    /// Memory ID to find similar memories for
    pub memory_id: i64,
    /// Maximum number of results
    #[serde(default = "default_similar_k")]
    pub k: i32,
}

fn default_similar_k() -> i32 { 5 }

// ─── Entity Params ───

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateEntityParams {
    /// Entity name
    pub name: String,
    /// Entity type (person, organization, device, concept, etc.)
    pub entity_type: String,
    /// Custom type label
    #[serde(default)]
    pub custom_type: Option<String>,
    /// Notes about this entity
    #[serde(default)]
    pub notes: Option<String>,
    /// Tags for categorization
    #[serde(default)]
    pub tags: Vec<String>,
    /// Also-known-as aliases
    #[serde(default)]
    pub aka: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetEntityParams {
    /// Entity ID
    pub entity_id: i64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListEntitiesParams {
    /// Filter by project IDs
    #[serde(default)]
    pub project_ids: Vec<i64>,
    /// Filter by entity type
    #[serde(default)]
    pub entity_type: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchEntitiesParams {
    /// Search query
    pub query: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateEntityParams {
    /// Entity ID
    pub entity_id: i64,
    /// New name
    #[serde(default)]
    pub name: Option<String>,
    /// New entity type
    #[serde(default)]
    pub entity_type: Option<String>,
    /// New custom type
    #[serde(default)]
    pub custom_type: Option<String>,
    /// New notes
    #[serde(default)]
    pub notes: Option<String>,
    /// New tags
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    /// New aliases
    #[serde(default)]
    pub aka: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DeleteEntityParams {
    /// Entity ID
    pub entity_id: i64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LinkEntityMemoryParams {
    /// Entity ID
    pub entity_id: i64,
    /// Memory ID
    pub memory_id: i64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LinkEntityProjectParams {
    /// Entity ID
    pub entity_id: i64,
    /// Project ID
    pub project_id: i64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetEntityMemoriesParams {
    /// Entity ID
    pub entity_id: i64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateRelationshipParams {
    /// Source entity ID
    pub source_entity_id: i64,
    /// Target entity ID
    pub target_entity_id: i64,
    /// Relationship type (e.g., works_for, owns, manages)
    pub relationship_type: String,
    /// Relationship strength 0.0-1.0
    #[serde(default)]
    pub strength: Option<f64>,
    /// Confidence score 0.0-1.0
    #[serde(default)]
    pub confidence: Option<f64>,
    /// Additional metadata
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetRelationshipsParams {
    /// Entity ID
    pub entity_id: i64,
    /// Direction: outgoing, incoming, or both
    #[serde(default = "default_direction")]
    pub direction: String,
}

fn default_direction() -> String {
    "both".to_string()
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateRelationshipParams {
    /// Relationship ID
    pub relationship_id: i64,
    /// New relationship type
    #[serde(default)]
    pub relationship_type: Option<String>,
    /// New strength
    #[serde(default)]
    pub strength: Option<f64>,
    /// New confidence
    #[serde(default)]
    pub confidence: Option<f64>,
    /// New metadata
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DeleteRelationshipParams {
    /// Relationship ID
    pub relationship_id: i64,
}

// ─── Project Params ───

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateProjectParams {
    /// Project name
    pub name: String,
    /// Project description
    pub description: String,
    /// Project type (e.g., development, research)
    #[serde(default)]
    pub project_type: Option<String>,
    /// Repository name (e.g., "owner/repo")
    #[serde(default)]
    pub repo_name: Option<String>,
    /// Notes
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetProjectParams {
    /// Project ID
    pub project_id: i64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListProjectsParams {
    /// Filter by repo name
    #[serde(default)]
    pub repo_name: Option<String>,
    /// Filter by status
    #[serde(default)]
    pub status: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateProjectParams {
    /// Project ID
    pub project_id: i64,
    /// New name
    #[serde(default)]
    pub name: Option<String>,
    /// New description
    #[serde(default)]
    pub description: Option<String>,
    /// New project type
    #[serde(default)]
    pub project_type: Option<String>,
    /// New status
    #[serde(default)]
    pub status: Option<String>,
    /// New repo name
    #[serde(default)]
    pub repo_name: Option<String>,
    /// New notes
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DeleteProjectParams {
    /// Project ID
    pub project_id: i64,
}

// ─── Document Params ───

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateDocumentParams {
    /// Document title
    pub title: String,
    /// Document description
    pub description: String,
    /// Document content
    pub content: String,
    /// Document type
    #[serde(default)]
    pub document_type: Option<String>,
    /// Filename
    #[serde(default)]
    pub filename: Option<String>,
    /// Tags
    #[serde(default)]
    pub tags: Vec<String>,
    /// Project ID
    #[serde(default)]
    pub project_id: Option<i64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetDocumentParams {
    /// Document ID
    pub document_id: i64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListDocumentsParams {
    /// Filter by project ID
    #[serde(default)]
    pub project_id: Option<i64>,
    /// Filter by document type
    #[serde(default)]
    pub document_type: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateDocumentParams {
    /// Document ID
    pub document_id: i64,
    /// New title
    #[serde(default)]
    pub title: Option<String>,
    /// New description
    #[serde(default)]
    pub description: Option<String>,
    /// New content
    #[serde(default)]
    pub content: Option<String>,
    /// New document type
    #[serde(default)]
    pub document_type: Option<String>,
    /// New filename
    #[serde(default)]
    pub filename: Option<String>,
    /// New tags
    #[serde(default)]
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DeleteDocumentParams {
    /// Document ID
    pub document_id: i64,
}

// ─── Code Artifact Params ───

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateCodeArtifactParams {
    /// Artifact title
    pub title: String,
    /// Artifact description
    pub description: String,
    /// The code
    pub code: String,
    /// Programming language
    pub language: String,
    /// Tags
    #[serde(default)]
    pub tags: Vec<String>,
    /// Project ID
    #[serde(default)]
    pub project_id: Option<i64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetCodeArtifactParams {
    /// Code artifact ID
    pub code_artifact_id: i64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListCodeArtifactsParams {
    /// Filter by project ID
    #[serde(default)]
    pub project_id: Option<i64>,
    /// Filter by language
    #[serde(default)]
    pub language: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateCodeArtifactParams {
    /// Code artifact ID
    pub code_artifact_id: i64,
    /// New title
    #[serde(default)]
    pub title: Option<String>,
    /// New description
    #[serde(default)]
    pub description: Option<String>,
    /// New code
    #[serde(default)]
    pub code: Option<String>,
    /// New language
    #[serde(default)]
    pub language: Option<String>,
    /// New tags
    #[serde(default)]
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DeleteCodeArtifactParams {
    /// Code artifact ID
    pub code_artifact_id: i64,
}

// ─── User Params ───

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateUserNotesParams {
    /// New notes content
    pub notes: String,
}

// ─── Meta Tool Params ───

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ToolInfoParams {
    /// Tool name to get info about
    pub tool_name: String,
}

// ─── Helper for estimating tokens ───
pub fn estimate_tokens(text: &str) -> usize {
    // Rough estimate: ~4 chars per token
    text.len() / 4
}
