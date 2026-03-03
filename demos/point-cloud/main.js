import init, { CanvasEngine } from '../../pkg/rendero.js';
await init();

let app = new CanvasEngine("PointCloud", 1);
window._app = app; // debug

// ─── DOM refs ───
const canvas = document.getElementById('canvas');
const ctx = canvas.getContext('2d');
const layersList = document.getElementById('layers-list');
const layersInfo = document.getElementById('layers-info');
const propsContent = document.getElementById('properties-content');
const ctxMenu = document.getElementById('ctx-menu');
const loadingEl = document.getElementById('loading');
const loadBar = document.getElementById('load-bar');
const loadText = document.getElementById('load-text');

// ─── Canvas sizing ───
const dpr = window.devicePixelRatio || 1;
let cssW, cssH;
function resize() {
    const wrap = document.getElementById('canvas-wrap');
    cssW = wrap.clientWidth; cssH = wrap.clientHeight;
    canvas.width = cssW * dpr; canvas.height = cssH * dpr;
    canvas.style.width = cssW + 'px'; canvas.style.height = cssH + 'px';
    app.set_viewport(cssW, cssH);
}
resize();
window.addEventListener('resize', () => { resize(); render(); });

// ─── Tool state ───
let currentTool = 'select';
let spaceHeld = false;

document.querySelectorAll('[data-tool]').forEach(btn => {
    btn.addEventListener('click', () => {
        currentTool = btn.dataset.tool;
        document.querySelectorAll('[data-tool]').forEach(b => b.classList.toggle('active', b.dataset.tool === currentTool));
        canvas.style.cursor = currentTool === 'select' ? 'default' : 'crosshair';
    });
});

// ─── FPS tracking ───
let frameTimes = [];
let lastTime = 0;

function updateStats(now) {
    if (lastTime > 0) {
        frameTimes.push(now - lastTime);
        if (frameTimes.length > 60) frameTimes.shift();
    }
    lastTime = now;
    const avg = frameTimes.length ? frameTimes.reduce((a, b) => a + b) / frameTimes.length : 0;
    const fps = avg > 0 ? Math.round(1000 / avg) : 0;
    const sel = app.get_selected();
    const selCount = sel.length / 2;
    const selText = selCount > 0 ? ` | <b>${selCount}</b> sel` : '';
    document.getElementById('info').innerHTML = `<b>${app.node_count()}</b> nodes | <b>${fps}</b> fps | <b>${app.drawn_count()}</b> drawn${selText}`;
}

// ─── Render ───
function render() {
    app.render_canvas2d(ctx, cssW, cssH, dpr);
}

function loop(now) {
    render();
    updateStats(now);
    requestAnimationFrame(loop);
}

// ═══════════════════════════════════════════════════════════
//  PATTERN GENERATORS
// ═══════════════════════════════════════════════════════════

// Seeded PRNG
let seed = 42;
function rand() { seed = (seed * 1664525 + 1013904223) & 0xffffffff; return (seed >>> 0) / 4294967296; }
function resetRand() { seed = 42; }

function hslToRgb(h, s, l) {
    const a = s * Math.min(l, 1 - l);
    const f = n => { const k = (n + h * 12) % 12; return l - a * Math.max(-1, Math.min(k - 3, 9 - k, 1)); };
    return [f(0), f(8), f(4)];
}

function generateClusters(count) {
    resetRand();
    const numClusters = 5 + Math.floor(rand() * 6);
    const centers = [];
    for (let i = 0; i < numClusters; i++) {
        centers.push({
            x: 200 + rand() * 3600, y: 200 + rand() * 2600,
            hue: rand(), spread: 80 + rand() * 250,
        });
    }

    const buf = new Float32Array(count * 8);
    for (let i = 0; i < count; i++) {
        const c = centers[Math.floor(rand() * numClusters)];
        // Box-Muller
        const u1 = Math.max(1e-10, rand()), u2 = rand();
        const z0 = Math.sqrt(-2 * Math.log(u1)) * Math.cos(2 * Math.PI * u2);
        const z1 = Math.sqrt(-2 * Math.log(u1)) * Math.sin(2 * Math.PI * u2);
        const x = c.x + z0 * c.spread;
        const y = c.y + z1 * c.spread;
        const size = 4 + rand() * 6;
        const [r, g, b] = hslToRgb(c.hue, 0.7, 0.5 + rand() * 0.2);
        const base = i * 8;
        buf[base] = x; buf[base+1] = y; buf[base+2] = size; buf[base+3] = size;
        buf[base+4] = r; buf[base+5] = g; buf[base+6] = b; buf[base+7] = 0.8;
    }
    return buf;
}

function generateGrid(count) {
    resetRand();
    const cols = Math.ceil(Math.sqrt(count));
    const rows = Math.ceil(count / cols);
    const spacing = 18;
    const buf = new Float32Array(count * 8);
    for (let i = 0; i < count; i++) {
        const col = i % cols, row = Math.floor(i / cols);
        const x = col * spacing + rand() * 2;
        const y = row * spacing + rand() * 2;
        const [r, g, b] = hslToRgb(col / cols * 0.8, 0.7, 0.35 + (row / rows) * 0.3);
        const base = i * 8;
        buf[base] = x; buf[base+1] = y; buf[base+2] = 10; buf[base+3] = 10;
        buf[base+4] = r; buf[base+5] = g; buf[base+6] = b; buf[base+7] = 1.0;
    }
    return buf;
}

function generateSpiral(count) {
    resetRand();
    const cx = 2000, cy = 1500;
    const buf = new Float32Array(count * 8);
    for (let i = 0; i < count; i++) {
        const t = (i / count) * Math.PI * 12;
        const r = t * 4 + rand() * 10;
        const x = cx + r * Math.cos(t);
        const y = cy + r * Math.sin(t);
        const size = 3 + (1 - i / count) * 6;
        const [cr, cg, cb] = hslToRgb((i / count) * 0.85, 0.8, 0.5);
        const base = i * 8;
        buf[base] = x; buf[base+1] = y; buf[base+2] = size; buf[base+3] = size;
        buf[base+4] = cr; buf[base+5] = cg; buf[base+6] = cb; buf[base+7] = 0.9;
    }
    return buf;
}

function generateGalaxy(count) {
    resetRand();
    const cx = 2000, cy = 1500;
    const buf = new Float32Array(count * 8);
    for (let i = 0; i < count; i++) {
        const arm = i % 3;
        const t = (i / count) * Math.PI * 5;
        const r = (i / count) * 1400;
        const angle = t + arm * (Math.PI * 2 / 3);
        const scatter = (rand() - 0.5) * r * 0.25;
        const x = cx + (r + scatter) * Math.cos(angle) + (rand() - 0.5) * 20;
        const y = cy + (r + scatter) * Math.sin(angle) + (rand() - 0.5) * 20;
        const brightness = 0.7 - 0.4 * (i / count);
        const size = 3 + rand() * 5;
        const base = i * 8;
        buf[base] = x; buf[base+1] = y; buf[base+2] = size; buf[base+3] = size;
        buf[base+4] = brightness; buf[base+5] = brightness * 0.85; buf[base+6] = brightness * 0.6; buf[base+7] = 0.85;
    }
    return buf;
}

function generateRandom(count) {
    resetRand();
    const buf = new Float32Array(count * 8);
    for (let i = 0; i < count; i++) {
        const x = rand() * 5000, y = rand() * 4000;
        const size = 4 + rand() * 14;
        const [r, g, b] = hslToRgb(rand(), 0.5 + rand() * 0.3, 0.4 + rand() * 0.2);
        const base = i * 8;
        buf[base] = x; buf[base+1] = y; buf[base+2] = size; buf[base+3] = size;
        buf[base+4] = r; buf[base+5] = g; buf[base+6] = b; buf[base+7] = 0.8;
    }
    return buf;
}

const generators = { clusters: generateClusters, grid: generateGrid, spiral: generateSpiral, galaxy: generateGalaxy, random: generateRandom };

// ═══════════════════════════════════════════════════════════
//  GENERATION FLOW
// ═══════════════════════════════════════════════════════════

function fitViewToData(buf, count) {
    const stride = 8;
    let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
    for (let i = 0; i < count; i++) {
        const base = i * stride;
        const x = buf[base], y = buf[base + 1], w = buf[base + 2], h = buf[base + 3];
        if (x < minX) minX = x;
        if (y < minY) minY = y;
        if (x + w > maxX) maxX = x + w;
        if (y + h > maxY) maxY = y + h;
    }
    const contentW = maxX - minX;
    const contentH = maxY - minY;
    if (contentW <= 0 || contentH <= 0) return;
    const padding = 50;
    const vw = cssW - padding * 2;
    const vh = cssH - padding * 2;
    const zoom = Math.min(vw / contentW, vh / contentH);
    app.set_camera(minX - padding / zoom, minY - padding / zoom, zoom);
}

// Recursive spatial hierarchy — ensures LOD collapse at every zoom level.
// At any zoom, at most ~LEAF_SIZE individual nodes render per visible frame.
const LEAF_SIZE = 200;

function addToSceneWithFrames(buf, count) {
    const indices = new Uint32Array(count);
    for (let i = 0; i < count; i++) indices[i] = i;
    // null parentFrame = root
    addRecursive(buf, indices, 0, 0, 0, null);
}

function addRecursive(buf, indices, originX, originY, depth, parentFrame) {
    const stride = 8;
    const n = indices.length;

    // Set insert target
    if (parentFrame) app.set_insert_parent(parentFrame[0], parentFrame[1]);
    else app.clear_insert_parent();

    // Leaf: add ellipses directly under current parent
    if (n <= LEAF_SIZE || depth >= 6) {
        const sub = new Float32Array(n * stride);
        for (let j = 0; j < n; j++) {
            const src = indices[j] * stride;
            const dst = j * stride;
            sub[dst]     = buf[src]     - originX;
            sub[dst + 1] = buf[src + 1] - originY;
            sub[dst + 2] = buf[src + 2];
            sub[dst + 3] = buf[src + 3];
            sub[dst + 4] = buf[src + 4];
            sub[dst + 5] = buf[src + 5];
            sub[dst + 6] = buf[src + 6];
            sub[dst + 7] = buf[src + 7];
        }
        app.add_ellipses_batch(sub);
        return;
    }

    // Compute bounds
    let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
    for (let i = 0; i < n; i++) {
        const base = indices[i] * stride;
        minX = Math.min(minX, buf[base]);
        minY = Math.min(minY, buf[base + 1]);
        maxX = Math.max(maxX, buf[base] + buf[base + 2]);
        maxY = Math.max(maxY, buf[base + 1] + buf[base + 3]);
    }

    const spanX = maxX - minX || 1;
    const spanY = maxY - minY || 1;

    // Grid: target ~8-25 children per level
    const childTarget = Math.max(4, Math.min(8, Math.ceil(Math.sqrt(n / LEAF_SIZE))));
    const aspect = spanX / spanY;
    const cols = Math.max(2, Math.round(Math.sqrt(childTarget * aspect)));
    const rows = Math.max(2, Math.round(childTarget / cols));
    const cellW = spanX / cols;
    const cellH = spanY / rows;

    // Bucket into grid
    const buckets = new Array(cols * rows);
    for (let i = 0; i < buckets.length; i++) buckets[i] = [];
    for (let i = 0; i < n; i++) {
        const base = indices[i] * stride;
        const col = Math.min(cols - 1, Math.floor((buf[base] - minX) / cellW));
        const row = Math.min(rows - 1, Math.floor((buf[base + 1] - minY) / cellH));
        buckets[row * cols + col].push(indices[i]);
    }

    // Create frame per cell, recurse into it
    for (let r = 0; r < rows; r++) {
        for (let c = 0; c < cols; c++) {
            const bucket = buckets[r * cols + c];
            if (bucket.length === 0) continue;

            // Tight bounds + average color
            let bMinX = Infinity, bMinY = Infinity, bMaxX = -Infinity, bMaxY = -Infinity;
            let bR = 0, bG = 0, bB = 0;
            for (const idx of bucket) {
                const base = idx * stride;
                bMinX = Math.min(bMinX, buf[base]);
                bMinY = Math.min(bMinY, buf[base + 1]);
                bMaxX = Math.max(bMaxX, buf[base] + buf[base + 2]);
                bMaxY = Math.max(bMaxY, buf[base + 1] + buf[base + 3]);
                bR += buf[base + 4]; bG += buf[base + 5]; bB += buf[base + 6];
            }
            bR /= bucket.length; bG /= bucket.length; bB /= bucket.length;
            const density = Math.min(1, bucket.length / LEAF_SIZE);
            const alpha = 0.2 + density * 0.5;

            // Re-set parent before adding frame (recursion may have changed it)
            if (parentFrame) app.set_insert_parent(parentFrame[0], parentFrame[1]);
            else app.clear_insert_parent();

            const frameId = app.add_frame(
                `L${depth}_${r}_${c}`,
                bMinX - originX, bMinY - originY,
                bMaxX - bMinX, bMaxY - bMinY,
                bR, bG, bB, alpha,
            );

            // Recurse: children go into this frame
            addRecursive(buf, new Uint32Array(bucket), bMinX, bMinY, depth + 1, frameId);
        }
    }
}

async function generate(pattern, count) {
    loadingEl.style.display = 'flex';
    loadBar.style.width = '0%';
    loadText.textContent = `Generating ${pattern} pattern...`;
    await new Promise(r => setTimeout(r, 0));

    // Fresh engine
    app = new CanvasEngine("PointCloud", 1);
    app.set_viewport(cssW, cssH);

    loadBar.style.width = '30%';
    loadText.textContent = `Computing ${count.toLocaleString()} points...`;
    await new Promise(r => setTimeout(r, 0));

    const buf = generators[pattern](count);

    loadBar.style.width = '60%';
    loadText.textContent = `Adding to scene graph...`;
    await new Promise(r => setTimeout(r, 0));

    // Wrap ellipses in spatial frames for hierarchical LOD —
    // when zoomed out, each frame collapses to a single colored rect.
    addToSceneWithFrames(buf, count);

    loadBar.style.width = '90%';
    loadText.textContent = `Fitting view...`;
    await new Promise(r => setTimeout(r, 0));

    fitViewToData(buf, count);
    window._app = app;

    loadBar.style.width = '100%';
    loadText.textContent = `${count.toLocaleString()} interactive nodes ready`;
    await new Promise(r => setTimeout(r, 200));

    loadingEl.style.display = 'none';
    updatePanels();
}

// ─── Slider + Generate ───
const countSlider = document.getElementById('count-slider');
const countLabel = document.getElementById('count-label');

function sliderToCount() {
    return Math.round(Math.pow(10, parseFloat(countSlider.value)));
}

function formatCount(n) {
    if (n >= 1_000_000) return (n / 1_000_000).toFixed(1) + 'M';
    if (n >= 1_000) return (n / 1_000).toFixed(0) + 'K';
    return n.toString();
}

countSlider.addEventListener('input', () => {
    countLabel.textContent = formatCount(sliderToCount());
});

document.getElementById('btn-generate').addEventListener('click', () => {
    const pattern = document.getElementById('pattern-select').value;
    generate(pattern, sliderToCount());
});

// ═══════════════════════════════════════════════════════════
//  INTERACTIVE FEATURES
// ═══════════════════════════════════════════════════════════

// ─── Mouse interaction ───
canvas.addEventListener('mousedown', (e) => {
    hideContextMenu();
    if (e.button === 2) { e.preventDefault(); return; }

    // Pan (middle click or space held)
    if (e.button === 1 || spaceHeld) {
        app.pan_start(e.offsetX, e.offsetY);
        canvas.style.cursor = 'grabbing';
        e.preventDefault();
        return;
    }

    // Select tool
    if (currentTool === 'select') {
        const hit = app.mouse_down(e.offsetX, e.offsetY, e.shiftKey);
        canvas.style.cursor = hit ? 'move' : 'default';
        render();
        updatePanels();
    }
});

canvas.addEventListener('mousemove', (e) => {
    if (e.buttons > 0 && (e.button === 1 || spaceHeld)) {
        app.pan_move(e.offsetX, e.offsetY);
    }
    if (e.buttons === 1 && currentTool === 'select' && !spaceHeld) {
        app.mouse_move(e.offsetX, e.offsetY);
    }
    if (app.needs_render()) render();
});

canvas.addEventListener('mouseup', (e) => {
    app.pan_end();
    app.mouse_up();
    canvas.style.cursor = spaceHeld ? 'grab' : 'default';
    updatePanels();
});

canvas.addEventListener('wheel', (e) => {
    e.preventDefault();
    if (e.ctrlKey || e.metaKey) {
        app.zoom(e.deltaY < 0 ? 1 : -1, e.offsetX, e.offsetY);
    } else {
        app.pan_start(e.offsetX, e.offsetY);
        app.pan_move(e.offsetX - e.deltaX, e.offsetY - e.deltaY);
        app.pan_end();
    }
    render();
}, { passive: false });

// ─── Context menu ───
canvas.addEventListener('contextmenu', (e) => { e.preventDefault(); showContextMenu(e.clientX, e.clientY); });
document.addEventListener('click', (e) => { if (!ctxMenu.contains(e.target)) hideContextMenu(); });

function showContextMenu(x, y) {
    const sel = app.get_selected();
    const hasSel = sel.length > 0;
    const items = [
        { label: 'Copy', action: 'copy', key: '⌘C', enabled: hasSel },
        { label: 'Paste', action: 'paste', key: '⌘V', enabled: true },
        { label: 'Duplicate', action: 'duplicate', key: '⌘D', enabled: hasSel },
        { label: 'Delete', action: 'delete', key: '⌫', enabled: hasSel },
        { sep: true },
        { label: 'Group', action: 'group', key: '⌘G', enabled: sel.length > 2 },
        { label: 'Ungroup', action: 'ungroup', key: '⇧⌘G', enabled: hasSel },
        { sep: true },
        { label: 'Bring to Front', action: 'front', key: '⌘]', enabled: hasSel },
        { label: 'Send to Back', action: 'back', key: '⌘[', enabled: hasSel },
        { sep: true },
        { label: 'Select All', action: 'selectall', key: '⌘A', enabled: true },
        { label: 'Zoom to Fit', action: 'zoomfit', key: '⌘0', enabled: true },
    ];
    let html = '';
    for (const item of items) {
        if (item.sep) { html += '<div class="ctx-sep"></div>'; continue; }
        html += `<div class="ctx-item ${item.enabled ? '' : 'disabled'}" data-action="${item.action}">
            <span>${item.label}</span><span class="ctx-shortcut">${item.key}</span></div>`;
    }
    ctxMenu.innerHTML = html;
    ctxMenu.style.display = 'block';
    ctxMenu.style.left = `${x}px`;
    ctxMenu.style.top = `${y}px`;
    ctxMenu.querySelectorAll('.ctx-item:not(.disabled)').forEach(el => {
        el.addEventListener('click', () => { doAction(el.dataset.action); hideContextMenu(); });
    });
}

function hideContextMenu() { ctxMenu.style.display = 'none'; }

function doAction(action) {
    switch (action) {
        case 'copy': app.copy_selected(); break;
        case 'paste': app.paste(); break;
        case 'duplicate': app.duplicate_selected(); break;
        case 'delete': app.delete_selected(); break;
        case 'group': app.group_selected(); break;
        case 'ungroup': app.ungroup_selected(); break;
        case 'front': app.bring_to_front(); break;
        case 'back': app.send_to_back(); break;
        case 'selectall': app.select_all(); break;
        case 'zoomfit': app.zoom_to_fit(); break;
    }
    render();
    updatePanels();
}

// ─── Keyboard shortcuts ───
window.addEventListener('keydown', (e) => {
    const cmd = e.metaKey || e.ctrlKey;
    if (e.key === 'Delete' || e.key === 'Backspace') { doAction('delete'); }
    if (cmd && e.key === 'z' && !e.shiftKey) { e.preventDefault(); app.undo(); render(); updatePanels(); }
    if (cmd && (e.key === 'Z' || (e.key === 'z' && e.shiftKey))) { e.preventDefault(); app.redo(); render(); updatePanels(); }
    if (cmd && e.key === 'c') { app.copy_selected(); }
    if (cmd && e.key === 'v') { e.preventDefault(); doAction('paste'); }
    if (cmd && e.key === 'd') { e.preventDefault(); doAction('duplicate'); }
    if (cmd && e.key === 'g' && !e.shiftKey) { e.preventDefault(); doAction('group'); }
    if (cmd && e.key === 'g' && e.shiftKey) { e.preventDefault(); doAction('ungroup'); }
    if (cmd && e.key === '0') { e.preventDefault(); doAction('zoomfit'); }
    if (cmd && e.key === 'a') { e.preventDefault(); doAction('selectall'); }
    if (cmd && e.key === ']') { e.preventDefault(); doAction('front'); }
    if (cmd && e.key === '[') { e.preventDefault(); doAction('back'); }
    if (e.key === ' ') { spaceHeld = true; canvas.style.cursor = 'grab'; e.preventDefault(); }
    if (e.key === 'v' && !cmd) { currentTool = 'select'; document.querySelectorAll('[data-tool]').forEach(b => b.classList.toggle('active', b.dataset.tool === 'select')); }
});

window.addEventListener('keyup', (e) => {
    if (e.key === ' ') { spaceHeld = false; canvas.style.cursor = 'default'; }
});

// ─── Toolbar buttons ───
document.getElementById('btn-delete').addEventListener('click', () => doAction('delete'));
document.getElementById('btn-group').addEventListener('click', () => doAction('group'));
document.getElementById('btn-ungroup').addEventListener('click', () => doAction('ungroup'));
document.getElementById('btn-undo').addEventListener('click', () => { app.undo(); render(); updatePanels(); });
document.getElementById('btn-redo').addEventListener('click', () => { app.redo(); render(); updatePanels(); });

// ─── Layers panel ───
function updateLayersPanel() {
    try {
        const json = app.get_layers();
        const layers = JSON.parse(json);
        const sel = app.get_selected();
        const selSet = new Set();
        for (let i = 0; i < sel.length; i += 2) selSet.add(`${sel[i]}_${sel[i + 1]}`);

        const total = layers.length;
        const selCount = sel.length / 2;
        layersInfo.innerHTML = `<b>${total}</b> nodes${selCount > 0 ? ` | <b>${selCount}</b> selected` : ''}`;

        // Only show up to 500 layers for performance
        const maxShow = 500;
        let html = '';
        const toShow = layers.slice(0, maxShow);
        for (const layer of toShow) {
            const key = `${layer.id[0]}_${layer.id[1]}`;
            const selected = selSet.has(key) ? ' selected' : '';
            const icon = getNodeIcon(layer.kind);
            html += `<div class="layer-item${selected}" data-counter="${layer.id[0]}" data-client="${layer.id[1]}">
                <span class="layer-icon">${icon}</span>${layer.name}</div>`;
        }
        if (total > maxShow) {
            html += `<div class="layer-item" style="opacity:0.4; pointer-events:none;">...and ${(total - maxShow).toLocaleString()} more</div>`;
        }
        layersList.innerHTML = html;
        layersList.querySelectorAll('.layer-item[data-counter]').forEach(el => {
            el.addEventListener('click', () => {
                app.select_node(+el.dataset.counter, +el.dataset.client);
                render();
                updatePanels();
            });
        });
    } catch (_) {}
}

function getNodeIcon(kind) {
    const icons = { Frame: '◻', Rectangle: '■', Ellipse: '●', Text: 'T', Vector: '⟡', Group: '◫' };
    return icons[kind] || '◇';
}

// ─── Properties panel ───
function updatePropertiesPanel() {
    const sel = app.get_selected();
    if (sel.length < 2) {
        propsContent.innerHTML = '<div class="prop-group"><div class="prop-info" style="padding: 8px 0; text-align: center;">No selection</div></div>';
        return;
    }
    if (sel.length > 2) {
        const count = sel.length / 2;
        propsContent.innerHTML = `<div class="prop-group"><div class="prop-info" style="padding: 8px 0; text-align: center;">${count} nodes selected</div></div>
        <div class="prop-group"><div class="prop-group-title">Actions</div>
        <div class="prop-info">⌘G to group | ⌘D to duplicate</div></div>`;
        return;
    }
    try {
        const nodeJson = app.get_node_info(sel[0], sel[1]);
        const node = JSON.parse(nodeJson);
        let html = '';
        html += `<div class="prop-group"><div class="prop-group-title">Node</div>
            <div class="prop-info">${node.name || 'Unnamed'} <span style="opacity:0.4">${node.kind || ''}</span></div></div>`;
        html += `<div class="prop-group"><div class="prop-group-title">Position</div>
            <div class="prop-row"><span class="prop-label">X</span><input class="prop-input" id="prop-x" type="number" value="${Math.round(node.x ?? 0)}"></div>
            <div class="prop-row"><span class="prop-label">Y</span><input class="prop-input" id="prop-y" type="number" value="${Math.round(node.y ?? 0)}"></div></div>`;
        html += `<div class="prop-group"><div class="prop-group-title">Size</div>
            <div class="prop-row"><span class="prop-label">W</span><input class="prop-input" type="number" value="${Math.round(node.width ?? 0)}" disabled></div>
            <div class="prop-row"><span class="prop-label">H</span><input class="prop-input" type="number" value="${Math.round(node.height ?? 0)}" disabled></div></div>`;
        if (node.fills && node.fills.length > 0) {
            const fill = node.fills[0];
            if (fill.Solid) {
                const c = fill.Solid;
                const hex = '#' + [c.r, c.g, c.b].map(v => Math.round(v * 255).toString(16).padStart(2, '0')).join('');
                html += `<div class="prop-group"><div class="prop-group-title">Fill</div>
                    <div class="prop-row"><div class="prop-swatch" style="background:${hex}"></div>
                    <span class="prop-info">${hex.toUpperCase()}</span></div></div>`;
            }
        }
        propsContent.innerHTML = html;
        const xInput = document.getElementById('prop-x');
        const yInput = document.getElementById('prop-y');
        if (xInput && yInput) {
            const commit = () => {
                app.set_node_position(sel[0], sel[1], parseFloat(xInput.value) || 0, parseFloat(yInput.value) || 0);
                render();
            };
            xInput.addEventListener('change', commit);
            yInput.addEventListener('change', commit);
        }
    } catch (_) {
        propsContent.innerHTML = '<div class="prop-group"><div class="prop-info" style="padding: 8px 0; text-align: center;">Error reading node</div></div>';
    }
}

function updatePanels() {
    updateLayersPanel();
    updatePropertiesPanel();
}

// ─── Init: generate default pattern ───
await generate('clusters', sliderToCount());
requestAnimationFrame(loop);
