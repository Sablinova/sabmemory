# sabmemory

Lightweight Rust MCP memory server. Near-zero RAM alternative to embedding-based memory systems.

**~3-10 MB RAM** vs ~845 MB for ONNX embedding models. Single static binary, no Python, no ML runtime.

## Features

- **49 native MCP tools** — memories, entities, projects, documents, code artifacts, relationships, profiles, versioning
- **FTS5 full-text search** with BM25 ranking + importance score boosting
- **Memory versioning** — version chains with relationship types (updates/extends/derives), inspired by [supermemory](https://github.com/supermemoryai/supermemory)
- **Auto-forget** — set `forget_after` date for automatic expiry; soft-forget vs obsolete distinction
- **Container scoping** — `container_tag` field for namespace isolation
- **Auto-linking** — new memories are automatically linked to similar existing memories via FTS5
- **Token-budgeted search** — results bounded by configurable token budget (default 8000)
- **Knowledge graph** — entities with typed relationships, bidirectional memory links
- **Documents & code artifacts** — store long-form content and reusable code snippets
- **User profiles** — auto-generated from high-importance memories (static + dynamic facts)
- **Built-in migration** — import data from [forgetful-ai](https://github.com/ScottRBK/forgetful) databases

## Install

### From source

```bash
git clone https://github.com/Sablinova/sabmemory.git
cd sabmemory
cargo build --release
cp target/release/sabmemory ~/.local/bin/
```

### Requirements

- Rust 1.70+ (uses edition 2021)
- No other dependencies — SQLite is bundled via rusqlite

## Usage

### MCP Server (stdio)

```bash
sabmemory serve
# or just:
sabmemory
```

The server communicates over stdin/stdout using the MCP protocol. Configure it in your MCP client.

### OpenCode configuration

```json
{
  "mcp": {
    "sabmemory": {
      "type": "local",
      "command": ["/path/to/sabmemory", "serve"],
      "enabled": true
    }
  }
}
```

### Migration from forgetful-ai

```bash
sabmemory migrate --from ~/.local/share/forgetful/forgetful.db
```

Migrates: projects, entities, memories, documents, code artifacts, relationships, and all associations. FTS indexes are rebuilt automatically after migration.

## Data Storage

Database: `~/.local/share/sabmemory/sabmemory.db`

The database is created automatically on first run. All data is stored in a single SQLite file with FTS5 virtual tables for full-text search.

## Architecture

### Search Strategy

1. Natural language query is tokenized into FTS5 search terms
2. FTS5 MATCH with BM25 ranking scores relevance
3. Post-filters: obsolete, forgotten, expired, project, tags, container, min_importance
4. Importance score boosting applied to final ranking
5. Token budget enforcement limits result size
6. Linked memory expansion adds connected knowledge

### Memory Model

Memories follow Zettelkasten (atomic note) principles:
- **Title**: Short, searchable (max 200 chars)
- **Content**: Single concept (max 2000 chars)
- **Context**: Why this matters (max 500 chars)
- **Keywords**: For search clustering (max 10)
- **Tags**: For categorization (max 10)
- **Importance**: 1-10 score affecting search ranking and profile generation

### Versioning (supermemory-inspired)

Memories can form version chains via `parent_memory_id`:
- **updates**: New version contradicts/replaces the parent
- **extends**: New version enriches the parent
- **derives**: System-inferred relationship

Only the latest version (`is_latest=true`) appears in search results.

### Auto-Forget

Set `forget_after` (ISO 8601 date) on a memory. Expired memories are automatically marked as forgotten before each query. Forgotten memories are excluded from search but preserved for audit.

## Tools Reference

### Memory Tools (13)
| Tool | Description |
|------|-------------|
| `create_memory` | Create atomic memory with auto-linking |
| `get_memory` | Get memory with all associations |
| `update_memory` | PATCH update fields |
| `delete_memory` | Permanent delete |
| `query_memory` | FTS5 search with BM25 + importance boosting |
| `get_recent` | Recently updated memories |
| `link_memories` | Create bidirectional links |
| `unlink_memories` | Remove links |
| `mark_obsolete` | Soft delete with audit trail |
| `forget_memory` | Soft-forget (different from obsolete) |
| `search_similar` | Find textually similar memories |
| `create_version` | Create versioned update |
| `get_version_chain` | Get full version history |

### Entity Tools (15)
| Tool | Description |
|------|-------------|
| `create_entity` | Create entity (person, org, device, concept) |
| `get_entity` | Get entity details |
| `update_entity` | PATCH update |
| `delete_entity` | Delete with all associations |
| `list_entities` | List/filter entities |
| `search_entities` | FTS search by name/aliases |
| `get_entity_memories` | Memory IDs for entity |
| `link_entity_memory` | Link entity to memory |
| `unlink_entity_memory` | Remove entity-memory link |
| `link_entity_project` | Link entity to project |
| `unlink_entity_project` | Remove entity-project link |
| `create_relationship` | Typed directed relationship |
| `get_relationships` | Get entity relationships |
| `update_relationship` | Update relationship |
| `delete_relationship` | Delete relationship |

### Project Tools (5)
| Tool | Description |
|------|-------------|
| `create_project` | Create project container |
| `get_project` | Get project with memory count |
| `update_project` | PATCH update |
| `delete_project` | Delete (keeps associated memories) |
| `list_projects` | List/filter projects |

### Document Tools (5)
| Tool | Description |
|------|-------------|
| `create_document` | Store long-form content |
| `get_document` | Get with full content |
| `update_document` | PATCH update |
| `delete_document` | Delete |
| `list_documents` | List/filter |

### Code Artifact Tools (5)
| Tool | Description |
|------|-------------|
| `create_code_artifact` | Store reusable code |
| `get_code_artifact` | Get artifact |
| `update_code_artifact` | PATCH update |
| `delete_code_artifact` | Delete |
| `list_code_artifacts` | List/filter |

### User & Profile Tools (4)
| Tool | Description |
|------|-------------|
| `get_user` | Get user info |
| `update_user_notes` | Update user notes |
| `get_profile` | Auto-generated profile |
| `refresh_profile` | Regenerate profile |

### Meta Tools (2)
| Tool | Description |
|------|-------------|
| `list_tools` | List all tools by category |
| `tool_info` | Detailed tool info + schema |

## Credits

sabmemory is inspired by and builds upon ideas from:

- **[supermemory](https://github.com/supermemoryai/supermemory)** — Memory versioning chains, auto-forget with expiry, container-based scoping, auto-generated user profiles, memory type classification
- **[forgetful-ai](https://github.com/ScottRBK/forgetful)** — Knowledge graph architecture (entities, relationships, memory-entity associations), document and code artifact storage, project-based organization, atomic memory principles

## License

MIT
