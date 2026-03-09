// sabmemory // neural interface dashboard
(function() {
  'use strict';

  // ─── State ───
  let graphData = null;
  let memoriesData = [];
  let entitiesData = [];
  let projectsData = [];
  let documentsData = [];
  let statsData = {};

  // Three.js state
  let scene, camera, renderer, composer, controls;
  let nodeMeshes = [];        // { mesh, data, velocity:{x,y,z}, label, ring }
  let edgeLines = null;       // THREE.LineSegments (structural)
  let edgeLabelSprites = [];
  let particles = null;
  let raycaster, mouse;
  let hoveredNode = null;
  let hoveredEdge = null;     // { edgeData, lineGroup:'structural'|'memlink', drawIdx }
  let graphNodes = [];        // filtered working copy
  let graphEdges = [];
  let graphInitialized = false;
  let clock;
  let frameCount = 0;
  let lastFPSTime = 0;
  let currentFPS = 0;
  let simAlpha = 1.0;         // simulation cooling
  let graphMode = '3d';       // '3d' or '2d'
  let modeTransition = 0;     // 0 = done, >0 = transitioning
  let adjacency = {};         // nodeId -> { nodes: Set, structEdgeIdx: [], memEdgeIdx: [] }
  let highlightActive = false;
  let pulsePhase = 0;         // for animated pulse effects

  // ─── Constants ───
  const REPULSION = 600;
  const STRUCTURAL_SPRING_K = 0.03;
  const STRUCTURAL_SPRING_LENGTH = 20;
  const MEMORY_LINK_SPRING_K = 0.001;    // very weak — decorative
  const MEMORY_LINK_SPRING_LENGTH = 50;
  const DAMPING = 0.85;
  const CENTER_GRAVITY = 0.001;
  const HUB_GRAVITY = 0.0005;            // hubs pulled gently to origin
  const ALPHA_DECAY = 0.997;
  const ALPHA_MIN = 0.001;
  const PARTICLE_COUNT = 600;
  const HUB_SPREAD = 60;                 // how far apart hubs are placed
  const CHILD_SPREAD = 18;               // radius for children around hub

  // ─── Init ───
  document.addEventListener('DOMContentLoaded', async () => {
    setupNav();
    setupSearch();
    setupModal();
    startClock();
    await loadAll();
  });

  // ─── Header Clock ───
  function startClock() {
    const el = document.getElementById('header-time');
    function tick() {
      const now = new Date();
      const h = String(now.getHours()).padStart(2, '0');
      const m = String(now.getMinutes()).padStart(2, '0');
      const s = String(now.getSeconds()).padStart(2, '0');
      el.textContent = `${h}:${m}:${s} UTC`;
    }
    tick();
    setInterval(tick, 1000);
  }

  // ─── Navigation ───
  function setupNav() {
    document.querySelectorAll('.nav-btn').forEach(btn => {
      btn.addEventListener('click', () => {
        document.querySelectorAll('.nav-btn').forEach(b => b.classList.remove('active'));
        btn.classList.add('active');
        const view = btn.dataset.view;
        document.querySelectorAll('.view').forEach(v => v.classList.remove('active'));
        document.getElementById('view-' + view).classList.add('active');
        if (view === 'graph') resizeGraph();
      });
    });
  }

  // ─── Load all data ───
  async function loadAll() {
    try {
      const [stats, memories, entities, projects, documents, graph] = await Promise.all([
        fetchJSON('/api/stats'),
        fetchJSON('/api/memories'),
        fetchJSON('/api/entities'),
        fetchJSON('/api/projects'),
        fetchJSON('/api/documents'),
        fetchJSON('/api/graph'),
      ]);
      statsData = stats;
      memoriesData = memories;
      entitiesData = entities;
      projectsData = projects;
      documentsData = documents;
      graphData = graph;

      renderStats();
      renderMemories();
      renderEntities();
      renderProjects();
      renderDocuments();
      initGraph();
    } catch (e) {
      console.error('Failed to load data:', e);
    }
  }

  async function fetchJSON(url) {
    const res = await fetch(url);
    if (!res.ok) throw new Error(`HTTP ${res.status} for ${url}`);
    return res.json();
  }

  // ─── Stats Bar ───
  function renderStats() {
    const bar = document.getElementById('stats-bar');
    const items = [
      { label: 'Memories', value: statsData.memories || 0 },
      { label: 'Entities', value: statsData.entities || 0 },
      { label: 'Projects', value: statsData.projects || 0 },
      { label: 'Documents', value: statsData.documents || 0 },
      { label: 'Artifacts', value: statsData.code_artifacts || 0 },
      { label: 'Links', value: statsData.memory_links || 0 },
      { label: 'Relations', value: statsData.relationships || 0 },
      { label: 'Obsolete', value: statsData.obsolete || 0 },
      { label: 'Forgotten', value: statsData.forgotten || 0 },
    ];
    bar.innerHTML = items.map(i =>
      `<div class="stat-item"><span class="stat-value">${i.value}</span><span class="stat-label">${i.label}</span></div>`
    ).join('');
  }

  // ─── Memories View ───
  function renderMemories() {
    const list = document.getElementById('memories-list');
    const typeFilter = document.getElementById('mem-filter-type');
    const statusFilter = document.getElementById('mem-filter-status');

    function render() {
      let items = memoriesData;
      const t = typeFilter.value;
      const s = statusFilter.value;
      if (t) items = items.filter(m => m.memory_type === t);
      if (s === 'active') items = items.filter(m => !m.is_obsolete && !m.is_forgotten);
      else if (s === 'obsolete') items = items.filter(m => m.is_obsolete);
      else if (s === 'forgotten') items = items.filter(m => m.is_forgotten);

      if (items.length === 0) {
        list.innerHTML = '<div class="empty-state"><h3>No memories found</h3></div>';
        return;
      }

      list.innerHTML = items.map(m => memoryCard(m)).join('');
      list.querySelectorAll('.card').forEach(card => {
        card.addEventListener('click', () => openMemory(parseInt(card.dataset.id)));
      });
    }

    typeFilter.addEventListener('change', render);
    statusFilter.addEventListener('change', render);
    render();
  }

  function memoryCard(m) {
    const tags = parseTags(m.tags);
    const badges = [];
    badges.push(`<span class="badge badge-importance">imp:${m.importance}</span>`);
    badges.push(`<span class="badge badge-type">${m.memory_type}</span>`);
    if (m.is_obsolete) badges.push('<span class="badge badge-obsolete">obsolete</span>');
    if (m.is_forgotten) badges.push('<span class="badge badge-forgotten">forgotten</span>');
    if (!m.is_latest) badges.push(`<span class="badge badge-forgotten">v${m.version}</span>`);

    return `<div class="card" data-id="${m.id}">
      <div class="card-title">${esc(m.title)}</div>
      <div class="card-meta">${badges.join('')}</div>
      <div class="card-content">${esc(m.content)}</div>
      ${tags.length ? '<div class="card-tags">' + tags.map(t => `<span class="tag">${esc(t)}</span>`).join('') + '</div>' : ''}
    </div>`;
  }

  // ─── Memory Detail ───
  async function openMemory(id) {
    const modal = document.getElementById('memory-modal');
    const detail = document.getElementById('memory-detail');
    detail.innerHTML = '<p style="color:var(--text-muted)">Loading...</p>';
    modal.hidden = false;

    try {
      const m = await fetchJSON('/api/memory/' + id);
      const tags = parseTags(m.tags);
      const keywords = parseTags(m.keywords);
      const badges = [];
      badges.push(`<span class="badge badge-importance">importance: ${m.importance}</span>`);
      badges.push(`<span class="badge badge-type">${m.memory_type}</span>`);
      if (m.is_obsolete) badges.push('<span class="badge badge-obsolete">obsolete</span>');
      if (m.is_forgotten) badges.push('<span class="badge badge-forgotten">forgotten</span>');
      badges.push(`<span class="badge" style="background:var(--bg-card);color:var(--text-dim);border:1px solid var(--border)">v${m.version}</span>`);

      let html = `
        <div class="detail-title">${esc(m.title)}</div>
        <div class="detail-meta">${badges.join('')}</div>
        <div class="detail-section"><h3>Content</h3><pre>${esc(m.content)}</pre></div>
        <div class="detail-section"><h3>Context</h3><p>${esc(m.context)}</p></div>
      `;

      if (keywords.length) {
        html += `<div class="detail-section"><h3>Keywords</h3><div class="card-tags">${keywords.map(k => `<span class="tag">${esc(k)}</span>`).join('')}</div></div>`;
      }
      if (tags.length) {
        html += `<div class="detail-section"><h3>Tags</h3><div class="card-tags">${tags.map(t => `<span class="tag">${esc(t)}</span>`).join('')}</div></div>`;
      }

      if (m.linked_memory_ids && m.linked_memory_ids.length) {
        const chips = m.linked_memory_ids.map(lid => {
          const linked = memoriesData.find(x => x.id === lid);
          const label = linked ? linked.title : `Memory #${lid}`;
          return `<span class="detail-link-chip" onclick="window.__openMemory(${lid})">${esc(label)}</span>`;
        }).join('');
        html += `<div class="detail-section"><h3>Linked Memories</h3><div class="detail-links">${chips}</div></div>`;
      }

      if (m.entity_ids && m.entity_ids.length) {
        const chips = m.entity_ids.map(eid => {
          const ent = entitiesData.find(x => x.id === eid);
          const label = ent ? ent.name : `Entity #${eid}`;
          return `<span class="detail-link-chip entity-chip">${esc(label)}</span>`;
        }).join('');
        html += `<div class="detail-section"><h3>Entities</h3><div class="detail-links">${chips}</div></div>`;
      }

      if (m.project_ids && m.project_ids.length) {
        const chips = m.project_ids.map(pid => {
          const proj = projectsData.find(x => x.id === pid);
          const label = proj ? proj.name : `Project #${pid}`;
          return `<span class="detail-link-chip project-chip">${esc(label)}</span>`;
        }).join('');
        html += `<div class="detail-section"><h3>Projects</h3><div class="detail-links">${chips}</div></div>`;
      }

      const prov = [];
      if (m.source_repo) prov.push(`repo: ${m.source_repo}`);
      if (m.source_url) prov.push(`url: ${m.source_url}`);
      if (m.confidence !== null && m.confidence !== undefined) prov.push(`confidence: ${m.confidence}`);
      if (m.encoding_agent) prov.push(`agent: ${m.encoding_agent}`);
      if (m.parent_memory_id) prov.push(`parent: #${m.parent_memory_id}`);
      if (m.relationship_type) prov.push(`rel: ${m.relationship_type}`);
      if (prov.length) {
        html += `<div class="detail-section"><h3>Provenance</h3><p>${prov.map(esc).join(' | ')}</p></div>`;
      }

      html += `<div class="detail-section"><h3>Timestamps</h3><p>Created: ${m.created_at} | Updated: ${m.updated_at}</p></div>`;

      detail.innerHTML = html;
    } catch (e) {
      detail.innerHTML = `<p style="color:var(--red)">Error loading memory: ${esc(e.message)}</p>`;
    }
  }
  window.__openMemory = openMemory;

  function setupModal() {
    const modal = document.getElementById('memory-modal');
    modal.querySelector('.modal-close').addEventListener('click', () => modal.hidden = true);
    modal.addEventListener('click', e => { if (e.target === modal) modal.hidden = true; });
    document.addEventListener('keydown', e => { if (e.key === 'Escape') modal.hidden = true; });
  }

  // ─── Entities View ───
  function renderEntities() {
    const list = document.getElementById('entities-list');
    if (entitiesData.length === 0) {
      list.innerHTML = '<div class="empty-state"><h3>No entities</h3></div>';
      return;
    }
    list.innerHTML = entitiesData.map(e => {
      const tags = parseTags(e.tags);
      const aka = parseTags(e.aka);
      return `<div class="card">
        <div class="card-title">${esc(e.name)} <span class="entity-type-badge">${esc(e.entity_type)}</span>${e.custom_type ? ` <span class="entity-type-badge">${esc(e.custom_type)}</span>` : ''}</div>
        <div class="card-meta"><span>${e.created_at}</span></div>
        ${e.notes ? `<div class="card-content">${esc(e.notes)}</div>` : ''}
        ${aka.length ? `<div class="card-tags">${aka.map(a => `<span class="tag entity-tag">aka: ${esc(a)}</span>`).join('')}</div>` : ''}
        ${tags.length ? `<div class="card-tags">${tags.map(t => `<span class="tag entity-tag">${esc(t)}</span>`).join('')}</div>` : ''}
      </div>`;
    }).join('');
  }

  // ─── Projects View ───
  function renderProjects() {
    const list = document.getElementById('projects-list');
    if (projectsData.length === 0) {
      list.innerHTML = '<div class="empty-state"><h3>No projects</h3></div>';
      return;
    }
    list.innerHTML = projectsData.map(p => {
      return `<div class="card">
        <div class="card-title">${esc(p.name)} <span class="badge badge-status">${esc(p.status)}</span></div>
        <div class="card-meta">
          <span>${p.memory_count} memories</span>
          ${p.project_type ? `<span>${esc(p.project_type)}</span>` : ''}
          ${p.repo_name ? `<span>${esc(p.repo_name)}</span>` : ''}
        </div>
        <div class="card-content">${esc(p.description)}</div>
      </div>`;
    }).join('');
  }

  // ─── Documents View ───
  function renderDocuments() {
    const list = document.getElementById('documents-list');
    if (documentsData.length === 0) {
      list.innerHTML = '<div class="empty-state"><h3>No documents</h3></div>';
      return;
    }
    list.innerHTML = documentsData.map(d => {
      const tags = parseTags(d.tags);
      const sizeKB = (d.size_bytes / 1024).toFixed(1);
      return `<div class="card">
        <div class="card-title">${esc(d.title)} ${d.document_type ? `<span class="badge badge-type">${esc(d.document_type)}</span>` : ''}</div>
        <div class="card-meta"><span>${sizeKB} KB</span><span>${d.created_at}</span></div>
        <div class="card-content">${esc(d.description)}</div>
        ${tags.length ? `<div class="card-tags">${tags.map(t => `<span class="tag">${esc(t)}</span>`).join('')}</div>` : ''}
      </div>`;
    }).join('');
  }

  // ─── Search ───
  function setupSearch() {
    const input = document.getElementById('search-input');
    const btn = document.getElementById('search-btn');
    const results = document.getElementById('search-results');

    async function doSearch() {
      const q = input.value.trim();
      if (!q) return;
      results.innerHTML = '<p style="color:var(--text-muted);padding:20px;">Searching...</p>';
      try {
        const data = await fetchJSON('/api/search?q=' + encodeURIComponent(q));
        if (data.length === 0) {
          results.innerHTML = '<div class="empty-state"><h3>No results</h3></div>';
          return;
        }
        results.innerHTML = data.map(m => memoryCard(m)).join('');
        results.querySelectorAll('.card').forEach(card => {
          card.addEventListener('click', () => openMemory(parseInt(card.dataset.id)));
        });
      } catch (e) {
        results.innerHTML = `<p style="color:var(--red)">Search error: ${esc(e.message)}</p>`;
      }
    }

    btn.addEventListener('click', doSearch);
    input.addEventListener('keydown', e => { if (e.key === 'Enter') doSearch(); });
  }

  // ═══════════════════════════════════════════════════════════
  // ─── THREE.JS KNOWLEDGE GRAPH ───
  // ═══════════════════════════════════════════════════════════

  function nodeColor(d) {
    if (d.type === 'entity') return 0xaa55ff;
    if (d.type === 'project') return 0x00ff88;
    if (d.is_obsolete) return 0xff3355;
    if (d.is_forgotten) return 0x2a4a5a;
    return 0x00d4ff;
  }

  function nodeRadius(d) {
    if (d.type === 'entity') return 3.5;
    if (d.type === 'project') return 4.0;
    return 0.8 + (d.importance || 5) * 0.15;
  }

  function edgeColor(d) {
    const t = d.type || '';
    if (t === 'entity_assoc') return 0xaa55ff;
    if (t === 'project_assoc') return 0x00ff88;
    if (t === 'relationship') return 0xff6622;
    if (t === 'entity_project') return 0xffaa00;
    return 0x00d4ff;
  }

  function edgeTypeName(t) {
    if (t === 'entity_assoc') return 'ENTITY LINK';
    if (t === 'project_assoc') return 'PROJECT LINK';
    if (t === 'relationship') return 'RELATIONSHIP';
    if (t === 'entity_project') return 'ENTITY-PROJECT';
    if (t === 'memory_link') return 'MEMORY LINK';
    return t.toUpperCase();
  }

  function createTextSprite(text, color, fontSize) {
    const canvas = document.createElement('canvas');
    const ctx = canvas.getContext('2d');
    const sz = fontSize || 48;
    ctx.font = `bold ${sz}px Orbitron, sans-serif`;
    const metrics = ctx.measureText(text);
    const w = Math.ceil(metrics.width) + 16;
    const h = sz + 16;
    canvas.width = w;
    canvas.height = h;
    ctx.font = `bold ${sz}px Orbitron, sans-serif`;
    ctx.fillStyle = color || '#00d4ff';
    ctx.shadowColor = color || '#00d4ff';
    ctx.shadowBlur = 8;
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText(text, w / 2, h / 2);
    const tex = new THREE.CanvasTexture(canvas);
    tex.minFilter = THREE.LinearFilter;
    const mat = new THREE.SpriteMaterial({
      map: tex,
      transparent: true,
      opacity: 0.9,
      depthTest: false,
      sizeAttenuation: true,
    });
    const sprite = new THREE.Sprite(mat);
    sprite.scale.set(w / 20, h / 20, 1);
    return sprite;
  }

  function buildAdjacency() {
    adjacency = {};
    graphNodes.forEach(n => {
      adjacency[n.id] = { nodes: new Set(), structEdgeIdx: [], memEdgeIdx: [] };
    });
    const sEdges = window._structuralEdges || [];
    sEdges.forEach((e, i) => {
      if (e._si === undefined || e._ti === undefined) return;
      const sId = graphNodes[e._si].id;
      const tId = graphNodes[e._ti].id;
      adjacency[sId].nodes.add(tId);
      adjacency[tId].nodes.add(sId);
      adjacency[sId].structEdgeIdx.push(i);
      adjacency[tId].structEdgeIdx.push(i);
    });
    const mEdges = window._memoryLinkEdges || [];
    mEdges.forEach((e, i) => {
      if (e._si === undefined || e._ti === undefined) return;
      const sId = graphNodes[e._si].id;
      const tId = graphNodes[e._ti].id;
      adjacency[sId].nodes.add(tId);
      adjacency[tId].nodes.add(sId);
      adjacency[sId].memEdgeIdx.push(i);
      adjacency[tId].memEdgeIdx.push(i);
    });
  }

  function initGraph() {
    if (!graphData || !graphData.nodes || !graphData.nodes.length) {
      document.getElementById('graph-container').innerHTML =
        '<div class="empty-state" style="padding-top:100px"><h3>No data to visualize</h3></div>';
      return;
    }

    const container = document.getElementById('graph-container');
    const canvas = document.getElementById('graph-canvas');
    const width = container.clientWidth || 800;
    const height = container.clientHeight || 600;

    // ─── Scene Setup ───
    scene = new THREE.Scene();
    scene.background = new THREE.Color(0x030810);
    scene.fog = new THREE.FogExp2(0x030810, 0.003);

    camera = new THREE.PerspectiveCamera(60, width / height, 0.1, 2000);
    camera.position.set(0, 0, 120);

    renderer = new THREE.WebGLRenderer({
      canvas: canvas,
      antialias: true,
      alpha: false,
    });
    renderer.setSize(width, height);
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    renderer.toneMapping = THREE.ReinhardToneMapping;
    renderer.toneMappingExposure = 1.2;

    // ─── Post-processing (Bloom) ───
    composer = new THREE.EffectComposer(renderer);
    const renderPass = new THREE.RenderPass(scene, camera);
    composer.addPass(renderPass);

    const bloomPass = new THREE.UnrealBloomPass(
      new THREE.Vector2(width, height),
      1.2,   // strength
      0.4,   // radius
      0.2    // threshold
    );
    composer.addPass(bloomPass);

    // ─── Controls ───
    controls = new THREE.OrbitControls(camera, renderer.domElement);
    controls.enableDamping = true;
    controls.dampingFactor = 0.08;
    controls.rotateSpeed = 0.5;
    controls.zoomSpeed = 0.8;
    controls.minDistance = 10;
    controls.maxDistance = 500;
    controls.enablePan = true;
    controls.screenSpacePanning = true;

    // ─── Raycaster ───
    raycaster = new THREE.Raycaster();
    raycaster.params.Points = { threshold: 1 };
    mouse = new THREE.Vector2(-9999, -9999);

    // ─── Build Graph Data ───
    buildGraphObjects();

    // ─── Particles ───
    createParticles();

    // ─── Ambient Light (subtle) ───
    const ambientLight = new THREE.AmbientLight(0x112233, 0.5);
    scene.add(ambientLight);

    // ─── Event Listeners ───
    const tooltip = document.getElementById('graph-tooltip');

    canvas.addEventListener('mousemove', (e) => {
      const rect = canvas.getBoundingClientRect();
      mouse.x = ((e.clientX - rect.left) / rect.width) * 2 - 1;
      mouse.y = -((e.clientY - rect.top) / rect.height) * 2 + 1;

      // Update tooltip position
      const containerRect = container.getBoundingClientRect();
      tooltip.style.left = (e.clientX - containerRect.left + 14) + 'px';
      tooltip.style.top = (e.clientY - containerRect.top - 10) + 'px';
    });

    canvas.addEventListener('mouseleave', () => {
      mouse.x = -9999;
      mouse.y = -9999;
      tooltip.classList.remove('visible');
      clearHighlights();
      hoveredNode = null;
      hoveredEdge = null;
      canvas.style.cursor = 'grab';
    });

    canvas.addEventListener('click', () => {
      if (hoveredNode && hoveredNode.data.type === 'memory') {
        openMemory(hoveredNode.data.raw_id);
      }
    });

    // ─── Graph Controls ───
    document.getElementById('graph-reset').addEventListener('click', () => {
      if (graphMode === '2d') {
        camera.position.set(0, 0, 150);
        camera.up.set(0, 1, 0);
      } else {
        camera.position.set(0, 0, 120);
      }
      controls.target.set(0, 0, 0);
      controls.update();
    });

    // ─── 3D / 2D Toggle ───
    document.querySelectorAll('.view-toggle-btn').forEach(btn => {
      btn.addEventListener('click', () => {
        const mode = btn.dataset.mode;
        if (mode === graphMode) return;
        document.querySelectorAll('.view-toggle-btn').forEach(b => b.classList.remove('active'));
        btn.classList.add('active');
        switchGraphMode(mode);
      });
    });

    const showObsolete = document.getElementById('show-obsolete');
    const showForgotten = document.getElementById('show-forgotten');
    const minImp = document.getElementById('min-importance');
    const minImpVal = document.getElementById('min-imp-val');

    function applyFilters() {
      const minI = parseInt(minImp.value);
      minImpVal.textContent = minI;
      rebuildGraphObjects();
    }

    showObsolete.addEventListener('change', applyFilters);
    showForgotten.addEventListener('change', applyFilters);
    minImp.addEventListener('input', applyFilters);

    // ─── Animation Loop ───
    clock = new THREE.Clock();
    graphInitialized = true;
    animate();
  }

  function buildGraphObjects() {
    const showObsolete = document.getElementById('show-obsolete').checked;
    const showForgotten = document.getElementById('show-forgotten').checked;
    const minI = parseInt(document.getElementById('min-importance').value);

    // Filter nodes
    graphNodes = graphData.nodes.filter(n => {
      if (n.type === 'entity' || n.type === 'project') return true;
      if (n.is_obsolete && !showObsolete) return false;
      if (n.is_forgotten && !showForgotten) return false;
      if ((n.importance || 5) < minI) return false;
      return true;
    }).map(n => ({ ...n }));

    const nodeIdSet = new Set(graphNodes.map(n => n.id));

    // Filter edges — separate structural from memory_links
    const allEdges = graphData.edges.filter(e =>
      nodeIdSet.has(e.source) && nodeIdSet.has(e.target)
    ).map(e => ({ ...e }));

    // Classify edges
    const structuralEdges = [];  // entity_assoc, project_assoc, relationship, entity_project
    const memoryLinkEdges = [];  // memory_link (decorative)
    allEdges.forEach(e => {
      if (e.type === 'memory_link') {
        memoryLinkEdges.push(e);
      } else {
        structuralEdges.push(e);
      }
    });
    graphEdges = allEdges;

    // Build ID -> index map
    const idToIdx = {};
    graphNodes.forEach((n, i) => { idToIdx[n.id] = i; });

    // ─── Hierarchical Initial Positioning ───
    // Identify hubs (entities + projects) and their children
    const hubs = graphNodes.filter(n => n.type === 'entity' || n.type === 'project');
    const memoryNodes = graphNodes.filter(n => n.type === 'memory');

    // Map each memory to its parent hub(s) via structural edges
    const memoryToHub = {};  // memoryId -> first hub id
    const hubChildren = {};  // hubId -> [memoryIds]
    hubs.forEach(h => { hubChildren[h.id] = []; });

    structuralEdges.forEach(e => {
      const sNode = graphNodes[idToIdx[e.source]];
      const tNode = graphNodes[idToIdx[e.target]];
      if (!sNode || !tNode) return;

      if ((sNode.type === 'entity' || sNode.type === 'project') && tNode.type === 'memory') {
        if (!memoryToHub[tNode.id]) {
          memoryToHub[tNode.id] = sNode.id;
          hubChildren[sNode.id].push(tNode.id);
        }
      } else if ((tNode.type === 'entity' || tNode.type === 'project') && sNode.type === 'memory') {
        if (!memoryToHub[sNode.id]) {
          memoryToHub[sNode.id] = tNode.id;
          hubChildren[tNode.id].push(sNode.id);
        }
      }
    });

    // Orphan memories (no hub parent)
    const orphans = memoryNodes.filter(m => !memoryToHub[m.id]);

    // Position hubs evenly spaced
    const hubAngleStep = (2 * Math.PI) / Math.max(hubs.length, 1);
    const layoutIs2D = graphMode === '2d';
    hubs.forEach((h, i) => {
      const angle = hubAngleStep * i;
      h.x = Math.cos(angle) * HUB_SPREAD;
      h.y = layoutIs2D ? (Math.sin(angle) * HUB_SPREAD) : ((i % 2 === 0 ? 1 : -1) * 10);
      h.z = layoutIs2D ? 0 : (Math.sin(angle) * HUB_SPREAD);
      h.vx = 0; h.vy = 0; h.vz = 0;
      h._isHub = true;
      h._hubAnchorX = h.x;
      h._hubAnchorY = h.y;
      h._hubAnchorZ = h.z;
    });

    // Position children around their hub
    hubs.forEach(h => {
      const children = hubChildren[h.id];
      const count = children.length;
      children.forEach((cid, ci) => {
        const cn = graphNodes[idToIdx[cid]];
        if (!cn) return;
        if (layoutIs2D) {
          // 2D: distribute children in a circle around hub in XY plane
          const theta = (2 * Math.PI * ci) / Math.max(count, 1);
          const r = CHILD_SPREAD * (0.6 + Math.random() * 0.4);
          cn.x = h.x + r * Math.cos(theta);
          cn.y = h.y + r * Math.sin(theta);
          cn.z = 0;
        } else {
          // 3D: Fibonacci sphere distribution around hub
          const phi = Math.acos(1 - 2 * (ci + 0.5) / Math.max(count, 1));
          const theta = Math.PI * (1 + Math.sqrt(5)) * ci;
          const r = CHILD_SPREAD * (0.6 + Math.random() * 0.4);
          cn.x = h.x + r * Math.sin(phi) * Math.cos(theta);
          cn.y = h.y + r * Math.sin(phi) * Math.sin(theta);
          cn.z = h.z + r * Math.cos(phi);
        }
        cn.vx = 0; cn.vy = 0; cn.vz = 0;
        cn._parentHub = h.id;
      });
    });

    // Position orphans in a cluster near the center
    orphans.forEach((o, i) => {
      if (layoutIs2D) {
        const theta = (2 * Math.PI * i) / Math.max(orphans.length, 1);
        const r = CHILD_SPREAD * 0.8;
        o.x = r * Math.cos(theta);
        o.y = r * Math.sin(theta);
        o.z = 0;
      } else {
        const phi = Math.acos(1 - 2 * (i + 0.5) / Math.max(orphans.length, 1));
        const theta = Math.PI * (1 + Math.sqrt(5)) * i;
        const r = CHILD_SPREAD * 0.8;
        o.x = r * Math.sin(phi) * Math.cos(theta);
        o.y = r * Math.sin(phi) * Math.sin(theta);
        o.z = r * Math.cos(phi);
      }
      o.vx = 0; o.vy = 0; o.vz = 0;
    });

    // Tag edges with their type category and store index refs
    allEdges.forEach(e => {
      e._si = idToIdx[e.source];
      e._ti = idToIdx[e.target];
      e._isStructural = (e.type !== 'memory_link');
    });

    // ─── Create Node Meshes ───
    nodeMeshes.forEach(nm => scene.remove(nm.mesh));
    nodeMeshes = [];

    graphNodes.forEach(n => {
      const r = nodeRadius(n);
      const color = nodeColor(n);
      const geo = new THREE.SphereGeometry(r, 16, 12);
      const mat = new THREE.MeshBasicMaterial({
        color: color,
        transparent: true,
        opacity: (n.is_obsolete || n.is_forgotten) ? 0.35 : 0.9,
      });
      const mesh = new THREE.Mesh(geo, mat);
      mesh.position.set(n.x, n.y, n.z);
      scene.add(mesh);

      // Glow shell
      const glowGeo = new THREE.SphereGeometry(r * 1.6, 12, 8);
      const glowMat = new THREE.MeshBasicMaterial({
        color: color,
        transparent: true,
        opacity: n._isHub ? 0.15 : 0.08,
        side: THREE.BackSide,
      });
      const glowMesh = new THREE.Mesh(glowGeo, glowMat);
      mesh.add(glowMesh);

      nodeMeshes.push({ mesh, data: n, glowMesh, glowMat });
    });

    // ─── Create Edges — two layers ───
    // Clean up old
    if (edgeLines) scene.remove(edgeLines);
    if (window._memLinkLines) scene.remove(window._memLinkLines);
    edgeLines = null;
    window._memLinkLines = null;

    // Structural edges (prominent)
    if (structuralEdges.length > 0) {
      const positions = new Float32Array(structuralEdges.length * 6);
      const colors = new Float32Array(structuralEdges.length * 6);

      structuralEdges.forEach((e, i) => {
        if (e._si === undefined || e._ti === undefined) return;
        const s = graphNodes[e._si];
        const t = graphNodes[e._ti];
        const off = i * 6;
        positions[off]     = s.x; positions[off+1] = s.y; positions[off+2] = s.z;
        positions[off + 3] = t.x; positions[off+4] = t.y; positions[off+5] = t.z;
        const c = new THREE.Color(edgeColor(e));
        colors[off]     = c.r; colors[off+1] = c.g; colors[off+2] = c.b;
        colors[off + 3] = c.r; colors[off+4] = c.g; colors[off+5] = c.b;
        e._drawIdx = i;
      });

      const lineGeo = new THREE.BufferGeometry();
      lineGeo.setAttribute('position', new THREE.BufferAttribute(positions, 3));
      lineGeo.setAttribute('color', new THREE.BufferAttribute(colors, 3));
      const lineMat = new THREE.LineBasicMaterial({
        vertexColors: true,
        transparent: true,
        opacity: 0.85,
        linewidth: 1,
      });
      edgeLines = new THREE.LineSegments(lineGeo, lineMat);
      scene.add(edgeLines);
    }

    // Memory link edges (subtle but visible)
    if (memoryLinkEdges.length > 0) {
      const positions = new Float32Array(memoryLinkEdges.length * 6);
      const colors = new Float32Array(memoryLinkEdges.length * 6);

      memoryLinkEdges.forEach((e, i) => {
        if (e._si === undefined || e._ti === undefined) return;
        const s = graphNodes[e._si];
        const t = graphNodes[e._ti];
        const off = i * 6;
        positions[off]     = s.x; positions[off+1] = s.y; positions[off+2] = s.z;
        positions[off + 3] = t.x; positions[off+4] = t.y; positions[off+5] = t.z;
        // Dim cyan for memory links
        colors[off]     = 0; colors[off+1] = 0.4; colors[off+2] = 0.6;
        colors[off + 3] = 0; colors[off+4] = 0.4; colors[off+5] = 0.6;
        e._drawIdx = i;
      });

      const lineGeo = new THREE.BufferGeometry();
      lineGeo.setAttribute('position', new THREE.BufferAttribute(positions, 3));
      lineGeo.setAttribute('color', new THREE.BufferAttribute(colors, 3));
      const lineMat = new THREE.LineBasicMaterial({
        vertexColors: true,
        transparent: true,
        opacity: 0.2,
      });
      window._memLinkLines = new THREE.LineSegments(lineGeo, lineMat);
      scene.add(window._memLinkLines);
    }

    // Store edge lists on module scope for simulation
    window._structuralEdges = structuralEdges;
    window._memoryLinkEdges = memoryLinkEdges;

    // Build adjacency map for highlight logic
    buildAdjacency();

    // ─── Text Labels for Hub Nodes ───
    edgeLabelSprites.forEach(s => scene.remove(s));
    edgeLabelSprites = [];

    nodeMeshes.forEach(nm => {
      const n = nm.data;
      if (n._isHub) {
        // Always-visible label for hubs
        const color = n.type === 'entity' ? '#aa55ff' : '#00ff88';
        const label = createTextSprite(n.label, color, 36);
        const r = nodeRadius(n);
        label.position.set(n.x, n.y + r + 3, n.z);
        scene.add(label);
        nm.label = label;
        nm.labelColor = color;
        edgeLabelSprites.push(label);
      } else {
        // Create hidden label for memory nodes (shown on hover)
        const label = createTextSprite(truncate(n.label, 40), '#00d4ff', 28);
        const r = nodeRadius(n);
        label.position.set(n.x, n.y + r + 2, n.z);
        label.visible = false;
        scene.add(label);
        nm.label = label;
        nm.labelColor = '#00d4ff';
        edgeLabelSprites.push(label);
      }

      // ─── Importance Ring ───
      if (n.type === 'memory' && n.importance) {
        const impNorm = (n.importance || 5) / 10;
        const ringGeo = new THREE.RingGeometry(
          nodeRadius(n) * 1.8,
          nodeRadius(n) * 2.2,
          24
        );
        const hue = impNorm * 0.33; // red(0) -> green(0.33)
        const ringColor = new THREE.Color().setHSL(hue, 1, 0.5);
        const ringMat = new THREE.MeshBasicMaterial({
          color: ringColor,
          transparent: true,
          opacity: 0.25 + impNorm * 0.2,
          side: THREE.DoubleSide,
        });
        const ring = new THREE.Mesh(ringGeo, ringMat);
        ring.lookAt(camera.position);
        nm.mesh.add(ring);
        nm.ring = ring;
        nm.ringMat = ringMat;
      }
    });

    // Update HUD
    document.getElementById('hud-nodes').textContent = graphNodes.length;
    document.getElementById('hud-edges').textContent = allEdges.length;

    // Reset simulation
    simAlpha = 1.0;
  }

  function rebuildGraphObjects() {
    buildGraphObjects();
  }

  // ─── Mode Switching ───
  function switchGraphMode(mode) {
    graphMode = mode;
    modeTransition = 60; // frames to animate transition
    simAlpha = 0.8;      // reheat simulation for new layout

    // Rebuild graph with new layout positions
    rebuildGraphObjects();

    if (mode === '2d') {
      // Disable 3D rotation, allow only pan/zoom
      controls.enableRotate = false;
      controls.enablePan = true;
      controls.panSpeed = 1.5;

      // Animate camera to top-down view (looking down Z axis)
      camera.up.set(0, 1, 0);
      animateCameraTo(0, 0, 150);
    } else {
      // Re-enable full 3D controls
      controls.enableRotate = true;
      controls.enablePan = true;
      controls.panSpeed = 1.0;

      // Animate camera to perspective view
      animateCameraTo(0, 0, 120);
    }
  }

  function animateCameraTo(tx, ty, tz) {
    const startPos = { x: camera.position.x, y: camera.position.y, z: camera.position.z };
    const startTarget = { x: controls.target.x, y: controls.target.y, z: controls.target.z };
    const frames = 40;
    let frame = 0;

    function step() {
      frame++;
      const t = easeInOutCubic(frame / frames);
      camera.position.set(
        startPos.x + (tx - startPos.x) * t,
        startPos.y + (ty - startPos.y) * t,
        startPos.z + (tz - startPos.z) * t
      );
      controls.target.set(
        startTarget.x * (1 - t),
        startTarget.y * (1 - t),
        startTarget.z * (1 - t)
      );
      controls.update();
      if (frame < frames) requestAnimationFrame(step);
    }
    step();
  }

  function easeInOutCubic(t) {
    return t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2;
  }

  function createParticles() {
    if (particles) scene.remove(particles);

    const positions = new Float32Array(PARTICLE_COUNT * 3);
    const alphas = new Float32Array(PARTICLE_COUNT);

    for (let i = 0; i < PARTICLE_COUNT; i++) {
      positions[i * 3]     = (Math.random() - 0.5) * 400;
      positions[i * 3 + 1] = (Math.random() - 0.5) * 400;
      positions[i * 3 + 2] = (Math.random() - 0.5) * 400;
      alphas[i] = Math.random();
    }

    const geo = new THREE.BufferGeometry();
    geo.setAttribute('position', new THREE.BufferAttribute(positions, 3));

    const mat = new THREE.PointsMaterial({
      color: 0x00d4ff,
      size: 0.4,
      transparent: true,
      opacity: 0.25,
      sizeAttenuation: true,
      blending: THREE.AdditiveBlending,
      depthWrite: false,
    });

    particles = new THREE.Points(geo, mat);
    scene.add(particles);
    particles._alphas = alphas;
  }

  // ─── Force Simulation (3D — Hierarchical) ───
  function simulateForces() {
    if (simAlpha < ALPHA_MIN) return;

    const n = graphNodes.length;

    // Repulsion — all pairs, but hubs repel much more strongly
    const is2D = graphMode === '2d';
    for (let i = 0; i < n; i++) {
      for (let j = i + 1; j < n; j++) {
        const a = graphNodes[i];
        const b = graphNodes[j];
        let dx = a.x - b.x;
        let dy = a.y - b.y;
        let dz = is2D ? 0 : (a.z - b.z);
        let dist2 = dx * dx + dy * dy + dz * dz;
        if (dist2 < 0.01) {
          dx = (Math.random() - 0.5) * 0.1;
          dy = (Math.random() - 0.5) * 0.1;
          dz = is2D ? 0 : (Math.random() - 0.5) * 0.1;
          dist2 = dx * dx + dy * dy + dz * dz;
        }
        const dist = Math.sqrt(dist2);
        // Hubs repel each other much more strongly to stay apart
        const bothHubs = a._isHub && b._isHub;
        const repMul = bothHubs ? 4.0 : 1.0;
        const force = REPULSION * repMul * simAlpha / dist2;
        const fx = dx / dist * force;
        const fy = dy / dist * force;
        const fz = is2D ? 0 : (dz / dist * force);
        // Hubs are "heavy" — they receive less force
        const aWeight = a._isHub ? 0.1 : 1.0;
        const bWeight = b._isHub ? 0.1 : 1.0;
        a.vx += fx * aWeight; a.vy += fy * aWeight; a.vz += fz * aWeight;
        b.vx -= fx * bWeight; b.vy -= fy * bWeight; b.vz -= fz * bWeight;
      }
    }

    // Structural spring forces (strong — these define the tree)
    const sEdges = window._structuralEdges || [];
    for (let i = 0; i < sEdges.length; i++) {
      const e = sEdges[i];
      if (e._si === undefined || e._ti === undefined) continue;
      const a = graphNodes[e._si];
      const b = graphNodes[e._ti];
      let dx = b.x - a.x;
      let dy = b.y - a.y;
      let dz = is2D ? 0 : (b.z - a.z);
      const dist = Math.sqrt(dx * dx + dy * dy + dz * dz) || 0.01;
      const displacement = dist - STRUCTURAL_SPRING_LENGTH;
      const force = STRUCTURAL_SPRING_K * displacement * simAlpha;
      const fx = dx / dist * force;
      const fy = dy / dist * force;
      const fz = is2D ? 0 : (dz / dist * force);
      const aWeight = a._isHub ? 0.05 : 1.0;
      const bWeight = b._isHub ? 0.05 : 1.0;
      a.vx += fx * aWeight; a.vy += fy * aWeight; a.vz += fz * aWeight;
      b.vx -= fx * bWeight; b.vy -= fy * bWeight; b.vz -= fz * bWeight;
    }

    // Memory link spring forces (very weak — just gentle clustering)
    const mEdges = window._memoryLinkEdges || [];
    for (let i = 0; i < mEdges.length; i++) {
      const e = mEdges[i];
      if (e._si === undefined || e._ti === undefined) continue;
      const a = graphNodes[e._si];
      const b = graphNodes[e._ti];
      let dx = b.x - a.x;
      let dy = b.y - a.y;
      let dz = is2D ? 0 : (b.z - a.z);
      const dist = Math.sqrt(dx * dx + dy * dy + dz * dz) || 0.01;
      const displacement = dist - MEMORY_LINK_SPRING_LENGTH;
      const force = MEMORY_LINK_SPRING_K * displacement * simAlpha;
      const fx = dx / dist * force;
      const fy = dy / dist * force;
      const fz = is2D ? 0 : (dz / dist * force);
      a.vx += fx; a.vy += fy; a.vz += fz;
      b.vx -= fx; b.vy -= fy; b.vz -= fz;
    }

    // Hub anchor gravity — hubs gently pulled back to their initial positions
    for (let i = 0; i < n; i++) {
      const nd = graphNodes[i];
      if (nd._isHub) {
        nd.vx += (nd._hubAnchorX - nd.x) * 0.01 * simAlpha;
        nd.vy += (nd._hubAnchorY - nd.y) * 0.01 * simAlpha;
        if (!is2D) {
          nd.vz += (nd._hubAnchorZ - nd.z) * 0.01 * simAlpha;
        }
      } else {
        // Regular center gravity for non-hubs (weak)
        nd.vx -= nd.x * CENTER_GRAVITY * simAlpha;
        nd.vy -= nd.y * CENTER_GRAVITY * simAlpha;
        if (!is2D) {
          nd.vz -= nd.z * CENTER_GRAVITY * simAlpha;
        }
      }
    }

    // Apply velocity + damping
    for (let i = 0; i < n; i++) {
      const nd = graphNodes[i];
      nd.vx *= DAMPING;
      nd.vy *= DAMPING;
      nd.vz *= DAMPING;

      // In 2D mode, kill Z velocity and flatten Z positions
      if (graphMode === '2d') {
        nd.vz = 0;
        if (modeTransition > 0) {
          // Smoothly lerp Z toward 0 during transition
          nd.z *= 0.9;
        } else {
          nd.z = 0;
        }
      }

      nd.x += nd.vx;
      nd.y += nd.vy;
      nd.z += nd.vz;
    }

    simAlpha *= ALPHA_DECAY;
  }

  function updatePositions() {
    // Update node meshes + labels
    for (let i = 0; i < nodeMeshes.length; i++) {
      const nm = nodeMeshes[i];
      nm.mesh.position.set(nm.data.x, nm.data.y, nm.data.z);
      if (nm.label) {
        const r = nodeRadius(nm.data);
        const yOff = nm.data._isHub ? r + 3 : r + 2;
        nm.label.position.set(nm.data.x, nm.data.y + yOff, nm.data.z);
      }
      // Make importance rings face camera
      if (nm.ring) {
        nm.ring.lookAt(
          camera.position.x - nm.mesh.position.x,
          camera.position.y - nm.mesh.position.y,
          camera.position.z - nm.mesh.position.z
        );
      }
    }

    // Update structural edge lines
    const sEdges = window._structuralEdges || [];
    if (edgeLines && sEdges.length > 0) {
      const pos = edgeLines.geometry.attributes.position.array;
      for (let i = 0; i < sEdges.length; i++) {
        const e = sEdges[i];
        if (e._si === undefined || e._ti === undefined) continue;
        const s = graphNodes[e._si];
        const t = graphNodes[e._ti];
        const off = i * 6;
        pos[off]     = s.x; pos[off+1] = s.y; pos[off+2] = s.z;
        pos[off + 3] = t.x; pos[off+4] = t.y; pos[off+5] = t.z;
      }
      edgeLines.geometry.attributes.position.needsUpdate = true;
    }

    // Update memory link lines
    const mEdges = window._memoryLinkEdges || [];
    if (window._memLinkLines && mEdges.length > 0) {
      const pos = window._memLinkLines.geometry.attributes.position.array;
      for (let i = 0; i < mEdges.length; i++) {
        const e = mEdges[i];
        if (e._si === undefined || e._ti === undefined) continue;
        const s = graphNodes[e._si];
        const t = graphNodes[e._ti];
        const off = i * 6;
        pos[off]     = s.x; pos[off+1] = s.y; pos[off+2] = s.z;
        pos[off + 3] = t.x; pos[off+4] = t.y; pos[off+5] = t.z;
      }
      window._memLinkLines.geometry.attributes.position.needsUpdate = true;
    }
  }

  function updateParticles(dt) {
    if (!particles) return;
    const pos = particles.geometry.attributes.position.array;
    const alphas = particles._alphas;
    for (let i = 0; i < PARTICLE_COUNT; i++) {
      const i3 = i * 3;
      // Gentle drift
      pos[i3]     += Math.sin(alphas[i] * 6.28 + dt * 0.5) * 0.02;
      pos[i3 + 1] += Math.cos(alphas[i] * 3.14 + dt * 0.3) * 0.02;
      pos[i3 + 2] += Math.sin(alphas[i] * 4.71 + dt * 0.4) * 0.015;
      alphas[i] += 0.001;
    }
    particles.geometry.attributes.position.needsUpdate = true;
    particles.rotation.y += 0.0001;
  }

  function pointToSegmentDist(px, py, pz, ax, ay, az, bx, by, bz) {
    const dx = bx - ax, dy = by - ay, dz = bz - az;
    const len2 = dx * dx + dy * dy + dz * dz;
    if (len2 < 0.0001) return Math.sqrt((px-ax)*(px-ax)+(py-ay)*(py-ay)+(pz-az)*(pz-az));
    let t = ((px-ax)*dx + (py-ay)*dy + (pz-az)*dz) / len2;
    t = Math.max(0, Math.min(1, t));
    const cx = ax + t * dx, cy = ay + t * dy, cz = az + t * dz;
    return Math.sqrt((px-cx)*(px-cx)+(py-cy)*(py-cy)+(pz-cz)*(pz-cz));
  }

  function clearHighlights() {
    if (!highlightActive) return;
    highlightActive = false;
    hoveredEdge = null;

    // Restore all node opacities and reset scale
    nodeMeshes.forEach(nm => {
      const isObs = nm.data.is_obsolete || nm.data.is_forgotten;
      nm.mesh.material.opacity = isObs ? 0.35 : 0.9;
      nm.mesh.scale.set(1, 1, 1);
      nm.glowMat.opacity = nm.data._isHub ? 0.15 : 0.08;
      if (nm.label && !nm.data._isHub) nm.label.visible = false;
      if (nm.label && nm.data._isHub) nm.label.material.opacity = 0.9;
    });

    // Restore edge opacities
    if (edgeLines) edgeLines.material.opacity = 0.85;
    if (window._memLinkLines) window._memLinkLines.material.opacity = 0.2;

    // Restore structural edge colors
    if (edgeLines) {
      const sEdges = window._structuralEdges || [];
      const colors = edgeLines.geometry.attributes.color.array;
      sEdges.forEach((e, i) => {
        const c = new THREE.Color(edgeColor(e));
        const off = i * 6;
        colors[off] = c.r; colors[off+1] = c.g; colors[off+2] = c.b;
        colors[off+3] = c.r; colors[off+4] = c.g; colors[off+5] = c.b;
      });
      edgeLines.geometry.attributes.color.needsUpdate = true;
    }

    // Restore memory link edge colors
    if (window._memLinkLines) {
      const mEdges = window._memoryLinkEdges || [];
      const colors = window._memLinkLines.geometry.attributes.color.array;
      mEdges.forEach((e, i) => {
        const off = i * 6;
        colors[off] = 0; colors[off+1] = 0.4; colors[off+2] = 0.6;
        colors[off+3] = 0; colors[off+4] = 0.4; colors[off+5] = 0.6;
      });
      window._memLinkLines.geometry.attributes.color.needsUpdate = true;
    }

    // Hide info panel
    const infoPanel = document.getElementById('graph-info-panel');
    if (infoPanel) infoPanel.classList.remove('visible');
  }

  function highlightNode(nm) {
    highlightActive = true;
    const nodeId = nm.data.id;
    const adj = adjacency[nodeId];
    if (!adj) return;

    const connectedNodes = adj.nodes;

    // Dim all nodes, brighten connected ones
    nodeMeshes.forEach(other => {
      if (other === nm) {
        // Hovered node — bright + pulsing glow
        other.mesh.material.opacity = 1.0;
        other.glowMat.opacity = 0.4;
        if (other.label) { other.label.visible = true; other.label.material.opacity = 1.0; }
      } else if (connectedNodes.has(other.data.id)) {
        // Connected neighbor — bright
        other.mesh.material.opacity = 0.9;
        other.glowMat.opacity = 0.2;
        if (other.label) { other.label.visible = true; other.label.material.opacity = 0.8; }
      } else {
        // Unrelated — dim
        other.mesh.material.opacity = 0.08;
        other.glowMat.opacity = 0.02;
        if (other.label && !other.data._isHub) other.label.visible = false;
        if (other.label && other.data._isHub) other.label.material.opacity = 0.15;
      }
    });

    // Dim all edges, brighten connected ones
    const sEdges = window._structuralEdges || [];
    const mEdges = window._memoryLinkEdges || [];

    // Structural edges
    if (edgeLines) {
      edgeLines.material.opacity = 0.85;
      const colors = edgeLines.geometry.attributes.color.array;
      const connectedStructIdx = new Set(adj.structEdgeIdx);
      sEdges.forEach((e, i) => {
        const off = i * 6;
        if (connectedStructIdx.has(i)) {
          // Connected — bright white-ish glow
          const c = new THREE.Color(edgeColor(e));
          c.lerp(new THREE.Color(0xffffff), 0.4);
          colors[off] = c.r; colors[off+1] = c.g; colors[off+2] = c.b;
          colors[off+3] = c.r; colors[off+4] = c.g; colors[off+5] = c.b;
        } else {
          // Not connected — very dim
          colors[off] = 0.03; colors[off+1] = 0.05; colors[off+2] = 0.08;
          colors[off+3] = 0.03; colors[off+4] = 0.05; colors[off+5] = 0.08;
        }
      });
      edgeLines.geometry.attributes.color.needsUpdate = true;
    }

    // Memory link edges
    if (window._memLinkLines) {
      window._memLinkLines.material.opacity = 0.35;
      const colors = window._memLinkLines.geometry.attributes.color.array;
      const connectedMemIdx = new Set(adj.memEdgeIdx);
      mEdges.forEach((e, i) => {
        const off = i * 6;
        if (connectedMemIdx.has(i)) {
          colors[off] = 0; colors[off+1] = 0.7; colors[off+2] = 1.0;
          colors[off+3] = 0; colors[off+4] = 0.7; colors[off+5] = 1.0;
        } else {
          colors[off] = 0.02; colors[off+1] = 0.04; colors[off+2] = 0.06;
          colors[off+3] = 0.02; colors[off+4] = 0.04; colors[off+5] = 0.06;
        }
      });
      window._memLinkLines.geometry.attributes.color.needsUpdate = true;
    }

    // Update info panel
    updateInfoPanel(nm.data, connectedNodes);
  }

  function highlightEdgeByIndex(edgeData, group, drawIdx) {
    highlightActive = true;
    hoveredEdge = { edgeData, group, drawIdx };

    const si = edgeData._si;
    const ti = edgeData._ti;
    if (si === undefined || ti === undefined) return;

    const srcNode = graphNodes[si];
    const tgtNode = graphNodes[ti];
    const endpointIds = new Set([srcNode.id, tgtNode.id]);

    // Dim all nodes except endpoints
    nodeMeshes.forEach(nm => {
      if (endpointIds.has(nm.data.id)) {
        nm.mesh.material.opacity = 1.0;
        nm.glowMat.opacity = 0.3;
        if (nm.label) { nm.label.visible = true; nm.label.material.opacity = 1.0; }
      } else {
        nm.mesh.material.opacity = 0.08;
        nm.glowMat.opacity = 0.02;
        if (nm.label && !nm.data._isHub) nm.label.visible = false;
        if (nm.label && nm.data._isHub) nm.label.material.opacity = 0.15;
      }
    });

    // Dim all edges except the hovered one
    const sEdges = window._structuralEdges || [];
    const mEdges = window._memoryLinkEdges || [];

    if (edgeLines) {
      const colors = edgeLines.geometry.attributes.color.array;
      sEdges.forEach((e, i) => {
        const off = i * 6;
        if (group === 'structural' && i === drawIdx) {
          colors[off] = 1; colors[off+1] = 1; colors[off+2] = 1;
          colors[off+3] = 1; colors[off+4] = 1; colors[off+5] = 1;
        } else {
          colors[off] = 0.03; colors[off+1] = 0.05; colors[off+2] = 0.08;
          colors[off+3] = 0.03; colors[off+4] = 0.05; colors[off+5] = 0.08;
        }
      });
      edgeLines.geometry.attributes.color.needsUpdate = true;
    }

    if (window._memLinkLines) {
      const colors = window._memLinkLines.geometry.attributes.color.array;
      mEdges.forEach((e, i) => {
        const off = i * 6;
        if (group === 'memlink' && i === drawIdx) {
          colors[off] = 0; colors[off+1] = 1; colors[off+2] = 1;
          colors[off+3] = 0; colors[off+4] = 1; colors[off+5] = 1;
        } else {
          colors[off] = 0.02; colors[off+1] = 0.04; colors[off+2] = 0.06;
          colors[off+3] = 0.02; colors[off+4] = 0.04; colors[off+5] = 0.06;
        }
      });
      window._memLinkLines.geometry.attributes.color.needsUpdate = true;
    }
  }

  function updateInfoPanel(nodeData, connectedNodeIds) {
    const panel = document.getElementById('graph-info-panel');
    if (!panel) return;

    const neighbors = [];
    connectedNodeIds.forEach(nid => {
      const n = graphNodes.find(gn => gn.id === nid);
      if (n) neighbors.push(n);
    });

    // Sort: hubs first, then by importance
    neighbors.sort((a, b) => {
      if (a._isHub && !b._isHub) return -1;
      if (!a._isHub && b._isHub) return 1;
      return (b.importance || 5) - (a.importance || 5);
    });

    const colorMap = { entity: 'var(--purple)', project: 'var(--green)', memory: 'var(--accent)' };
    const typeIcon = { entity: '&#9650;', project: '&#9679;', memory: '&#9632;' };

    let html = `<div class="info-header">
      <span class="info-type" style="color:${colorMap[nodeData.type] || 'var(--accent)'}">${typeIcon[nodeData.type] || ''} ${(nodeData.type || '').toUpperCase()}</span>
      <span class="info-title">${esc(truncate(nodeData.label, 50))}</span>
    </div>`;

    if (nodeData.importance) {
      html += `<div class="info-importance"><span class="imp-bar" style="width:${nodeData.importance * 10}%"></span><span class="imp-text">IMP ${nodeData.importance}/10</span></div>`;
    }

    html += `<div class="info-connections-header">${neighbors.length} CONNECTION${neighbors.length !== 1 ? 'S' : ''}</div>`;
    html += '<div class="info-connections">';
    neighbors.slice(0, 12).forEach(n => {
      const c = colorMap[n.type] || 'var(--accent)';
      const icon = typeIcon[n.type] || '';
      html += `<div class="info-conn-item">
        <span class="info-conn-dot" style="color:${c}">${icon}</span>
        <span class="info-conn-label">${esc(truncate(n.label, 35))}</span>
        ${n.importance ? `<span class="info-conn-imp">${n.importance}</span>` : ''}
      </div>`;
    });
    if (neighbors.length > 12) {
      html += `<div class="info-conn-more">+${neighbors.length - 12} more</div>`;
    }
    html += '</div>';

    panel.innerHTML = html;
    panel.classList.add('visible');
  }

  function doRaycast() {
    if (!raycaster || mouse.x < -5) return;

    raycaster.setFromCamera(mouse, camera);
    const meshes = nodeMeshes.map(nm => nm.mesh);
    const intersects = raycaster.intersectObjects(meshes, false);

    const tooltip = document.getElementById('graph-tooltip');
    const canvas = document.getElementById('graph-canvas');

    // ── Check node hover first ──
    if (intersects.length > 0) {
      const hit = intersects[0].object;
      const nm = nodeMeshes.find(nm => nm.mesh === hit);
      if (nm) {
        if (hoveredNode !== nm) {
          clearHighlights();
          hoveredNode = nm;
          hoveredEdge = null;
          canvas.style.cursor = nm.data.type === 'memory' ? 'pointer' : 'default';

          // Highlight this node + connections
          highlightNode(nm);

          // Update tooltip
          const d = nm.data;
          let html = `<div class="tt-title">${esc(d.label)}</div>`;
          html += `<div class="tt-type">${d.type}${d.entity_type ? ' / ' + d.entity_type : ''}${d.memory_type ? ' / ' + d.memory_type : ''}</div>`;
          if (d.importance) html += `<div class="tt-content">Importance: ${d.importance}/10</div>`;
          const adj = adjacency[d.id];
          if (adj) html += `<div class="tt-content">${adj.nodes.size} connection${adj.nodes.size !== 1 ? 's' : ''}</div>`;
          if (d.tags) {
            const tags = parseTags(d.tags);
            if (tags.length) html += `<div class="tt-tags">${tags.map(t => `<span class="tag">${esc(t)}</span>`).join('')}</div>`;
          }
          tooltip.innerHTML = html;
          tooltip.classList.add('visible');
        }
        return;
      }
    }

    // ── Check edge hover (line proximity) ──
    const ray = raycaster.ray;
    const threshold = graphMode === '2d' ? 2.5 : 1.8;
    let closestEdge = null;
    let closestDist = threshold;
    let closestGroup = null;
    let closestIdx = -1;

    // Project mouse to a world point on the near plane for distance calcs
    const sEdges = window._structuralEdges || [];
    const mEdges = window._memoryLinkEdges || [];

    function checkEdges(edges, group) {
      for (let i = 0; i < edges.length; i++) {
        const e = edges[i];
        if (e._si === undefined || e._ti === undefined) continue;
        const s = graphNodes[e._si];
        const t = graphNodes[e._ti];

        // Project endpoints to screen and find closest point on line segment
        const sv = new THREE.Vector3(s.x, s.y, s.z);
        const tv = new THREE.Vector3(t.x, t.y, t.z);
        const mid = new THREE.Vector3().addVectors(sv, tv).multiplyScalar(0.5);

        // Distance from ray to line segment midpoint (rough but fast)
        const toMid = new THREE.Vector3().subVectors(mid, ray.origin);
        const proj = toMid.dot(ray.direction);
        if (proj < 0) continue; // behind camera

        const closestOnRay = new THREE.Vector3().copy(ray.direction).multiplyScalar(proj).add(ray.origin);
        // Now check distance from closestOnRay to the actual line segment
        const dist = pointToSegmentDist(
          closestOnRay.x, closestOnRay.y, closestOnRay.z,
          s.x, s.y, s.z, t.x, t.y, t.z
        );

        if (dist < closestDist) {
          closestDist = dist;
          closestEdge = e;
          closestGroup = group;
          closestIdx = i;
        }
      }
    }

    checkEdges(sEdges, 'structural');
    checkEdges(mEdges, 'memlink');

    if (closestEdge) {
      if (!hoveredEdge || hoveredEdge.drawIdx !== closestIdx || hoveredEdge.group !== closestGroup) {
        clearHighlights();
        hoveredNode = null;
        canvas.style.cursor = 'crosshair';

        highlightEdgeByIndex(closestEdge, closestGroup, closestIdx);

        // Edge tooltip
        const s = graphNodes[closestEdge._si];
        const t = graphNodes[closestEdge._ti];
        let html = `<div class="tt-title">${edgeTypeName(closestEdge.type)}</div>`;
        html += `<div class="tt-type">${esc(s.label)} &harr; ${esc(t.label)}</div>`;
        if (closestEdge.label) html += `<div class="tt-content">${esc(closestEdge.label)}</div>`;
        tooltip.innerHTML = html;
        tooltip.classList.add('visible');
      }
      return;
    }

    // ── Nothing hovered ──
    if (hoveredNode || hoveredEdge) {
      clearHighlights();
      hoveredNode = null;
      hoveredEdge = null;
      canvas.style.cursor = 'grab';
      tooltip.classList.remove('visible');
    }
  }

  function animate() {
    if (!graphInitialized) return;
    requestAnimationFrame(animate);

    const dt = clock.getDelta();
    const elapsed = clock.getElapsedTime();

    // Force simulation
    simulateForces();
    updatePositions();

    // Mode transition countdown
    if (modeTransition > 0) {
      modeTransition--;
    }

    // Pulse effect on hovered node
    pulsePhase = elapsed;
    if (hoveredNode && highlightActive) {
      const pulse = 0.3 + Math.sin(pulsePhase * 4) * 0.15;
      hoveredNode.glowMat.opacity = pulse;
      // Subtle scale pulse
      const s = 1.0 + Math.sin(pulsePhase * 3) * 0.05;
      hoveredNode.mesh.scale.set(s, s, s);
    }

    // Particles
    updateParticles(elapsed);

    // Raycasting (every 2 frames for smoother hover)
    frameCount++;
    if (frameCount % 2 === 0) {
      doRaycast();
    }

    // Controls
    controls.update();

    // Render with bloom
    composer.render();

    // FPS counter
    if (elapsed - lastFPSTime >= 1.0) {
      currentFPS = Math.round(frameCount / (elapsed - lastFPSTime));
      frameCount = 0;
      lastFPSTime = elapsed;
      document.getElementById('hud-fps').textContent = currentFPS;
    }
  }

  function resizeGraph() {
    if (!graphInitialized) return;
    const container = document.getElementById('graph-container');
    const width = container.clientWidth;
    const height = container.clientHeight;
    if (width === 0 || height === 0) return;

    camera.aspect = width / height;
    camera.updateProjectionMatrix();
    renderer.setSize(width, height);
    composer.setSize(width, height);
  }

  window.addEventListener('resize', () => {
    if (document.getElementById('view-graph').classList.contains('active')) {
      resizeGraph();
    }
  });

  // ─── Helpers ───
  function parseTags(val) {
    if (!val) return [];
    if (Array.isArray(val)) return val;
    try { return JSON.parse(val); } catch { return []; }
  }

  function esc(str) {
    if (str === null || str === undefined) return '';
    const div = document.createElement('div');
    div.textContent = String(str);
    return div.innerHTML;
  }

  function truncate(str, max) {
    if (!str) return '';
    return str.length > max ? str.slice(0, max) + '...' : str;
  }
})();
