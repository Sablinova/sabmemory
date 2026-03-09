# sabmemory

Lightweight Rust MCP memory server. Near-zero RAM alternative to embedding-based memory systems.

**~6 MB RAM** in production. Single 4.1 MB static binary. No Python, no Node.js, no ML runtime, no external dependencies. Built-in web dashboard with knowledge graph visualization.

## Why sabmemory?

Every other MCP memory server requires a heavy runtime (Python/Node.js), ML models for embeddings, or cloud API calls. sabmemory uses SQLite FTS5 full-text search instead of vector embeddings -- achieving fast, relevant search at a fraction of the resource cost.

### Comparison with other MCP memory servers

#### Overview

| Server | Language | Stars | License | Search Method | RAM Usage | Install Size |
|--------|----------|-------|---------|---------------|-----------|-------------|
| **sabmemory** | **Rust** | -- | MIT | FTS5 + BM25 | **~6 MB** | **4.1 MB binary** |
| [mcp-memory-service](https://github.com/doobidoo/mcp-memory-service) | Python | 1.5k | Apache 2.0 | BM25 + ONNX vector hybrid | ~500-850 MB | ~400 MB (models + deps) |
| [Forgetful](https://github.com/ScottRBK/forgetful) | Python | 190 | MIT | FastEmbed + cross-encoder rerank | ~300-500 MB | ~200 MB (models + deps) |
| [mem0](https://github.com/mem0ai/mem0) | Python/TS | 49.1k | Apache 2.0 | LLM-powered extraction + vector | ~300-500 MB | pip install + LLM API |
| [Basic Memory](https://github.com/basicmachines-co/basic-memory) | Python | 2.6k | MIT | FastEmbed vector search | ~200-400 MB | pip install (~200 MB) |
| [@mcp/memory](https://github.com/modelcontextprotocol/servers) | TypeScript | -- | MIT | Entity graph traversal | ~50-80 MB | npm install |
| [supermemory](https://github.com/supermemoryai/supermemory) | TypeScript | 16.8k | MIT | Hybrid RAG (cloud) | Cloud-hosted | SaaS |

#### Features

| Feature | sabmemory | mcp-memory-service | Forgetful | mem0 | Basic Memory | @mcp/memory | supermemory |
|---------|-----------|-------------------|-----------|------|--------------|-------------|-------------|
| MCP Tools | **49** | 15+ (REST + MCP) | 42 (3 meta) | ~10 | ~10 | 4 | Cloud API |
| Knowledge Graph | Yes | Yes (typed edges) | Yes | No | No | Yes (triples) | No |
| Entity System | Yes (typed, AKA) | No | Yes | No | No | Yes (simple) | No |
| Memory Versioning | Yes (chains) | No | No | No | No | No | Yes |
| Auto-Linking | Yes (FTS5) | No | Yes (vector) | No | No | No | No |
| Auto-Forget/Expiry | Yes | Yes (decay) | No | No | No | No | Yes |
| Token Budget | Yes | No | Yes | No | No | No | No |
| Projects | Yes | No | Yes | No | No | No | No |
| Documents | Yes | No | Yes | No | Yes (Markdown) | No | No |
| Code Artifacts | Yes | No | Yes | No | No | No | No |
| User Profiles | Yes (auto-gen) | No | No | Yes | No | No | No |
| Web Dashboard | Yes (built-in) | Yes | No | Yes (OpenMemory) | No | No | Yes (SaaS) |
| Multi-User | Single (local) | Yes (auth) | Yes (multi-tenant) | Yes | Single | Single | Yes (SaaS) |
| Container Scoping | Yes | No | No | No | No | No | No |

#### Runtime and Dependencies

| Aspect | sabmemory | mcp-memory-service | Forgetful | mem0 | Basic Memory | @mcp/memory | supermemory |
|--------|-----------|-------------------|-----------|------|--------------|-------------|-------------|
| Runtime | None (static binary) | Python 3.10+ | Python 3.12+ | Python 3.8+ | Python 3.11+ | Node.js 18+ | Cloud |
| External Services | None | None (ONNX local) | None (FastEmbed local) | OpenAI API key | None | None | Cloud API |
| ML Models Needed | **No** | Yes (MiniLM-L6-v2) | Yes (bge-small-en) | Yes (via LLM API) | Yes (FastEmbed) | No | Yes (cloud) |
| Storage Backend | SQLite (bundled) | SQLite-vec | SQLite or PostgreSQL | Qdrant/ChromaDB/custom | SQLite + Markdown | JSONL file | Cloud |
| Docker Support | Not needed | Yes | Yes | Yes | No | No | N/A |
| Install Command | `cargo build` | `pip install` | `uvx forgetful-ai` | `pip install mem0ai` | `pip install basic-memory` | `npx` | Sign up |
| Config Required | Zero-config | `.env` recommended | Zero-config | LLM API key required | Zero-config | Zero-config | Account required |
| Offline Operation | **Yes** | Yes | Yes | **No** (needs LLM API) | Yes | Yes | **No** |
| Self-Hosted | Yes | Yes | Yes | Yes | Yes | Yes | No (SaaS) |

#### Search Quality Tradeoffs

sabmemory trades semantic similarity for resource efficiency. FTS5 + BM25 performs keyword-aware ranked search rather than meaning-aware vector search. In practice, this works well for the MCP memory use case because:

1. Memories are short, atomic notes with explicit keywords and tags
2. The agent writing the memory is the same agent searching for it -- it uses consistent terminology
3. Importance score boosting surfaces the most relevant results regardless of lexical match
4. Auto-linking via FTS5 similarity captures related concepts at write time

For use cases requiring true semantic search (finding "automobile" when searching "car"), an embedding-based server like mcp-memory-service or Forgetful is more appropriate.

**Measured on a 2 GB VPS (Ubuntu 24.04).** sabmemory RSS: 6.5 MB Pss under active use with 16 memories, 2 entities, 2 projects, and 1 document. Idle instances drop to ~2 MB.

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

## Web Dashboard

sabmemory includes a built-in web dashboard for visualizing your knowledge graph, browsing memories, and exploring entities/projects/documents.

### Running the dashboard

```bash
sabmemory dashboard --port 3080
```

The dashboard is served at `http://localhost:3080` and provides:

- **Knowledge graph** -- interactive d3-force visualization of all memories, entities, and their connections
- **Memory browser** -- searchable list of all memories with importance, tags, and metadata
- **Entity explorer** -- view entities, their types, aliases, and relationships
- **Project view** -- projects with associated memory counts
- **Document view** -- stored long-form documents
- **Search** -- full-text search across all memories
- **Detail modal** -- click any memory to see full content, links, and associations

### Dashboard API

The dashboard exposes a REST API at `/api/`:

| Endpoint | Description |
|----------|-------------|
| `GET /api/stats` | Memory, entity, project, document, and link counts |
| `GET /api/memories` | All memories with metadata |
| `GET /api/memory/{id}` | Single memory with links and associations |
| `GET /api/entities` | All entities |
| `GET /api/relationships` | Entity relationships with resolved names |
| `GET /api/projects` | Projects with memory counts |
| `GET /api/documents` | All documents |
| `GET /api/graph` | Full knowledge graph (nodes + edges) for visualization |
| `GET /api/search?q=...` | FTS5 search |

### Running as a systemd service

```bash
# Create user service
mkdir -p ~/.config/systemd/user
cat > ~/.config/systemd/user/sabmemory-dashboard.service << 'EOF'
[Unit]
Description=sabmemory web dashboard
After=network.target

[Service]
ExecStart=%h/.local/bin/sabmemory dashboard --port 3080
Restart=on-failure
RestartSec=5

[Install]
WantedBy=default.target
EOF

# Enable and start
systemctl --user daemon-reload
systemctl --user enable --now sabmemory-dashboard
```

### Reverse proxy (nginx)

To serve the dashboard behind a reverse proxy with HTTPS:

```nginx
server {
    listen 443 ssl;
    server_name memory.yourdomain.com;

    location / {
        proxy_pass http://127.0.0.1:3080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

## Credits

sabmemory is inspired by and builds upon ideas from:

- **[supermemory](https://github.com/supermemoryai/supermemory)** — Memory versioning chains, auto-forget with expiry, container-based scoping, auto-generated user profiles, memory type classification
- **[forgetful-ai](https://github.com/ScottRBK/forgetful)** — Knowledge graph architecture (entities, relationships, memory-entity associations), document and code artifact storage, project-based organization, atomic memory principles

## License

MIT
