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
  let simulation = null;
  let svgZoom = null;

  // ─── Init ───
  document.addEventListener('DOMContentLoaded', async () => {
    setupNav();
    setupSearch();
    setupModal();
    await loadAll();
  });

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
    if (!m.is_latest) badges.push('<span class="badge badge-forgotten">v${m.version}</span>');

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

      // Linked memories
      if (m.linked_memory_ids && m.linked_memory_ids.length) {
        const chips = m.linked_memory_ids.map(lid => {
          const linked = memoriesData.find(x => x.id === lid);
          const label = linked ? linked.title : `Memory #${lid}`;
          return `<span class="detail-link-chip" onclick="window.__openMemory(${lid})">${esc(label)}</span>`;
        }).join('');
        html += `<div class="detail-section"><h3>Linked Memories</h3><div class="detail-links">${chips}</div></div>`;
      }

      // Entities
      if (m.entity_ids && m.entity_ids.length) {
        const chips = m.entity_ids.map(eid => {
          const ent = entitiesData.find(x => x.id === eid);
          const label = ent ? ent.name : `Entity #${eid}`;
          return `<span class="detail-link-chip entity-chip">${esc(label)}</span>`;
        }).join('');
        html += `<div class="detail-section"><h3>Entities</h3><div class="detail-links">${chips}</div></div>`;
      }

      // Projects
      if (m.project_ids && m.project_ids.length) {
        const chips = m.project_ids.map(pid => {
          const proj = projectsData.find(x => x.id === pid);
          const label = proj ? proj.name : `Project #${pid}`;
          return `<span class="detail-link-chip project-chip">${esc(label)}</span>`;
        }).join('');
        html += `<div class="detail-section"><h3>Projects</h3><div class="detail-links">${chips}</div></div>`;
      }

      // Provenance
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

  // ─── Knowledge Graph ───
  function initGraph() {
    if (!graphData || !graphData.nodes.length) {
      document.getElementById('graph-container').innerHTML = '<div class="empty-state" style="padding-top:100px"><h3>No data to visualize</h3></div>';
      return;
    }

    const container = document.getElementById('graph-container');
    const svg = d3.select('#graph-svg');
    const width = container.clientWidth || 800;
    const height = container.clientHeight || 600;

    svg.attr('viewBox', [0, 0, width, height]);

    // Build filtered data
    const nodes = graphData.nodes.map(n => ({ ...n }));
    const edges = graphData.edges.filter(e => {
      return nodes.some(n => n.id === e.source) && nodes.some(n => n.id === e.target);
    }).map(e => ({ ...e }));

    // Color scale
    function nodeColor(d) {
      if (d.type === 'entity') return 'var(--node-entity)';
      if (d.type === 'project') return 'var(--node-project)';
      if (d.is_obsolete) return 'var(--red)';
      if (d.is_forgotten) return 'var(--text-dim)';
      return 'var(--node-memory)';
    }

    function nodeRadius(d) {
      if (d.type === 'entity') return 8;
      if (d.type === 'project') return 10;
      return 4 + (d.importance || 5) * 0.6;
    }

    function linkClass(d) {
      const t = d.type || '';
      if (t === 'memory_link') return 'link memory-link';
      if (t === 'entity_assoc') return 'link entity-assoc';
      if (t === 'project_assoc') return 'link project-assoc';
      if (t === 'relationship') return 'link relationship';
      if (t === 'entity_project') return 'link entity-project';
      return 'link';
    }

    // Zoom
    const g = svg.append('g');
    svgZoom = d3.zoom()
      .scaleExtent([0.1, 8])
      .on('zoom', (event) => g.attr('transform', event.transform));
    svg.call(svgZoom);

    // Links
    const link = g.append('g')
      .selectAll('line')
      .data(edges)
      .join('line')
      .attr('class', d => linkClass(d))
      .attr('stroke-width', d => d.type === 'relationship' ? 2 : 1);

    // Edge labels for relationships
    const edgeLabels = g.append('g')
      .selectAll('text')
      .data(edges.filter(e => e.type === 'relationship' && e.label))
      .join('text')
      .text(d => d.label)
      .attr('font-size', 8)
      .attr('fill', 'var(--orange)')
      .attr('text-anchor', 'middle')
      .attr('dy', -4)
      .style('pointer-events', 'none');

    // Nodes
    const node = g.append('g')
      .selectAll('g')
      .data(nodes)
      .join('g')
      .attr('class', 'node')
      .call(d3.drag()
        .on('start', dragstarted)
        .on('drag', dragged)
        .on('end', dragended));

    node.append('circle')
      .attr('r', d => nodeRadius(d))
      .attr('fill', d => nodeColor(d))
      .attr('stroke', d => d.type === 'project' ? 'var(--green)' : d.type === 'entity' ? 'var(--purple)' : 'var(--accent-dim)')
      .attr('opacity', d => (d.is_obsolete || d.is_forgotten) ? 0.4 : 0.9);

    // Labels for entities and projects (always visible)
    node.filter(d => d.type === 'entity' || d.type === 'project')
      .append('text')
      .text(d => truncate(d.label, 20))
      .attr('dy', d => nodeRadius(d) + 12);

    // Tooltip
    const tooltip = document.getElementById('graph-tooltip');
    node.on('mouseover', (event, d) => {
      let html = `<div class="tt-title">${esc(d.label)}</div>`;
      html += `<div class="tt-type">${d.type}${d.entity_type ? ' / ' + d.entity_type : ''}${d.memory_type ? ' / ' + d.memory_type : ''}</div>`;
      if (d.importance) html += `<div class="tt-content">Importance: ${d.importance}</div>`;
      if (d.tags) {
        const tags = parseTags(d.tags);
        if (tags.length) html += `<div class="tt-tags">${tags.map(t => `<span class="tag">${esc(t)}</span>`).join('')}</div>`;
      }
      tooltip.innerHTML = html;
      tooltip.classList.add('visible');
      const rect = container.getBoundingClientRect();
      tooltip.style.left = (event.clientX - rect.left + 14) + 'px';
      tooltip.style.top = (event.clientY - rect.top - 10) + 'px';
    })
    .on('mousemove', (event) => {
      const rect = container.getBoundingClientRect();
      tooltip.style.left = (event.clientX - rect.left + 14) + 'px';
      tooltip.style.top = (event.clientY - rect.top - 10) + 'px';
    })
    .on('mouseout', () => {
      tooltip.classList.remove('visible');
    })
    .on('click', (event, d) => {
      if (d.type === 'memory') openMemory(d.raw_id);
    });

    // Simulation
    simulation = d3.forceSimulation(nodes)
      .force('link', d3.forceLink(edges).id(d => d.id).distance(80).strength(0.3))
      .force('charge', d3.forceManyBody().strength(-120).distanceMax(400))
      .force('center', d3.forceCenter(width / 2, height / 2))
      .force('collision', d3.forceCollide().radius(d => nodeRadius(d) + 4))
      .force('x', d3.forceX(width / 2).strength(0.05))
      .force('y', d3.forceY(height / 2).strength(0.05))
      .on('tick', () => {
        link
          .attr('x1', d => d.source.x)
          .attr('y1', d => d.source.y)
          .attr('x2', d => d.target.x)
          .attr('y2', d => d.target.y);

        edgeLabels
          .attr('x', d => (d.source.x + d.target.x) / 2)
          .attr('y', d => (d.source.y + d.target.y) / 2);

        node.attr('transform', d => `translate(${d.x},${d.y})`);
      });

    // Graph controls
    document.getElementById('graph-reset').addEventListener('click', () => {
      svg.transition().duration(500).call(svgZoom.transform, d3.zoomIdentity);
    });

    // Filter controls
    const showObsolete = document.getElementById('show-obsolete');
    const showForgotten = document.getElementById('show-forgotten');
    const minImp = document.getElementById('min-importance');
    const minImpVal = document.getElementById('min-imp-val');

    function applyFilters() {
      const minI = parseInt(minImp.value);
      minImpVal.textContent = minI;

      node.attr('display', d => {
        if (d.type === 'entity' || d.type === 'project') return null;
        if (d.is_obsolete && !showObsolete.checked) return 'none';
        if (d.is_forgotten && !showForgotten.checked) return 'none';
        if ((d.importance || 5) < minI) return 'none';
        return null;
      });

      const visibleIds = new Set();
      node.each(function(d) {
        if (d3.select(this).attr('display') !== 'none') visibleIds.add(d.id);
      });

      link.attr('display', d => {
        const sid = typeof d.source === 'object' ? d.source.id : d.source;
        const tid = typeof d.target === 'object' ? d.target.id : d.target;
        return (visibleIds.has(sid) && visibleIds.has(tid)) ? null : 'none';
      });

      edgeLabels.attr('display', d => {
        const sid = typeof d.source === 'object' ? d.source.id : d.source;
        const tid = typeof d.target === 'object' ? d.target.id : d.target;
        return (visibleIds.has(sid) && visibleIds.has(tid)) ? null : 'none';
      });
    }

    showObsolete.addEventListener('change', applyFilters);
    showForgotten.addEventListener('change', applyFilters);
    minImp.addEventListener('input', applyFilters);

    function dragstarted(event, d) {
      if (!event.active) simulation.alphaTarget(0.3).restart();
      d.fx = d.x;
      d.fy = d.y;
    }

    function dragged(event, d) {
      d.fx = event.x;
      d.fy = event.y;
    }

    function dragended(event, d) {
      if (!event.active) simulation.alphaTarget(0);
      d.fx = null;
      d.fy = null;
    }
  }

  function resizeGraph() {
    const container = document.getElementById('graph-container');
    const svg = d3.select('#graph-svg');
    const width = container.clientWidth;
    const height = container.clientHeight;
    svg.attr('viewBox', [0, 0, width, height]);
    if (simulation) {
      simulation.force('center', d3.forceCenter(width / 2, height / 2));
      simulation.force('x', d3.forceX(width / 2).strength(0.05));
      simulation.force('y', d3.forceY(height / 2).strength(0.05));
      simulation.alpha(0.3).restart();
    }
  }

  window.addEventListener('resize', () => {
    if (document.getElementById('view-graph').classList.contains('active')) resizeGraph();
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
