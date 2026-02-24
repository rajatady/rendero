import init, { CanvasEngine } from '../../pkg/rendero.js';
await init();

const app = new CanvasEngine("Rendero Design", 1);

// ─── DOM refs ───
const canvas = document.getElementById('canvas');
const ctx = canvas.getContext('2d');
const info = document.getElementById('info');
const layersList = document.getElementById('layers-list');
const pageTabs = document.getElementById('page-tabs');
const propsContent = document.getElementById('properties-content');
const ctxMenu = document.getElementById('ctx-menu');
const figInput = document.getElementById('fig-input');
const loading = document.getElementById('loading');

// ─── Canvas sizing ───
function resize() {
    const wrap = document.getElementById('canvas-wrap');
    canvas.width = wrap.clientWidth;
    canvas.height = wrap.clientHeight;
    app.set_viewport(canvas.width, canvas.height);
}
resize();
window.addEventListener('resize', () => { resize(); render(); });

// ─── Tool state ───
let currentTool = 'select';
let spaceHeld = false;
let dragStart = null;   // {sx, sy} for creation tools
let layersDirty = true;

function setTool(tool) {
    currentTool = tool;
    document.querySelectorAll('[data-tool]').forEach(b => {
        b.classList.toggle('active', b.dataset.tool === tool);
    });
    canvas.style.cursor = tool === 'select' ? 'default' : 'crosshair';
    if (tool !== 'pen' && app.pen_is_active()) app.pen_cancel();
}

document.querySelectorAll('[data-tool]').forEach(btn => {
    btn.addEventListener('click', () => setTool(btn.dataset.tool));
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
    info.innerHTML = `<b>${app.node_count()}</b> nodes | <b>${fps}</b> fps | <b>${app.drawn_count()}</b> drawn${selText}`;
}

// ─── Render ───
function render(force = false) {
    app.render_canvas2d(ctx, canvas.width, canvas.height);
    // Pen overlay
    if (app.pen_is_active()) drawPenOverlay();
}

let rafId = 0;
function loop(now) {
    render();
    updateStats(now);
    rafId = requestAnimationFrame(loop);
}

// ─── Pen overlay drawing ───
function drawPenOverlay() {
    const json = app.pen_get_state();
    if (!json) return;
    const state = JSON.parse(json);
    const cam = app.get_camera();
    const camX = cam[0], camY = cam[1], zoom = cam[2];
    const toSx = wx => (wx - camX) * zoom;
    const toSy = wy => (wy - camY) * zoom;

    ctx.save();
    ctx.strokeStyle = '#5b8af5';
    ctx.lineWidth = 2;
    const anchors = state.anchors;
    if (anchors.length >= 2) {
        ctx.beginPath();
        ctx.moveTo(toSx(anchors[0].x), toSy(anchors[0].y));
        for (let i = 1; i < anchors.length; i++) {
            const prev = anchors[i - 1], curr = anchors[i];
            if (prev.hox || prev.hoy || curr.hix || curr.hiy) {
                ctx.bezierCurveTo(
                    toSx(prev.x + prev.hox), toSy(prev.y + prev.hoy),
                    toSx(curr.x + curr.hix), toSy(curr.y + curr.hiy),
                    toSx(curr.x), toSy(curr.y)
                );
            } else {
                ctx.lineTo(toSx(curr.x), toSy(curr.y));
            }
        }
        ctx.stroke();
    }
    if (anchors.length >= 1) {
        const last = anchors[anchors.length - 1];
        ctx.beginPath();
        ctx.setLineDash([4, 4]);
        ctx.moveTo(toSx(last.x), toSy(last.y));
        ctx.lineTo(toSx(state.cx), toSy(state.cy));
        ctx.stroke();
        ctx.setLineDash([]);
    }
    for (const a of anchors) {
        const sx = toSx(a.x), sy = toSy(a.y);
        ctx.fillStyle = '#5b8af5';
        ctx.fillRect(sx - 4, sy - 4, 8, 8);
        ctx.strokeStyle = '#fff';
        ctx.lineWidth = 1;
        ctx.strokeRect(sx - 4, sy - 4, 8, 8);
        ctx.strokeStyle = '#5b8af5';
    }
    ctx.restore();
}

// ─── Screen → World coords ───
function toWorld(sx, sy) {
    const cam = app.get_camera();
    return { x: cam[0] + sx / cam[2], y: cam[1] + sy / cam[2] };
}

// ─── Mouse interaction ───
canvas.addEventListener('mousedown', (e) => {
    hideContextMenu();
    // Right click → context menu
    if (e.button === 2) { e.preventDefault(); return; }

    // Pan (middle click or space held)
    if (e.button === 1 || spaceHeld) {
        app.pan_start(e.offsetX, e.offsetY);
        canvas.style.cursor = 'grabbing';
        e.preventDefault();
        return;
    }

    // Pen tool
    if (currentTool === 'pen') {
        if (!app.pen_is_active()) app.pen_start();
        app.pen_mouse_down(e.offsetX, e.offsetY);
        render();
        return;
    }

    // Creation tools: start drag
    if (['frame', 'rect', 'ellipse', 'text'].includes(currentTool)) {
        dragStart = { sx: e.offsetX, sy: e.offsetY };
        return;
    }

    // Select tool
    const hit = app.mouse_down(e.offsetX, e.offsetY, e.shiftKey);
    canvas.style.cursor = hit ? 'move' : 'default';
    render();
    updatePanels();
});

canvas.addEventListener('mousemove', (e) => {
    // Pan (any button during pan)
    if (e.buttons > 0 && (e.button === 1 || spaceHeld)) {
        app.pan_move(e.offsetX, e.offsetY);
    }

    // Pen tool
    if (currentTool === 'pen' && app.pen_is_active()) {
        if (e.buttons === 1) app.pen_mouse_drag(e.offsetX, e.offsetY);
        else app.pen_mouse_move(e.offsetX, e.offsetY);
        if (app.needs_render()) render();
        return;
    }

    // Creation drag preview (just track, we don't draw preview rects yet)
    if (dragStart) return;

    // Select tool drag
    if (e.buttons === 1 && currentTool === 'select' && !spaceHeld) {
        app.mouse_move(e.offsetX, e.offsetY);
    }
    if (app.needs_render()) render();
});

canvas.addEventListener('mouseup', (e) => {
    // Pen tool
    if (currentTool === 'pen' && app.pen_is_active()) {
        app.pen_mouse_up();
        render();
        return;
    }

    // Creation tools: finish drag
    if (dragStart && ['frame', 'rect', 'ellipse', 'text'].includes(currentTool)) {
        const start = toWorld(dragStart.sx, dragStart.sy);
        const end = toWorld(e.offsetX, e.offsetY);
        const x = Math.min(start.x, end.x);
        const y = Math.min(start.y, end.y);
        let w = Math.abs(end.x - start.x);
        let h = Math.abs(end.y - start.y);
        // If just clicked (no drag), use default size
        if (w < 5 && h < 5) { w = 100; h = 100; }

        if (currentTool === 'frame') {
            app.add_frame('Frame', x, y, w, h, 1, 1, 1, 1);
        } else if (currentTool === 'rect') {
            app.add_rectangle('Rectangle', x, y, w, h, 0.85, 0.85, 0.85, 1);
        } else if (currentTool === 'ellipse') {
            app.add_ellipse('Ellipse', x, y, w, h, 0.85, 0.85, 0.85, 1);
        } else if (currentTool === 'text') {
            app.add_text('Text', 'Hello', x, y, 24, 0.1, 0.1, 0.1, 1);
        }
        dragStart = null;
        layersDirty = true;
        setTool('select');
        render();
        updatePanels();
        return;
    }

    // Select tool
    app.pan_end();
    app.mouse_up();
    canvas.style.cursor = spaceHeld ? 'grab' : (currentTool === 'select' ? 'default' : 'crosshair');
    updatePanels();
});

canvas.addEventListener('dblclick', () => {
    if (app.pen_is_active()) { app.pen_finish_open(); render(); }
});

// ─── Zoom/Pan via wheel ───
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
canvas.addEventListener('contextmenu', (e) => {
    e.preventDefault();
    showContextMenu(e.clientX, e.clientY);
});
document.addEventListener('click', (e) => {
    if (!ctxMenu.contains(e.target)) hideContextMenu();
});

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
        case 'paste': app.paste(); layersDirty = true; break;
        case 'duplicate': app.duplicate_selected(); layersDirty = true; break;
        case 'delete': app.delete_selected(); layersDirty = true; break;
        case 'group': app.group_selected(); layersDirty = true; break;
        case 'ungroup': app.ungroup_selected(); layersDirty = true; break;
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

    // Tool shortcuts (only when no modifier)
    if (!cmd && !e.altKey && !e.shiftKey) {
        const toolMap = { v: 'select', f: 'frame', r: 'rect', o: 'ellipse', t: 'text', p: 'pen' };
        if (toolMap[e.key]) { setTool(toolMap[e.key]); return; }
    }

    if (e.key === 'Delete' || e.key === 'Backspace') { doAction('delete'); }
    if (cmd && e.key === 'z' && !e.shiftKey) { e.preventDefault(); app.undo(); layersDirty = true; render(); updatePanels(); }
    if (cmd && (e.key === 'Z' || (e.key === 'z' && e.shiftKey))) { e.preventDefault(); app.redo(); layersDirty = true; render(); updatePanels(); }
    if (cmd && e.key === 'c') { app.copy_selected(); }
    if (cmd && e.key === 'v') { e.preventDefault(); doAction('paste'); }
    if (cmd && e.key === 'd') { e.preventDefault(); doAction('duplicate'); }
    if (cmd && e.key === 'g' && !e.shiftKey) { e.preventDefault(); doAction('group'); }
    if (cmd && e.key === 'g' && e.shiftKey) { e.preventDefault(); doAction('ungroup'); }
    if (cmd && e.key === '0') { e.preventDefault(); doAction('zoomfit'); }
    if (cmd && e.key === 'a') { e.preventDefault(); doAction('selectall'); }
    if (cmd && e.key === ']') { e.preventDefault(); doAction('front'); }
    if (cmd && e.key === '[') { e.preventDefault(); doAction('back'); }
    if (e.key === 'Escape') {
        if (app.pen_is_active()) { app.pen_cancel(); setTool('select'); }
        else app.exit_group();
        render();
    }
    if (e.key === 'Enter' && app.pen_is_active()) { app.pen_finish_open(); setTool('select'); render(); }
    if (e.key === ' ') { spaceHeld = true; canvas.style.cursor = 'grab'; e.preventDefault(); }
});

window.addEventListener('keyup', (e) => {
    if (e.key === ' ') { spaceHeld = false; canvas.style.cursor = currentTool === 'select' ? 'default' : 'crosshair'; }
});

// ─── Toolbar buttons ───
document.getElementById('btn-delete').addEventListener('click', () => doAction('delete'));
document.getElementById('btn-export-png').addEventListener('click', exportPNG);
document.getElementById('btn-export-svg').addEventListener('click', exportSVG);
document.getElementById('btn-import').addEventListener('click', () => figInput.click());
figInput.addEventListener('change', importFig);

// ─── Export PNG ───
function exportPNG() {
    const w = 1920, h = 1080;
    const pixels = app.render(w, h);
    const offCanvas = document.createElement('canvas');
    offCanvas.width = w;
    offCanvas.height = h;
    const offCtx = offCanvas.getContext('2d');
    const imgData = new ImageData(new Uint8ClampedArray(pixels), w, h);
    offCtx.putImageData(imgData, 0, 0);
    offCanvas.toBlob(blob => {
        const a = document.createElement('a');
        a.href = URL.createObjectURL(blob);
        a.download = 'rendero-export.png';
        a.click();
    });
}

// ─── Export SVG ───
function exportSVG() {
    const svg = app.export_svg(1920, 1080);
    const blob = new Blob([svg], { type: 'image/svg+xml' });
    const a = document.createElement('a');
    a.href = URL.createObjectURL(blob);
    a.download = 'rendero-export.svg';
    a.click();
}

// ─── Import .fig ───
function importFig(e) {
    const file = e.target.files[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = () => {
        const bytes = new Uint8Array(reader.result);
        const result = app.import_fig_binary(bytes);
        console.log('Import result:', result);
        layersDirty = true;
        app.zoom_to_fit();
        render();
        updatePanels();
        updatePageTabs();
    };
    reader.readAsArrayBuffer(file);
    figInput.value = '';
}

// ─── Layers panel ───
function updateLayersPanel() {
    try {
        const json = app.get_layers();
        const layers = JSON.parse(json);
        const sel = app.get_selected();
        const selSet = new Set();
        for (let i = 0; i < sel.length; i += 2) selSet.add(`${sel[i]}_${sel[i + 1]}`);

        let html = '';
        for (const layer of layers) {
            const key = `${layer.id[0]}_${layer.id[1]}`;
            const selected = selSet.has(key) ? ' selected' : '';
            const icon = getNodeIcon(layer.kind);
            html += `<div class="layer-item${selected}" data-counter="${layer.id[0]}" data-client="${layer.id[1]}">
                <span class="layer-icon">${icon}</span>${layer.name}</div>`;
        }
        layersList.innerHTML = html;
        layersList.querySelectorAll('.layer-item').forEach(el => {
            el.addEventListener('click', () => {
                app.select_node(+el.dataset.counter, +el.dataset.client);
                render();
                updatePanels();
            });
        });
    } catch (_) {}
}

function getNodeIcon(kind) {
    const icons = { Frame: '◻', Rectangle: '■', Ellipse: '●', Text: 'T', Vector: '⟡', Line: '╱', Image: '▣', Group: '◫' };
    return icons[kind] || '◇';
}

// ─── Properties panel ───
function updatePropertiesPanel() {
    const sel = app.get_selected();
    if (sel.length < 2) {
        propsContent.innerHTML = '<div class="prop-group"><div class="prop-info" style="padding: 8px 0; text-align: center;">No selection</div></div>';
        return;
    }
    try {
        const nodeJson = app.get_node_info(sel[0], sel[1]);
        const node = JSON.parse(nodeJson);
        let html = '';
        // Name
        html += `<div class="prop-group"><div class="prop-group-title">Node</div>
            <div class="prop-info">${node.name || 'Unnamed'} <span style="opacity:0.4">${node.kind || ''}</span></div></div>`;
        // Position
        html += `<div class="prop-group"><div class="prop-group-title">Position</div>
            <div class="prop-row"><span class="prop-label">X</span><input class="prop-input" id="prop-x" type="number" value="${Math.round(node.x ?? 0)}"></div>
            <div class="prop-row"><span class="prop-label">Y</span><input class="prop-input" id="prop-y" type="number" value="${Math.round(node.y ?? 0)}"></div></div>`;
        // Size
        html += `<div class="prop-group"><div class="prop-group-title">Size</div>
            <div class="prop-row"><span class="prop-label">W</span><input class="prop-input" type="number" value="${Math.round(node.width ?? 0)}" disabled></div>
            <div class="prop-row"><span class="prop-label">H</span><input class="prop-input" type="number" value="${Math.round(node.height ?? 0)}" disabled></div></div>`;
        // Fill
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
        // Bind position inputs
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

// ─── Page tabs ───
function updatePageTabs() {
    try {
        const pagesJson = app.get_pages();
        const pages = JSON.parse(pagesJson);
        const current = app.current_page_index();
        let html = '';
        pages.forEach((p, i) => {
            html += `<div class="page-tab ${i === current ? 'active' : ''}" data-page="${i}">${p.name}</div>`;
        });
        html += '<div class="page-tab-add" id="add-page">+</div>';
        pageTabs.innerHTML = html;
        pageTabs.querySelectorAll('.page-tab').forEach(el => {
            el.addEventListener('click', () => {
                app.switch_page(+el.dataset.page);
                layersDirty = true;
                render();
                updatePanels();
                updatePageTabs();
            });
        });
        document.getElementById('add-page')?.addEventListener('click', () => {
            app.add_page('New Page');
            app.switch_page(app.page_count() - 1);
            layersDirty = true;
            render();
            updatePanels();
            updatePageTabs();
        });
    } catch (_) {}
}

// ─── Demo content ───
function createDemoContent() {
    app.rename_page(0, "iPhone 16 Pro");

    // Artboard 1: iPhone hero
    const a1 = app.add_frame("iPhone-Hero", 0, 0, 1440, 900, 0, 0, 0, 1);
    app.set_insert_parent(a1[0], a1[1]);
    app.add_rectangle("Nav", 0, 0, 1440, 52, 0.1, 0.1, 0.1, 0.92);
    app.add_text("Logo", "Apple", 80, 14, 20, 1, 1, 1, 1);
    app.add_text("Nav-Store", "Store", 180, 18, 13, 0.85, 0.85, 0.85, 1);
    app.add_text("Nav-Mac", "Mac", 260, 18, 13, 0.85, 0.85, 0.85, 1);
    app.add_text("Nav-iPad", "iPad", 330, 18, 13, 0.85, 0.85, 0.85, 1);
    app.add_text("Nav-iPhone", "iPhone", 400, 18, 13, 0.85, 0.85, 0.85, 1);
    app.add_text("Hero-Title", "iPhone 16 Pro", 440, 200, 64, 0.85, 0.75, 0.55, 1);
    app.add_text("Hero-Sub", "Built for Apple Intelligence.", 450, 290, 28, 0.6, 0.6, 0.6, 1);
    app.add_text("CTA-Learn", "Learn more >", 560, 350, 21, 0.25, 0.55, 1, 1);
    app.add_text("CTA-Buy", "Buy >", 760, 350, 21, 0.25, 0.55, 1, 1);
    app.add_rectangle("Phone-Body", 560, 420, 320, 440, 0.12, 0.12, 0.12, 1);
    app.add_rectangle("Phone-Screen", 575, 438, 290, 400, 0.06, 0.06, 0.15, 1);
    app.clear_insert_parent();

    // Artboard 2: MacBook Air
    const a2 = app.add_frame("MacBook-Air", 1600, 0, 1440, 900, 0.96, 0.97, 0.98, 1);
    app.set_insert_parent(a2[0], a2[1]);
    app.add_text("MBA-Title", "MacBook Air", 480, 200, 64, 0.07, 0.07, 0.07, 1);
    app.add_text("MBA-Sub", "Lean. Mean. M4 machine.", 460, 290, 28, 0.4, 0.4, 0.4, 1);
    app.add_text("MBA-CTA", "Learn more >", 580, 350, 21, 0, 0.44, 0.89, 1);
    app.add_rounded_rect("MBA-Laptop", 420, 420, 600, 380, 0.75, 0.75, 0.77, 1, 12);
    app.add_rectangle("MBA-Screen", 440, 435, 560, 330, 0.2, 0.2, 0.25, 1);
    app.clear_insert_parent();

    // Artboard 3: Vision Pro
    const a3 = app.add_frame("Vision-Pro", 3200, 0, 1440, 900, 0, 0, 0, 1);
    app.set_insert_parent(a3[0], a3[1]);
    app.add_text("VP-Title", "Apple Vision Pro", 380, 200, 64, 0.85, 0.85, 0.87, 1);
    app.add_text("VP-Sub", "Welcome to the era of spatial computing.", 350, 290, 28, 0.5, 0.5, 0.5, 1);
    app.add_text("VP-CTA", "Learn more >", 580, 350, 21, 0.25, 0.55, 1, 1);
    app.add_rounded_rect("VP-Headset", 470, 430, 500, 280, 0.15, 0.15, 0.17, 1, 40);
    app.add_rounded_rect("VP-Glass", 490, 460, 460, 140, 0.25, 0.25, 0.3, 1, 30);
    app.add_ellipse("VP-Lens-L", 560, 480, 80, 80, 0.1, 0.1, 0.12, 1);
    app.add_ellipse("VP-Lens-R", 760, 480, 80, 80, 0.1, 0.1, 0.12, 1);
    app.clear_insert_parent();

    app.zoom_to_fit();
}

// ─── Init ───
createDemoContent();
updatePageTabs();
updatePanels();
loading.style.display = 'none';
rafId = requestAnimationFrame(loop);
