// sabmemory dashboard
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
  let nodeMeshes = [];        // { mesh, data, glowSprite, coronaRing, label }
  let edgeMeshes = [];        // { line, data, curveOff }
  let memLinkMeshes = [];     // { line, data, curveOff }
  let particles = null;
  let starField = null;
  let raycaster, mouse;
  let hoveredNode = null;
  let hoveredEdge = null;
  let graphNodes = [];
  let graphEdges = [];
  let graphInitialized = false;
  let clock;
  let frameCount = 0;
  let lastFPSTime = 0;
  let currentFPS = 0;
  let graphMode = '3d';
  let adjacency = {};
  let highlightActive = false;

  // Layout state
  let layoutPhase = 0;        // 0=animating to positions, 1=settled
  let layoutProgress = 0;     // 0..1 animation progress
  let targetPositions = [];   // [{x,y,z}] final positions per node
  let startPositions = [];    // [{x,y,z}] initial random positions

  // ─── Constants ───
  const GOLDEN_ANGLE = 137.508 * (Math.PI / 180);
  const NUCLEUS_RADIUS = 55;
  const SPIRAL_SCALE = 8;
  const PARTICLE_COUNT = 200;
  const LAYOUT_DURATION = 2.0; // seconds to animate layout

  // ─── Color Palette (Cyberpunk) ───
  const COLORS = {
    memory:       0x00d4ff,
    entity:       0xaa55ff,
    project:      0x00ff88,
    obsolete:     0xff3355,
    forgotten:    0x2a4a5a,
    memoryLink:   0x00d4ff,
    entityAssoc:  0xaa55ff,
    projectAssoc: 0x00ff88,
    relationship: 0xff6622,
    entityProject:0xffaa00,
    background:   0x0f2240,
  };

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
    if (d.type === 'entity') return COLORS.entity;
    if (d.type === 'project') return COLORS.project;
    if (d.is_obsolete) return COLORS.obsolete;
    if (d.is_forgotten) return COLORS.forgotten;
    return COLORS.memory;
  }

  function nodeRadius(d) {
    if (d.type === 'entity') return 3.5;
    if (d.type === 'project') return 4.0;
    return 1.2 + (d.importance || 5) * 0.18;
  }

  function edgeColor(d) {
    const t = d.type || '';
    if (t === 'entity_assoc') return COLORS.entityAssoc;
    if (t === 'project_assoc') return COLORS.projectAssoc;
    if (t === 'relationship') return COLORS.relationship;
    if (t === 'entity_project') return COLORS.entityProject;
    return COLORS.memoryLink;
  }

  function edgeTypeName(t) {
    if (t === 'entity_assoc') return 'Entity Link';
    if (t === 'project_assoc') return 'Project Link';
    if (t === 'relationship') return 'Relationship';
    if (t === 'entity_project') return 'Entity-Project';
    if (t === 'memory_link') return 'Memory Link';
    return t;
  }

  // ─── Glow Texture Generation ───
  function createGlowTexture(color, size) {
    const canvas = document.createElement('canvas');
    canvas.width = size;
    canvas.height = size;
    const ctx = canvas.getContext('2d');
    const half = size / 2;
    const gradient = ctx.createRadialGradient(half, half, 0, half, half, half);
    const c = new THREE.Color(color);
    const r = Math.round(c.r * 255);
    const g = Math.round(c.g * 255);
    const b = Math.round(c.b * 255);
    gradient.addColorStop(0, `rgba(${r},${g},${b},0.6)`);
    gradient.addColorStop(0.3, `rgba(${r},${g},${b},0.2)`);
    gradient.addColorStop(0.7, `rgba(${r},${g},${b},0.05)`);
    gradient.addColorStop(1, `rgba(${r},${g},${b},0)`);
    ctx.fillStyle = gradient;
    ctx.fillRect(0, 0, size, size);
    const tex = new THREE.CanvasTexture(canvas);
    tex.needsUpdate = true;
    return tex;
  }

  // ─── Text Sprite (Orbitron + Neon Glow) ───
  function createTextSprite(text, color, fontSize) {
    const canvas = document.createElement('canvas');
    const ctx = canvas.getContext('2d');
    const dpr = 2;
    const sz = (fontSize || 40) * dpr;
    const pad = 16 * dpr;
    ctx.font = `bold ${sz}px Orbitron, sans-serif`;
    const metrics = ctx.measureText(text);
    const w = Math.ceil(metrics.width) + pad * 2;
    const h = sz + pad * 2;
    canvas.width = w;
    canvas.height = h;

    // Subtle background
    ctx.fillStyle = 'rgba(15, 34, 64, 0.6)';
    const rx = pad * 0.4, ry = pad * 0.4;
    const rw = w - pad * 0.8, rh = h - pad * 0.8;
    const cr = 8 * dpr;
    ctx.beginPath();
    ctx.moveTo(rx + cr, ry);
    ctx.lineTo(rx + rw - cr, ry);
    ctx.quadraticCurveTo(rx + rw, ry, rx + rw, ry + cr);
    ctx.lineTo(rx + rw, ry + rh - cr);
    ctx.quadraticCurveTo(rx + rw, ry + rh, rx + rw - cr, ry + rh);
    ctx.lineTo(rx + cr, ry + rh);
    ctx.quadraticCurveTo(rx, ry + rh, rx, ry + rh - cr);
    ctx.lineTo(rx, ry + cr);
    ctx.quadraticCurveTo(rx, ry, rx + cr, ry);
    ctx.fill();

    // Text with neon glow
    ctx.font = `bold ${sz}px Orbitron, sans-serif`;
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.shadowColor = color || '#00d4ff';
    ctx.shadowBlur = 16 * dpr;
    ctx.fillStyle = color || '#00d4ff';
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
    const scaleFactor = dpr * 10;
    sprite.scale.set(w / scaleFactor, h / scaleFactor, 1);
    return sprite;
  }

  // ─── Quadratic Bezier Curve for Edges ───
  function createBezierEdge(sx, sy, sz, tx, ty, tz, curveOff, color, opacity) {
    const mx = (sx + tx) / 2;
    const my = (sy + ty) / 2;
    const mz = (sz + tz) / 2;

    // Perpendicular offset for curve
    const dx = tx - sx;
    const dy = ty - sy;
    const dz = tz - sz;
    const len = Math.sqrt(dx * dx + dy * dy + dz * dz) || 1;

    // Find a perpendicular vector
    let px, py, pz;
    if (Math.abs(dx) < Math.abs(dy)) {
      px = 0; py = -dz; pz = dy;
    } else {
      px = -dz; py = 0; pz = dx;
    }
    const plen = Math.sqrt(px * px + py * py + pz * pz) || 1;
    px /= plen; py /= plen; pz /= plen;

    const cx = mx + px * curveOff;
    const cy = my + py * curveOff;
    const cz = mz + pz * curveOff;

    const curve = new THREE.QuadraticBezierCurve3(
      new THREE.Vector3(sx, sy, sz),
      new THREE.Vector3(cx, cy, cz),
      new THREE.Vector3(tx, ty, tz)
    );

    const points = curve.getPoints(20);
    const geometry = new THREE.BufferGeometry().setFromPoints(points);
    const material = new THREE.LineBasicMaterial({
      color: color,
      transparent: true,
      opacity: opacity,
      linewidth: 1,
    });
    return new THREE.Line(geometry, material);
  }

  function updateBezierEdge(line, sx, sy, sz, tx, ty, tz, curveOff) {
    const mx = (sx + tx) / 2;
    const my = (sy + ty) / 2;
    const mz = (sz + tz) / 2;

    const dx = tx - sx;
    const dy = ty - sy;
    const dz = tz - sz;

    let px, py, pz;
    if (Math.abs(dx) < Math.abs(dy)) {
      px = 0; py = -dz; pz = dy;
    } else {
      px = -dz; py = 0; pz = dx;
    }
    const plen = Math.sqrt(px * px + py * py + pz * pz) || 1;
    px /= plen; py /= plen; pz /= plen;

    const cx = mx + px * curveOff;
    const cy = my + py * curveOff;
    const cz = mz + pz * curveOff;

    const curve = new THREE.QuadraticBezierCurve3(
      new THREE.Vector3(sx, sy, sz),
      new THREE.Vector3(cx, cy, cz),
      new THREE.Vector3(tx, ty, tz)
    );

    const points = curve.getPoints(20);
    const positions = line.geometry.attributes.position;
    for (let i = 0; i < points.length && i < positions.count; i++) {
      positions.setXYZ(i, points[i].x, points[i].y, points[i].z);
    }
    positions.needsUpdate = true;
  }

  // ─── Adjacency ───
  function buildAdjacency() {
    adjacency = {};
    graphNodes.forEach(n => {
      adjacency[n.id] = { nodes: new Set(), structEdges: [], memEdges: [] };
    });
    edgeMeshes.forEach((em, i) => {
      const e = em.data;
      if (e._si === undefined || e._ti === undefined) return;
      const sId = graphNodes[e._si].id;
      const tId = graphNodes[e._ti].id;
      adjacency[sId].nodes.add(tId);
      adjacency[tId].nodes.add(sId);
      adjacency[sId].structEdges.push(i);
      adjacency[tId].structEdges.push(i);
    });
    memLinkMeshes.forEach((em, i) => {
      const e = em.data;
      if (e._si === undefined || e._ti === undefined) return;
      const sId = graphNodes[e._si].id;
      const tId = graphNodes[e._ti].id;
      adjacency[sId].nodes.add(tId);
      adjacency[tId].nodes.add(sId);
      adjacency[sId].memEdges.push(i);
      adjacency[tId].memEdges.push(i);
    });
  }

  // ─── Orbital Layout ───
  function computeOrbitalLayout() {
    const is2D = graphMode === '2d';

    // Identify hubs (entities + projects) and memory nodes
    const hubs = graphNodes.filter(n => n.type === 'entity' || n.type === 'project');
    const memoryNodes = graphNodes.filter(n => n.type === 'memory');

    // Build structural edge list for hub-child mapping
    const structEdges = graphEdges.filter(e => e.type !== 'memory_link');

    // Map memories to parent hubs
    const idToIdx = {};
    graphNodes.forEach((n, i) => { idToIdx[n.id] = i; });

    const memoryToHub = {};
    const hubChildren = {};
    hubs.forEach(h => { hubChildren[h.id] = []; });

    structEdges.forEach(e => {
      const si = idToIdx[e.source];
      const ti = idToIdx[e.target];
      if (si === undefined || ti === undefined) return;
      const sNode = graphNodes[si];
      const tNode = graphNodes[ti];

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

    const orphans = memoryNodes.filter(m => !memoryToHub[m.id]);

    // Position hubs at golden-angle intervals
    const positions = new Array(graphNodes.length);

    hubs.forEach((h, i) => {
      const angle = i * GOLDEN_ANGLE;
      const idx = idToIdx[h.id];
      if (is2D) {
        positions[idx] = {
          x: Math.cos(angle) * NUCLEUS_RADIUS,
          y: Math.sin(angle) * NUCLEUS_RADIUS,
          z: 0
        };
      } else {
        positions[idx] = {
          x: Math.cos(angle) * NUCLEUS_RADIUS,
          y: (i % 2 === 0 ? 1 : -1) * 12,
          z: Math.sin(angle) * NUCLEUS_RADIUS
        };
      }
      h._isHub = true;
    });

    // Position children in Fermat spiral around their hub
    hubs.forEach(h => {
      const children = hubChildren[h.id];
      const hubPos = positions[idToIdx[h.id]];

      children.forEach((cid, ci) => {
        const idx = idToIdx[cid];
        if (idx === undefined) return;
        const angle = ci * GOLDEN_ANGLE;
        const r = SPIRAL_SCALE * Math.sqrt(ci + 1);

        if (is2D) {
          positions[idx] = {
            x: hubPos.x + Math.cos(angle) * r,
            y: hubPos.y + Math.sin(angle) * r,
            z: 0
          };
        } else {
          // In 3D, add slight Z variation based on importance
          const imp = graphNodes[idx].importance || 5;
          positions[idx] = {
            x: hubPos.x + Math.cos(angle) * r,
            y: hubPos.y + Math.sin(angle) * r * 0.8,
            z: hubPos.z + (imp - 5) * 1.5
          };
        }
      });
    });

    // Position orphans in Fermat spiral around center
    orphans.forEach((o, i) => {
      const idx = idToIdx[o.id];
      const angle = i * GOLDEN_ANGLE;
      const r = SPIRAL_SCALE * Math.sqrt(i + 1) * 0.8;

      if (is2D) {
        positions[idx] = { x: Math.cos(angle) * r, y: Math.sin(angle) * r, z: 0 };
      } else {
        positions[idx] = {
          x: Math.cos(angle) * r,
          y: Math.sin(angle) * r * 0.8,
          z: (Math.random() - 0.5) * 8
        };
      }
    });

    // Fill any missing positions (shouldn't happen, but safety)
    for (let i = 0; i < graphNodes.length; i++) {
      if (!positions[i]) {
        positions[i] = { x: (Math.random() - 0.5) * 20, y: (Math.random() - 0.5) * 20, z: is2D ? 0 : (Math.random() - 0.5) * 20 };
      }
    }

    return positions;
  }

  // ─── Init Graph ───
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
    scene.background = new THREE.Color(COLORS.background);

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
    renderer.toneMappingExposure = 1.0;

    // ─── Post-processing (softer Bloom) ───
    composer = new THREE.EffectComposer(renderer);
    composer.addPass(new THREE.RenderPass(scene, camera));
    const bloomPass = new THREE.UnrealBloomPass(
      new THREE.Vector2(width, height),
      0.4,   // strength (was 0.8)
      0.3,   // radius (was 0.4)
      0.6    // threshold (was 0.5)
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

    // ─── Lighting ───
    const hemiLight = new THREE.HemisphereLight(0x4466aa, 0x222244, 0.6);
    scene.add(hemiLight);
    const ambientLight = new THREE.AmbientLight(0x223344, 0.3);
    scene.add(ambientLight);

    // ─── Background (procedural star field + gradient) ───
    createBackground();

    // ─── Build Graph ───
    buildGraphObjects();

    // ─── Particles ───
    createParticles();

    // ─── Event Listeners ───
    const tooltip = document.getElementById('graph-tooltip');

    canvas.addEventListener('mousemove', (e) => {
      const rect = canvas.getBoundingClientRect();
      mouse.x = ((e.clientX - rect.left) / rect.width) * 2 - 1;
      mouse.y = -((e.clientY - rect.top) / rect.height) * 2 + 1;

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
      minImpVal.textContent = minImp.value;
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

  function createBackground() {
    // Procedural galaxy background sphere
    const bgGeo = new THREE.SphereGeometry(800, 64, 64);
    const bgMat = new THREE.ShaderMaterial({
      side: THREE.BackSide,
      uniforms: {
        uTime: { value: 0.0 },
      },
      vertexShader: `
        varying vec3 vWorldPos;
        varying vec2 vUv;
        void main() {
          vUv = uv;
          vec4 worldPos = modelMatrix * vec4(position, 1.0);
          vWorldPos = worldPos.xyz;
          gl_Position = projectionMatrix * viewMatrix * worldPos;
        }
      `,
      fragmentShader: `
        uniform float uTime;
        varying vec3 vWorldPos;
        varying vec2 vUv;

        // Hash functions for noise
        float hash(vec2 p) {
          return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
        }

        float hash3(vec3 p) {
          return fract(sin(dot(p, vec3(127.1, 311.7, 74.7))) * 43758.5453);
        }

        // 2D noise
        float noise(vec2 p) {
          vec2 i = floor(p);
          vec2 f = fract(p);
          f = f * f * (3.0 - 2.0 * f);
          float a = hash(i);
          float b = hash(i + vec2(1.0, 0.0));
          float c = hash(i + vec2(0.0, 1.0));
          float d = hash(i + vec2(1.0, 1.0));
          return mix(mix(a, b, f.x), mix(c, d, f.x), f.y);
        }

        // FBM (Fractal Brownian Motion)
        float fbm(vec2 p) {
          float v = 0.0;
          float a = 0.5;
          mat2 rot = mat2(0.8, 0.6, -0.6, 0.8);
          for (int i = 0; i < 5; i++) {
            v += a * noise(p);
            p = rot * p * 2.0;
            a *= 0.5;
          }
          return v;
        }

        void main() {
          vec3 dir = normalize(vWorldPos);

          // Spherical coordinates
          float theta = atan(dir.z, dir.x);
          float phi = acos(dir.y);

          // UV from spherical coords
          vec2 uv = vec2(theta / 6.28318 + 0.5, phi / 3.14159);

          // Base color: medium navy gradient
          vec3 bgCol1 = vec3(0.059, 0.133, 0.251); // #0f2240
          vec3 bgCol2 = vec3(0.086, 0.157, 0.314); // #162850
          vec3 col = mix(bgCol2, bgCol1, dir.y * 0.5 + 0.5);

          // Spiral arm structure
          float spiralAngle = theta + dir.y * 1.5;
          float spiral = sin(spiralAngle * 2.0 + uTime * 0.02) * 0.5 + 0.5;
          spiral *= sin(spiralAngle * 3.0 - uTime * 0.015) * 0.5 + 0.5;
          spiral = pow(spiral, 2.0) * 0.3;

          // Nebula clouds using FBM
          vec2 nebUV = uv * 4.0 + uTime * 0.005;
          float neb1 = fbm(nebUV);
          float neb2 = fbm(nebUV * 1.5 + 3.7);
          float neb3 = fbm(nebUV * 0.8 + 7.3);

          // Purple nebula patches
          vec3 nebPurple = vec3(0.25, 0.1, 0.4) * pow(neb1, 2.5) * 0.5;

          // Blue nebula patches
          vec3 nebBlue = vec3(0.05, 0.15, 0.35) * pow(neb2, 2.0) * 0.6;

          // Pink/magenta wisps
          vec3 nebPink = vec3(0.3, 0.05, 0.2) * pow(neb3, 3.0) * 0.3;

          // Combine nebula with spiral structure
          col += (nebPurple + nebBlue + nebPink) * (0.5 + spiral);

          // Star density modulated by spiral arms
          float starDensity = 0.3 + spiral * 0.5;

          // Layer 1: tiny dim stars (many)
          vec2 starUV1 = uv * 200.0;
          float star1 = hash(floor(starUV1));
          star1 = step(1.0 - starDensity * 0.015, star1);
          col += vec3(0.6, 0.7, 0.9) * star1 * 0.3;

          // Layer 2: medium stars
          vec2 starUV2 = uv * 80.0;
          float star2 = hash(floor(starUV2) + 47.0);
          star2 = step(1.0 - starDensity * 0.008, star2);
          float starBright2 = hash(floor(starUV2) + 91.0);
          col += vec3(0.7, 0.8, 1.0) * star2 * (0.4 + starBright2 * 0.3);

          // Layer 3: bright accent stars (rare, twinkling)
          vec2 starUV3 = uv * 30.0;
          float star3 = hash(floor(starUV3) + 137.0);
          star3 = step(0.992, star3);
          float twinkle = sin(uTime * 2.0 + hash(floor(starUV3)) * 6.28) * 0.5 + 0.5;
          vec3 starCol3 = mix(vec3(0.5, 0.7, 1.0), vec3(0.8, 0.6, 1.0), hash(floor(starUV3) + 200.0));
          col += starCol3 * star3 * (0.5 + twinkle * 0.5);

          // Subtle galactic core glow at center
          float coreDist = length(dir.xz);
          float coreGlow = exp(-coreDist * coreDist * 3.0) * 0.15;
          col += vec3(0.15, 0.2, 0.4) * coreGlow;

          gl_FragColor = vec4(col, 1.0);
        }
      `,
    });
    const bgMesh = new THREE.Mesh(bgGeo, bgMat);
    bgMesh.userData.isBgSphere = true;
    scene.add(bgMesh);

    // Additional foreground star layers (Three.js Points)
    // Layer 1: tiny white stars close to graph
    const starCount1 = 500;
    const starPos1 = new Float32Array(starCount1 * 3);
    for (let i = 0; i < starCount1; i++) {
      const theta = Math.random() * Math.PI * 2;
      const phi = Math.acos(2 * Math.random() - 1);
      const r = 200 + Math.random() * 300;
      starPos1[i * 3] = r * Math.sin(phi) * Math.cos(theta);
      starPos1[i * 3 + 1] = r * Math.sin(phi) * Math.sin(theta);
      starPos1[i * 3 + 2] = r * Math.cos(phi);
    }
    const starGeo1 = new THREE.BufferGeometry();
    starGeo1.setAttribute('position', new THREE.BufferAttribute(starPos1, 3));
    const starMat1 = new THREE.PointsMaterial({
      color: 0xaaccff,
      size: 0.3,
      transparent: true,
      opacity: 0.3,
      sizeAttenuation: true,
      depthWrite: false,
    });
    const stars1 = new THREE.Points(starGeo1, starMat1);
    scene.add(stars1);

    // Layer 2: larger cyan-tinted accent stars
    const starCount2 = 80;
    const starPos2 = new Float32Array(starCount2 * 3);
    for (let i = 0; i < starCount2; i++) {
      const theta = Math.random() * Math.PI * 2;
      const phi = Math.acos(2 * Math.random() - 1);
      const r = 180 + Math.random() * 350;
      starPos2[i * 3] = r * Math.sin(phi) * Math.cos(theta);
      starPos2[i * 3 + 1] = r * Math.sin(phi) * Math.sin(theta);
      starPos2[i * 3 + 2] = r * Math.cos(phi);
    }
    const starGeo2 = new THREE.BufferGeometry();
    starGeo2.setAttribute('position', new THREE.BufferAttribute(starPos2, 3));
    const starMat2 = new THREE.PointsMaterial({
      color: 0x00d4ff,
      size: 0.6,
      transparent: true,
      opacity: 0.4,
      sizeAttenuation: true,
      depthWrite: false,
      blending: THREE.AdditiveBlending,
    });
    starField = new THREE.Points(starGeo2, starMat2);
    scene.add(starField);
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

    // Filter + classify edges
    const allEdges = graphData.edges.filter(e =>
      nodeIdSet.has(e.source) && nodeIdSet.has(e.target)
    ).map(e => ({ ...e }));

    const structuralEdges = [];
    const memoryLinkEdges = [];
    allEdges.forEach(e => {
      if (e.type === 'memory_link') memoryLinkEdges.push(e);
      else structuralEdges.push(e);
    });
    graphEdges = allEdges;

    // Build ID -> index map
    const idToIdx = {};
    graphNodes.forEach((n, i) => { idToIdx[n.id] = i; });

    // Tag edges with indices
    allEdges.forEach(e => {
      e._si = idToIdx[e.source];
      e._ti = idToIdx[e.target];
    });

    // ─── Compute orbital layout ───
    targetPositions = computeOrbitalLayout();

    // Set random start positions for animation
    startPositions = graphNodes.map(() => ({
      x: (Math.random() - 0.5) * 100,
      y: (Math.random() - 0.5) * 100,
      z: graphMode === '2d' ? 0 : (Math.random() - 0.5) * 100,
    }));

    // Set initial positions
    graphNodes.forEach((n, i) => {
      n.x = startPositions[i].x;
      n.y = startPositions[i].y;
      n.z = startPositions[i].z;
    });

    layoutPhase = 0;
    layoutProgress = 0;

    // ─── Clean up old scene objects ───
    nodeMeshes.forEach(nm => {
      scene.remove(nm.mesh);
      if (nm.glowSprite) scene.remove(nm.glowSprite);
      if (nm.coronaRing) scene.remove(nm.coronaRing);
      if (nm.label) scene.remove(nm.label);
    });
    nodeMeshes = [];

    edgeMeshes.forEach(em => scene.remove(em.line));
    edgeMeshes = [];
    memLinkMeshes.forEach(em => scene.remove(em.line));
    memLinkMeshes = [];

    // ─── Create Node Meshes ───
    graphNodes.forEach((n, i) => {
      const r = nodeRadius(n);
      const color = nodeColor(n);
      const isHub = n.type === 'entity' || n.type === 'project';
      n._isHub = isHub;

      // Orb mesh
      const geo = new THREE.SphereGeometry(r, 20, 14);
      const mat = new THREE.MeshStandardMaterial({
        color: color,
        emissive: color,
        emissiveIntensity: 0.4,
        roughness: 0.3,
        metalness: 0.1,
        transparent: true,
        opacity: (n.is_obsolete || n.is_forgotten) ? 0.35 : 0.9,
      });
      const mesh = new THREE.Mesh(geo, mat);
      mesh.position.set(n.x, n.y, n.z);
      scene.add(mesh);

      // Glow sprite
      const glowTex = createGlowTexture(color, 128);
      const glowMat = new THREE.SpriteMaterial({
        map: glowTex,
        transparent: true,
        opacity: isHub ? 0.25 : 0.18,
        blending: THREE.AdditiveBlending,
        depthWrite: false,
        depthTest: false,
      });
      const glowSprite = new THREE.Sprite(glowMat);
      const glowScale = r * 5;
      glowSprite.scale.set(glowScale, glowScale, 1);
      glowSprite.position.copy(mesh.position);
      scene.add(glowSprite);

      // Corona ring (hubs only)
      let coronaRing = null;
      if (isHub) {
        const ringGeo = new THREE.RingGeometry(r * 1.8, r * 2.3, 32);
        const ringMat = new THREE.MeshBasicMaterial({
          color: color,
          transparent: true,
          opacity: 0.12,
          side: THREE.DoubleSide,
          blending: THREE.AdditiveBlending,
          depthWrite: false,
        });
        coronaRing = new THREE.Mesh(ringGeo, ringMat);
        coronaRing.position.copy(mesh.position);
        scene.add(coronaRing);
      }

      // Labels
      let label;
      if (isHub) {
        const labelColor = n.type === 'entity' ? '#aa55ff' : '#00ff88';
        label = createTextSprite(n.label, labelColor, 40);
        label.position.set(n.x, n.y + r + 4, n.z);
        label.visible = true;
        scene.add(label);
      } else {
        label = createTextSprite(truncate(n.label, 40), '#e2e8f0', 36);
        label.position.set(n.x, n.y + r + 3, n.z);
        label.visible = false;
        scene.add(label);
      }

      nodeMeshes.push({
        mesh, data: n, glowSprite, glowMat, coronaRing, label,
        baseGlowOpacity: isHub ? 0.25 : 0.18,
        _orbitalAngle: Math.random() * Math.PI * 2,
        _orbitalSpeed: (0.0005 + Math.random() * 0.001) * (Math.random() < 0.5 ? 1 : -1),
      });
    });

    // ─── Create Bezier Edges ───
    // Structural edges
    structuralEdges.forEach(e => {
      if (e._si === undefined || e._ti === undefined) return;
      const s = graphNodes[e._si];
      const t = graphNodes[e._ti];
      const curveOff = (3 + Math.random() * 5) * (Math.random() < 0.5 ? 1 : -1);
      const line = createBezierEdge(s.x, s.y, s.z, t.x, t.y, t.z, curveOff, edgeColor(e), 0.4);
      scene.add(line);
      edgeMeshes.push({ line, data: e, curveOff });
    });

    // Memory link edges
    memoryLinkEdges.forEach(e => {
      if (e._si === undefined || e._ti === undefined) return;
      const s = graphNodes[e._si];
      const t = graphNodes[e._ti];
      const curveOff = (2 + Math.random() * 4) * (Math.random() < 0.5 ? 1 : -1);
      const line = createBezierEdge(s.x, s.y, s.z, t.x, t.y, t.z, curveOff, COLORS.memoryLink, 0.12);
      scene.add(line);
      memLinkMeshes.push({ line, data: e, curveOff });
    });

    // Build adjacency
    buildAdjacency();

    // Update HUD
    document.getElementById('hud-nodes').textContent = graphNodes.length;
    document.getElementById('hud-edges').textContent = allEdges.length;
  }

  function rebuildGraphObjects() {
    buildGraphObjects();
  }

  // ─── Mode Switching ───
  function switchGraphMode(mode) {
    graphMode = mode;
    rebuildGraphObjects();

    if (mode === '2d') {
      controls.enableRotate = false;
      controls.panSpeed = 1.5;
      camera.up.set(0, 1, 0);
      animateCameraTo(0, 0, 150);
    } else {
      controls.enableRotate = true;
      controls.panSpeed = 1.0;
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

    // Ambient dust particles
    const positions = new Float32Array(PARTICLE_COUNT * 3);
    const alphas = new Float32Array(PARTICLE_COUNT);

    for (let i = 0; i < PARTICLE_COUNT; i++) {
      positions[i * 3]     = (Math.random() - 0.5) * 300;
      positions[i * 3 + 1] = (Math.random() - 0.5) * 300;
      positions[i * 3 + 2] = (Math.random() - 0.5) * 300;
      alphas[i] = Math.random();
    }

    const geo = new THREE.BufferGeometry();
    geo.setAttribute('position', new THREE.BufferAttribute(positions, 3));

    const mat = new THREE.PointsMaterial({
      color: 0x00d4ff,
      size: 0.3,
      transparent: true,
      opacity: 0.15,
      sizeAttenuation: true,
      blending: THREE.AdditiveBlending,
      depthWrite: false,
    });

    particles = new THREE.Points(geo, mat);
    scene.add(particles);
    particles._alphas = alphas;
  }

  // ─── Layout Animation ───
  function updateLayout(dt) {
    if (layoutPhase === 1) return; // settled

    layoutProgress += dt / LAYOUT_DURATION;
    if (layoutProgress >= 1.0) {
      layoutProgress = 1.0;
      layoutPhase = 1;
    }

    const t = easeInOutCubic(layoutProgress);

    for (let i = 0; i < graphNodes.length; i++) {
      const n = graphNodes[i];
      n.x = startPositions[i].x + (targetPositions[i].x - startPositions[i].x) * t;
      n.y = startPositions[i].y + (targetPositions[i].y - startPositions[i].y) * t;
      n.z = startPositions[i].z + (targetPositions[i].z - startPositions[i].z) * t;
    }
  }

  // ─── Orbital Motion (post-layout) ───
  function updateOrbitalMotion(dt) {
    if (layoutPhase !== 1) return; // only after settled

    for (let i = 0; i < nodeMeshes.length; i++) {
      const nm = nodeMeshes[i];
      if (nm.data._isHub) continue; // hubs stay still

      nm._orbitalAngle += nm._orbitalSpeed;
      const target = targetPositions[i];
      // Very subtle orbital wobble
      const wobble = 0.5;
      nm.data.x = target.x + Math.cos(nm._orbitalAngle) * wobble;
      nm.data.y = target.y + Math.sin(nm._orbitalAngle) * wobble;
      if (graphMode !== '2d') {
        nm.data.z = target.z + Math.sin(nm._orbitalAngle * 0.7) * wobble * 0.5;
      }
    }
  }

  function updatePositions() {
    // Update node meshes + labels + glows + coronas
    for (let i = 0; i < nodeMeshes.length; i++) {
      const nm = nodeMeshes[i];
      const n = nm.data;
      nm.mesh.position.set(n.x, n.y, n.z);
      nm.glowSprite.position.set(n.x, n.y, n.z);
      if (nm.coronaRing) {
        nm.coronaRing.position.set(n.x, n.y, n.z);
        nm.coronaRing.lookAt(camera.position);
      }
      if (nm.label) {
        const r = nodeRadius(n);
        const yOff = n._isHub ? r + 4 : r + 3;
        nm.label.position.set(n.x, n.y + yOff, n.z);
      }
    }

    // Update Bezier edges
    edgeMeshes.forEach(em => {
      const e = em.data;
      if (e._si === undefined || e._ti === undefined) return;
      const s = graphNodes[e._si];
      const t = graphNodes[e._ti];
      updateBezierEdge(em.line, s.x, s.y, s.z, t.x, t.y, t.z, em.curveOff);
    });

    memLinkMeshes.forEach(em => {
      const e = em.data;
      if (e._si === undefined || e._ti === undefined) return;
      const s = graphNodes[e._si];
      const t = graphNodes[e._ti];
      updateBezierEdge(em.line, s.x, s.y, s.z, t.x, t.y, t.z, em.curveOff);
    });
  }

  function updateAnimations(elapsed) {
    // Glow pulsing
    for (let i = 0; i < nodeMeshes.length; i++) {
      const nm = nodeMeshes[i];
      if (!highlightActive || hoveredNode === nm) {
        const pulse = nm.baseGlowOpacity + Math.sin(elapsed * 1.5 + i * 0.5) * 0.05;
        nm.glowMat.opacity = Math.max(0.05, pulse);
      }
    }

    // Corona rotation
    for (let i = 0; i < nodeMeshes.length; i++) {
      const nm = nodeMeshes[i];
      if (nm.coronaRing) {
        nm.coronaRing.rotation.z += 0.003;
      }
    }

    // Star twinkle
    if (starField) {
      starField.material.opacity = 0.35 + Math.sin(elapsed * 0.5) * 0.1;
    }
  }

  function updateParticles(elapsed) {
    if (!particles) return;
    const pos = particles.geometry.attributes.position.array;
    const alphas = particles._alphas;
    for (let i = 0; i < PARTICLE_COUNT; i++) {
      const i3 = i * 3;
      pos[i3]     += Math.sin(alphas[i] * 6.28 + elapsed * 0.2) * 0.008;
      pos[i3 + 1] += Math.cos(alphas[i] * 3.14 + elapsed * 0.15) * 0.008;
      pos[i3 + 2] += Math.sin(alphas[i] * 4.71 + elapsed * 0.18) * 0.006;
      alphas[i] += 0.0005;
    }
    particles.geometry.attributes.position.needsUpdate = true;
  }

  // ─── Highlight Logic ───
  function clearHighlights() {
    if (!highlightActive) return;
    highlightActive = false;
    hoveredEdge = null;

    nodeMeshes.forEach(nm => {
      const isObs = nm.data.is_obsolete || nm.data.is_forgotten;
      nm.mesh.material.opacity = isObs ? 0.35 : 0.9;
      nm.mesh.material.emissiveIntensity = 0.4;
      nm.mesh.scale.set(1, 1, 1);
      nm.glowMat.opacity = nm.baseGlowOpacity;
      if (nm.label && !nm.data._isHub) nm.label.visible = false;
      if (nm.label && nm.data._isHub) nm.label.material.opacity = 0.9;
    });

    edgeMeshes.forEach(em => {
      em.line.material.opacity = 0.4;
      em.line.material.color.set(edgeColor(em.data));
    });
    memLinkMeshes.forEach(em => {
      em.line.material.opacity = 0.12;
      em.line.material.color.set(COLORS.memoryLink);
    });

    const infoPanel = document.getElementById('graph-info-panel');
    if (infoPanel) infoPanel.classList.remove('visible');
  }

  function highlightNode(nm) {
    highlightActive = true;
    const nodeId = nm.data.id;
    const adj = adjacency[nodeId];
    if (!adj) return;

    const connectedNodes = adj.nodes;
    const connectedStructEdges = new Set(adj.structEdges);
    const connectedMemEdges = new Set(adj.memEdges);

    // Dim all nodes, brighten connected ones
    nodeMeshes.forEach(other => {
      if (other === nm) {
        other.mesh.material.opacity = 1.0;
        other.mesh.material.emissiveIntensity = 0.8;
        other.glowMat.opacity = 0.5;
        if (other.label) { other.label.visible = true; other.label.material.opacity = 1.0; }
      } else if (connectedNodes.has(other.data.id)) {
        other.mesh.material.opacity = 0.85;
        other.mesh.material.emissiveIntensity = 0.5;
        other.glowMat.opacity = 0.25;
        if (other.label) { other.label.visible = true; other.label.material.opacity = 0.8; }
      } else {
        other.mesh.material.opacity = 0.06;
        other.mesh.material.emissiveIntensity = 0.1;
        other.glowMat.opacity = 0.02;
        if (other.label && !other.data._isHub) other.label.visible = false;
        if (other.label && other.data._isHub) other.label.material.opacity = 0.15;
      }
    });

    // Dim all edges, brighten connected ones
    edgeMeshes.forEach((em, i) => {
      if (connectedStructEdges.has(i)) {
        em.line.material.opacity = 0.7;
        const c = new THREE.Color(edgeColor(em.data));
        c.lerp(new THREE.Color(0xffffff), 0.3);
        em.line.material.color.copy(c);
      } else {
        em.line.material.opacity = 0.03;
        em.line.material.color.set(0x111522);
      }
    });

    memLinkMeshes.forEach((em, i) => {
      if (connectedMemEdges.has(i)) {
        em.line.material.opacity = 0.35;
        em.line.material.color.set(0x818cf8);
      } else {
        em.line.material.opacity = 0.02;
        em.line.material.color.set(0x111522);
      }
    });

    updateInfoPanel(nm.data, connectedNodes);
  }

  function highlightEdge(edgeData, edgeMesh, group) {
    highlightActive = true;
    hoveredEdge = { edgeData, edgeMesh, group };

    const si = edgeData._si;
    const ti = edgeData._ti;
    if (si === undefined || ti === undefined) return;

    const endpointIds = new Set([graphNodes[si].id, graphNodes[ti].id]);

    nodeMeshes.forEach(nm => {
      if (endpointIds.has(nm.data.id)) {
        nm.mesh.material.opacity = 1.0;
        nm.mesh.material.emissiveIntensity = 0.7;
        nm.glowMat.opacity = 0.4;
        if (nm.label) { nm.label.visible = true; nm.label.material.opacity = 1.0; }
      } else {
        nm.mesh.material.opacity = 0.06;
        nm.mesh.material.emissiveIntensity = 0.1;
        nm.glowMat.opacity = 0.02;
        if (nm.label && !nm.data._isHub) nm.label.visible = false;
        if (nm.label && nm.data._isHub) nm.label.material.opacity = 0.15;
      }
    });

    edgeMeshes.forEach(em => {
      if (em === edgeMesh) {
        em.line.material.opacity = 0.9;
        em.line.material.color.set(0xffffff);
      } else {
        em.line.material.opacity = 0.03;
        em.line.material.color.set(0x111522);
      }
    });

    memLinkMeshes.forEach(em => {
      if (em === edgeMesh) {
        em.line.material.opacity = 0.8;
        em.line.material.color.set(0xffffff);
      } else {
        em.line.material.opacity = 0.02;
        em.line.material.color.set(0x111522);
      }
    });
  }

  function updateInfoPanel(nodeData, connectedNodeIds) {
    const panel = document.getElementById('graph-info-panel');
    if (!panel) return;

    const neighbors = [];
    connectedNodeIds.forEach(nid => {
      const n = graphNodes.find(gn => gn.id === nid);
      if (n) neighbors.push(n);
    });

    neighbors.sort((a, b) => {
      if (a._isHub && !b._isHub) return -1;
      if (!a._isHub && b._isHub) return 1;
      return (b.importance || 5) - (a.importance || 5);
    });

    const colorMap = { entity: 'var(--purple)', project: 'var(--green)', memory: 'var(--accent)' };
    const typeIcon = { entity: '\u25B2', project: '\u25CF', memory: '\u25A0' };

    let html = `<div class="info-header">
      <span class="info-type" style="color:${colorMap[nodeData.type] || 'var(--accent)'}">${typeIcon[nodeData.type] || ''} ${(nodeData.type || '').toUpperCase()}</span>
      <span class="info-title">${esc(truncate(nodeData.label, 50))}</span>
    </div>`;

    if (nodeData.importance) {
      html += `<div class="info-importance"><span class="imp-bar" style="width:${nodeData.importance * 10}%"></span><span class="imp-text">IMP ${nodeData.importance}/10</span></div>`;
    }

    html += `<div class="info-connections-header">${neighbors.length} connection${neighbors.length !== 1 ? 's' : ''}</div>`;
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

  // ─── Raycasting ───
  function doRaycast() {
    if (!raycaster || mouse.x < -5) return;

    raycaster.setFromCamera(mouse, camera);
    const meshes = nodeMeshes.map(nm => nm.mesh);
    const intersects = raycaster.intersectObjects(meshes, false);

    const tooltip = document.getElementById('graph-tooltip');
    const canvas = document.getElementById('graph-canvas');

    // Check node hover
    if (intersects.length > 0) {
      const hit = intersects[0].object;
      const nm = nodeMeshes.find(nm => nm.mesh === hit);
      if (nm) {
        if (hoveredNode !== nm) {
          clearHighlights();
          hoveredNode = nm;
          hoveredEdge = null;
          canvas.style.cursor = nm.data.type === 'memory' ? 'pointer' : 'default';

          highlightNode(nm);

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

    // Check edge hover (proximity to Bezier curves)
    const ray = raycaster.ray;
    const threshold = graphMode === '2d' ? 2.5 : 1.8;
    let closestEdgeMesh = null;
    let closestDist = threshold;
    let closestGroup = null;

    function checkEdgeGroup(edges, group) {
      for (let i = 0; i < edges.length; i++) {
        const em = edges[i];
        const e = em.data;
        if (e._si === undefined || e._ti === undefined) continue;
        const s = graphNodes[e._si];
        const t = graphNodes[e._ti];

        const sv = new THREE.Vector3(s.x, s.y, s.z);
        const tv = new THREE.Vector3(t.x, t.y, t.z);
        const mid = new THREE.Vector3().addVectors(sv, tv).multiplyScalar(0.5);

        const toMid = new THREE.Vector3().subVectors(mid, ray.origin);
        const proj = toMid.dot(ray.direction);
        if (proj < 0) continue;

        const closestOnRay = new THREE.Vector3().copy(ray.direction).multiplyScalar(proj).add(ray.origin);
        const dist = pointToSegmentDist(
          closestOnRay.x, closestOnRay.y, closestOnRay.z,
          s.x, s.y, s.z, t.x, t.y, t.z
        );

        if (dist < closestDist) {
          closestDist = dist;
          closestEdgeMesh = em;
          closestGroup = group;
        }
      }
    }

    checkEdgeGroup(edgeMeshes, 'structural');
    checkEdgeGroup(memLinkMeshes, 'memlink');

    if (closestEdgeMesh) {
      if (!hoveredEdge || hoveredEdge.edgeMesh !== closestEdgeMesh) {
        clearHighlights();
        hoveredNode = null;
        canvas.style.cursor = 'crosshair';

        highlightEdge(closestEdgeMesh.data, closestEdgeMesh, closestGroup);

        const e = closestEdgeMesh.data;
        const s = graphNodes[e._si];
        const t = graphNodes[e._ti];
        let html = `<div class="tt-title">${edgeTypeName(e.type)}</div>`;
        html += `<div class="tt-type">${esc(s.label)} \u2194 ${esc(t.label)}</div>`;
        if (e.label) html += `<div class="tt-content">${esc(e.label)}</div>`;
        tooltip.innerHTML = html;
        tooltip.classList.add('visible');
      }
      return;
    }

    // Nothing hovered
    if (hoveredNode || hoveredEdge) {
      clearHighlights();
      hoveredNode = null;
      hoveredEdge = null;
      canvas.style.cursor = 'grab';
      tooltip.classList.remove('visible');
    }
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

  // ─── Main Animation Loop ───
  function animate() {
    if (!graphInitialized) return;
    requestAnimationFrame(animate);

    const dt = clock.getDelta();
    const elapsed = clock.getElapsedTime();

    // Update galaxy shader time
    scene.traverse(function(obj) {
      if (obj.userData && obj.userData.isBgSphere && obj.material && obj.material.uniforms) {
        obj.material.uniforms.uTime.value = elapsed;
      }
    });

    // Layout animation
    updateLayout(dt);

    // Orbital motion (after layout settles)
    updateOrbitalMotion(dt);

    // Update positions
    updatePositions();

    // Animations (glow pulse, corona rotation, star twinkle)
    updateAnimations(elapsed);

    // Particles
    updateParticles(elapsed);

    // Hover pulse effect
    if (hoveredNode && highlightActive) {
      const pulse = 0.4 + Math.sin(elapsed * 4) * 0.15;
      hoveredNode.glowMat.opacity = pulse;
      const s = 1.0 + Math.sin(elapsed * 3) * 0.04;
      hoveredNode.mesh.scale.set(s, s, s);
    }

    // Raycasting (every 2 frames)
    frameCount++;
    if (frameCount % 2 === 0) {
      doRaycast();
    }

    controls.update();
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
