import init, { CanvasEngine } from '../../pkg/rendero.js';
await init();

const engine = new CanvasEngine("NeuralNetViz", 1);

// ─── DOM refs ───
const canvas = document.getElementById('canvas');
const labelCanvas = document.getElementById('label-canvas');
const loading = document.getElementById('loading');
const barFill = document.getElementById('bar-fill');
const barText = document.getElementById('bar-text');

// ─── Contexts ───
const gl = canvas.getContext('webgl2', { alpha: false, premultipliedAlpha: false, antialias: false });
if (!gl) {
    barText.textContent = 'WebGL2 not available.';
    throw new Error('WebGL2 required');
}
const labelCtx = labelCanvas.getContext('2d');

// ─── Panel dimensions (pixels) for viewport clamping ───
const PANEL_LEFT = 236 + 16;   // nav width + gap
const PANEL_RIGHT = 260 + 16;  // info/colorbar width + gap
const PANEL_TOP = 42;          // HUD height
const PANEL_BOTTOM = 0;

// ─── Sizing ───
function resize() {
    const w = window.innerWidth, h = window.innerHeight;
    canvas.width = w; canvas.height = h;
    labelCanvas.width = w; labelCanvas.height = h;
    engine.set_viewport(w, h);
}
resize();
window.addEventListener('resize', () => { resize(); clampCamera(); });

// ─── HUD refs ───
const sPoints = document.getElementById('s-points');
const sDrawn = document.getElementById('s-drawn');
const sFps = document.getElementById('s-fps');
const sZoom = document.getElementById('s-zoom');

// ─── Load metadata ───
barText.textContent = 'Loading metadata...';
const meta = await fetch('data/meta.json').then(r => r.json());

// ─── World bounds for camera clamping ───
const WORLD_PAD = 500;
const WORLD_X_MIN = -WORLD_PAD;
const WORLD_X_MAX = meta.world_width + WORLD_PAD;
const WORLD_Y_MIN = -WORLD_PAD;
const WORLD_Y_MAX = meta.world_height + WORLD_PAD;

function clampCamera() {
    const cam = engine.get_camera(); // [x, y, zoom]
    let [cx, cy, zoom] = cam;
    // The visible canvas area (excluding panels) in world units
    const visW = (canvas.width - PANEL_LEFT - PANEL_RIGHT) / zoom;
    const visH = (canvas.height - PANEL_TOP - PANEL_BOTTOM) / zoom;
    // Camera x,y is the top-left of the full viewport (including panel areas).
    // We want the visible area (after panels) to stay within world bounds.
    // Visible area starts at: worldX = cx + PANEL_LEFT/zoom, worldY = cy + PANEL_TOP/zoom
    const visXmin = cx + PANEL_LEFT / zoom;
    const visYmin = cy + PANEL_TOP / zoom;
    // Clamp visible area within world bounds
    let newVisXmin = Math.max(WORLD_X_MIN, Math.min(visXmin, WORLD_X_MAX - visW));
    let newVisYmin = Math.max(WORLD_Y_MIN, Math.min(visYmin, WORLD_Y_MAX - visH));
    cx = newVisXmin - PANEL_LEFT / zoom;
    cy = newVisYmin - PANEL_TOP / zoom;
    engine.set_camera(cx, cy, zoom);
}

let currentTensorIdx = 1;

// ─── Navigate to a tensor with clamped zoom ───
function jumpToTensor(idx) {
    const t = meta.tensors[idx];
    // Usable viewport area (between panels)
    const usableW = canvas.width - PANEL_LEFT - PANEL_RIGHT;
    const usableH = canvas.height - PANEL_TOP - PANEL_BOTTOM;
    // Zoom to fit tensor in the usable area
    let zoom = Math.min(usableH / (t.h + 200), usableW / (t.w + 200));
    zoom = Math.max(0.02, Math.min(256, zoom));
    // Center the tensor in the usable viewport area
    // Usable center in screen coords: (PANEL_LEFT + usableW/2, PANEL_TOP + usableH/2)
    // That screen point should map to tensor center in world coords
    const tcx = t.x + t.w / 2;
    const tcy = t.y + t.h / 2;
    const cx = tcx - (PANEL_LEFT + usableW / 2) / zoom;
    const cy = tcy - (PANEL_TOP + usableH / 2) / zoom;
    engine.set_camera(cx, cy, zoom);
    clampCamera();
    currentTensorIdx = idx;
    updateTensorInfo(idx);
    highlightNav(idx);
}

// ─── Build layer navigation ───
const navList = document.getElementById('nav-list');
function buildNav() {
    let html = '';
    let currentLayer = -1;
    meta.tensors.forEach((t, i) => {
        if (t.name === 'model.embed_tokens.weight') {
            html += `<div class="nav-group">Embedding</div>`;
        } else if (t.name === 'model.norm.weight') {
            html += `<div class="nav-group">Output</div>`;
        } else {
            const m = t.name.match(/layers\.(\d+)\./);
            if (m) {
                const li = parseInt(m[1]);
                if (li !== currentLayer) {
                    currentLayer = li;
                    html += `<div class="nav-group">Layer ${li}</div>`;
                }
            }
        }
        const shortName = t.label.replace(/Layer \d+ — /, '');
        html += `<div class="nav-item" data-idx="${i}">
            ${shortName} <span class="nav-numel">${(t.numel / 1000).toFixed(0)}K</span>
        </div>`;
    });
    navList.innerHTML = html;
}
buildNav();

// ─── Nav click → jump to tensor ───
navList.addEventListener('click', (e) => {
    const item = e.target.closest('.nav-item');
    if (!item) return;
    jumpToTensor(parseInt(item.dataset.idx));
});

function highlightNav(idx) {
    const items = navList.querySelectorAll('.nav-item');
    items.forEach((el, i) => {
        el.classList.toggle('active', i === idx);
    });
    const active = navList.querySelector('.nav-item.active');
    if (active) active.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
}

function updateTensorInfo(idx) {
    const t = meta.tensors[idx];
    document.getElementById('ti-name').textContent = t.label;
    document.getElementById('ti-shape').textContent = t.shape.join(' × ');
    document.getElementById('ti-numel').textContent = t.numel.toLocaleString();
    document.getElementById('ti-min').textContent = t.min.toFixed(4);
    document.getElementById('ti-max').textContent = t.max.toFixed(4);
    document.getElementById('ti-mean').textContent = t.mean.toFixed(6);
    document.getElementById('ti-std').textContent = t.std.toFixed(4);
}

// ─── Load all tensor chunks progressively ───
const totalChunks = meta.tensors.reduce((sum, t) => sum + t.chunks.length, 0);
let loadedChunks = 0;
let loadedPoints = 0;

barText.textContent = `Loading ${meta.total_points.toLocaleString()} weights (${totalChunks} chunks)...`;

function flushToGPU() {
    engine.render_webgl(gl, canvas.width, canvas.height);
}

for (const tensor of meta.tensors) {
    for (const chunk of tensor.chunks) {
        const resp = await fetch(`data/${chunk.file}`);
        const buf = await resp.arrayBuffer();
        const f32 = new Float32Array(buf);
        engine.add_point_cloud(gl, f32);
        flushToGPU();
        loadedChunks++;
        loadedPoints += chunk.points;
        const pct = (loadedChunks / totalChunks * 100).toFixed(1);
        barFill.style.width = pct + '%';
        barText.textContent = `${loadedPoints.toLocaleString()} / ${meta.total_points.toLocaleString()} weights loaded (${pct}%)`;
        if (loadedChunks % 5 === 0) await new Promise(r => setTimeout(r, 0));
    }
}

// ─── Done loading — set camera ───
loading.style.display = 'none';
// Start at Layer 0 Q projection with zoom 1.3
{
    const t = meta.tensors[1];
    const usableW = canvas.width - PANEL_LEFT - PANEL_RIGHT;
    const usableH = canvas.height - PANEL_TOP - PANEL_BOTTOM;
    const zoom = 1.3;
    const tcx = t.x + t.w / 2;
    const tcy = t.y + t.h / 2;
    const cx = tcx - (PANEL_LEFT + usableW / 2) / zoom;
    const cy = tcy - (PANEL_TOP + usableH / 2) / zoom;
    engine.set_camera(cx, cy, zoom);
    clampCamera();
    currentTensorIdx = 1;
    updateTensorInfo(1);
    highlightNav(1);
}

// Expose for debugging
window._engine = engine;
window._meta = meta;

// ─── Mouse: pan & zoom with clamping ───
let isPanning = false;
canvas.addEventListener('mousedown', (e) => {
    if (e.button === 0) {
        isPanning = true;
        engine.pan_start(e.clientX, e.clientY);
    }
});
canvas.addEventListener('mousemove', (e) => {
    if (isPanning) {
        engine.pan_move(e.clientX, e.clientY);
        clampCamera();
    }
});
canvas.addEventListener('mouseup', () => {
    if (isPanning) { isPanning = false; engine.pan_end(); clampCamera(); }
});
canvas.addEventListener('mouseleave', () => {
    if (isPanning) { isPanning = false; engine.pan_end(); clampCamera(); }
});
canvas.addEventListener('wheel', (e) => {
    e.preventDefault();
    engine.zoom(e.deltaY < 0 ? 1 : -1, e.clientX, e.clientY);
    clampCamera();
}, { passive: false });

// ─── Keyboard shortcuts ───
document.addEventListener('keydown', (e) => {
    if (e.key === '0') jumpToTensor(1);
    if (e.key === 'ArrowUp' || e.key === 'k') {
        e.preventDefault();
        currentTensorIdx = Math.max(0, currentTensorIdx - 1);
        jumpToTensor(currentTensorIdx);
    }
    if (e.key === 'ArrowDown' || e.key === 'j') {
        e.preventDefault();
        currentTensorIdx = Math.min(meta.tensors.length - 1, currentTensorIdx + 1);
        jumpToTensor(currentTensorIdx);
    }
    if (e.key === 'PageUp') {
        e.preventDefault();
        currentTensorIdx = Math.max(0, currentTensorIdx - 9);
        jumpToTensor(currentTensorIdx);
    }
    if (e.key === 'PageDown') {
        e.preventDefault();
        currentTensorIdx = Math.min(meta.tensors.length - 1, currentTensorIdx + 9);
        jumpToTensor(currentTensorIdx);
    }
});

// ─── Toolbar buttons ───
document.getElementById('btn-fit').addEventListener('click', () => {
    const zoom = 0.02;
    const vw = canvas.width;
    const cx = meta.world_width / 2 - vw / zoom / 2;
    engine.set_camera(cx, WORLD_Y_MIN, zoom);
    clampCamera();
});
document.getElementById('btn-top').addEventListener('click', () => {
    jumpToTensor(0);
});

// ─── Detect which tensor is in view ───
function findVisibleTensor() {
    const cam = engine.get_camera();
    // Center of the usable viewport in world coords
    const cx = cam[0] + (PANEL_LEFT + (canvas.width - PANEL_LEFT - PANEL_RIGHT) / 2) / cam[2];
    const cy = cam[1] + (PANEL_TOP + (canvas.height - PANEL_TOP - PANEL_BOTTOM) / 2) / cam[2];
    let bestIdx = 0;
    let bestDist = Infinity;
    for (let i = 0; i < meta.tensors.length; i++) {
        const t = meta.tensors[i];
        const tcx = t.x + t.w / 2;
        const tcy = t.y + t.h / 2;
        const d = Math.abs(cx - tcx) + Math.abs(cy - tcy);
        if (d < bestDist) { bestDist = d; bestIdx = i; }
    }
    return bestIdx;
}

// ─── Label rendering on Canvas2D overlay ───
function renderLabels() {
    const cam = engine.get_camera();
    const [camX, camY, zoom] = cam;
    const w = labelCanvas.width;
    const h = labelCanvas.height;

    labelCtx.clearRect(0, 0, w, h);

    // Only render labels when zoom is high enough to read them
    // At zoom < 0.05, labels would be unreadable
    if (zoom < 0.03) return;

    // Adaptive font size based on zoom
    const fontSize = Math.max(8, Math.min(14, zoom * 12));
    labelCtx.font = `500 ${fontSize}px 'Azeret Mono', monospace`;
    labelCtx.textAlign = 'left';

    for (let i = 0; i < meta.tensors.length; i++) {
        const t = meta.tensors[i];
        // Label position: below the tensor, left-aligned
        const labelWorldX = t.x;
        const labelWorldY = t.y + t.h + 8; // 8 world units below tensor

        // World → screen transform
        const sx = (labelWorldX - camX) * zoom;
        const sy = (labelWorldY - camY) * zoom;

        // Cull if off-screen (with margin)
        if (sx > w + 200 || sy > h + 50 || sx < -400 || sy < -50) continue;

        // Also cull label text width: tensor name
        const shortName = t.label.replace(/Layer \d+ — /, '');
        const shape = `[${t.shape.join('×')}]`;
        const text = `${shortName} ${shape}`;

        // Shadow for readability
        labelCtx.fillStyle = 'rgba(8,9,12,0.8)';
        labelCtx.fillText(text, sx + 1, sy + 1);
        // Label text
        labelCtx.fillStyle = 'rgba(196,202,214,0.7)';
        labelCtx.fillText(text, sx, sy);
    }
}

// ─── Render loop ───
let lastTime = performance.now();
let frames = 0;
let fps = 0;

function render() {
    let drawn = 0;
    try {
        drawn = engine.render_webgl(gl, canvas.width, canvas.height) || 0;
    } catch (e) {
        // silent
    }

    // Render text labels on overlay
    renderLabels();

    frames++;
    const now = performance.now();
    if (now - lastTime > 500) {
        fps = Math.round(frames / (now - lastTime) * 1000);
        frames = 0;
        lastTime = now;

        sPoints.textContent = meta.total_points.toLocaleString();
        const pcCount = engine.point_cloud_count();
        sDrawn.textContent = pcCount.toLocaleString();
        sFps.textContent = fps;
        const cam = engine.get_camera();
        sZoom.textContent = cam[2].toFixed(2);

        const visIdx = findVisibleTensor();
        currentTensorIdx = visIdx;
        updateTensorInfo(visIdx);
        highlightNav(visIdx);
    }
    requestAnimationFrame(render);
}
requestAnimationFrame(render);
