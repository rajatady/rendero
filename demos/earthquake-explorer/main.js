import init, { CanvasEngine } from '../../pkg/rendero.js';
await init();

let engine = new CanvasEngine("EarthquakeExplorer", 1);

// ─── DOM refs ───
const mapCanvas = document.getElementById('map-canvas');
const dataCanvas = document.getElementById('data-canvas');
const loading = document.getElementById('loading');
const barFill = document.getElementById('bar-fill');
const barText = document.getElementById('bar-text');

// ─── Contexts ───
const mapCtx = mapCanvas.getContext('2d');
const gl = dataCanvas.getContext('webgl2', { alpha: true, premultipliedAlpha: true, antialias: true });
if (!gl) {
    barText.textContent = 'WebGL2 not available. This demo requires WebGL2.';
    throw new Error('WebGL2 required');
}

// ─── Canvas sizing ───
const dpr = window.devicePixelRatio || 1;
let cssW, cssH;
function resize() {
    cssW = window.innerWidth; cssH = window.innerHeight;
    mapCanvas.width = cssW * dpr; mapCanvas.height = cssH * dpr;
    dataCanvas.width = cssW * dpr; dataCanvas.height = cssH * dpr;
    engine.set_viewport(cssW, cssH);
}
resize();
window.addEventListener('resize', () => { resize(); clampCamera(); });

// ─── Mercator projection ───
// World space: square (Web Mercator maps the world to a square at zoom 0)
const WORLD_W = 12000;
const WORLD_H = 12000;

// 3x horizontal repeat offsets
const WRAP_OFFSETS = [-WORLD_W, 0, WORLD_W];

function lonToX(lon) {
    return ((lon + 180) / 360) * WORLD_W;
}

function latToY(lat) {
    const latRad = lat * Math.PI / 180;
    const mercN = Math.log(Math.tan(Math.PI / 4 + latRad / 2));
    const yNorm = (1 - mercN / Math.PI) / 2;
    return yNorm * WORLD_H;
}

function xToLon(x) {
    return (x / WORLD_W) * 360 - 180;
}

function yToLat(y) {
    const yNorm = y / WORLD_H;
    const mercN = (1 - 2 * yNorm) * Math.PI;
    return (180 / Math.PI) * Math.atan(Math.sinh(mercN));
}

// ─── Magnitude → color + size ───
function magToColor(mag) {
    if (mag < 4.0) return { r: 0.133, g: 0.773, b: 0.369 };
    if (mag < 5.0) return { r: 0.918, g: 0.702, b: 0.031 };
    if (mag < 6.0) return { r: 0.976, g: 0.451, b: 0.086 };
    if (mag < 7.0) return { r: 0.937, g: 0.267, b: 0.267 };
    return { r: 0.863, g: 0.149, b: 0.149 };
}

function magToSize(mag) {
    if (mag < 3.0) return 8;
    if (mag < 4.0) return 12;
    if (mag < 5.0) return 18;
    if (mag < 6.0) return 28;
    if (mag < 7.0) return 40;
    return 60;
}

// ═══════════════════════════════════════════════════════════
//  TILE MANAGER — CartoDB Dark Matter tiles
// ═══════════════════════════════════════════════════════════

const TILE_SIZE = 256;
const TILE_URL = (z, x, y) => `https://basemaps.cartocdn.com/dark_all/${z}/${x}/${y}@2x.png`;

// LRU tile cache
const tileCache = new Map(); // key: "z/x/y" → { img, loaded, lastUsed }
const MAX_CACHE = 300;

function getTileKey(z, x, y) { return `${z}/${x}/${y}`; }

function requestTile(z, x, y) {
    const key = getTileKey(z, x, y);
    if (tileCache.has(key)) {
        const entry = tileCache.get(key);
        entry.lastUsed = performance.now();
        return entry;
    }

    // Evict old entries
    if (tileCache.size >= MAX_CACHE) {
        let oldest = null, oldestKey = null;
        for (const [k, v] of tileCache) {
            if (!oldest || v.lastUsed < oldest.lastUsed) { oldest = v; oldestKey = k; }
        }
        if (oldestKey) tileCache.delete(oldestKey);
    }

    const entry = { img: new Image(), loaded: false, lastUsed: performance.now() };
    entry.img.crossOrigin = 'anonymous';
    entry.img.onload = () => { entry.loaded = true; };
    entry.img.onerror = () => { entry.loaded = false; };
    entry.img.src = TILE_URL(z, x, y);
    tileCache.set(key, entry);
    return entry;
}

// Slippy map math
function lonLatToTile(lon, lat, z) {
    const n = 1 << z;
    const tx = Math.floor((lon + 180) / 360 * n);
    const latRad = lat * Math.PI / 180;
    const ty = Math.floor((1 - Math.log(Math.tan(latRad) + 1 / Math.cos(latRad)) / Math.PI) / 2 * n);
    return { x: Math.max(0, Math.min(n - 1, tx)), y: Math.max(0, Math.min(n - 1, ty)) };
}

// Convert tile coords back to world-space bounds
function tileToWorldBounds(z, tx, ty) {
    const n = 1 << z;
    const lonLeft = tx / n * 360 - 180;
    const lonRight = (tx + 1) / n * 360 - 180;
    const latTop = Math.atan(Math.sinh(Math.PI * (1 - 2 * ty / n))) * 180 / Math.PI;
    const latBottom = Math.atan(Math.sinh(Math.PI * (1 - 2 * (ty + 1) / n))) * 180 / Math.PI;
    return {
        left: lonToX(lonLeft),
        right: lonToX(lonRight),
        top: latToY(latTop),
        bottom: latToY(latBottom),
    };
}

// Draw one copy of the tile grid at a given horizontal world-space offset
function drawTilesAtOffset(camX, camY, zoom, w, h, z, offsetX) {
    // Viewport in world coords (shifted by offset)
    const vpLeft = camX - offsetX;
    const vpRight = camX + w / zoom - offsetX;
    const vpTop = camY;
    const vpBottom = camY + h / zoom;

    // Check if this copy is visible at all
    if (vpRight < 0 || vpLeft > WORLD_W) return;
    if (vpBottom < 0 || vpTop > WORLD_H) return;

    // Clamp to valid lat/lon
    const lonL = Math.max(-180, xToLon(Math.max(0, vpLeft)));
    const lonR = Math.min(180, xToLon(Math.min(WORLD_W, vpRight)));
    const latT = Math.min(85.05, yToLat(Math.max(0, vpTop)));
    const latB = Math.max(-85.05, yToLat(Math.min(WORLD_H, vpBottom)));

    const tMin = lonLatToTile(lonL, latT, z);
    const tMax = lonLatToTile(lonR, latB, z);

    for (let ty = tMin.y; ty <= tMax.y; ty++) {
        for (let tx = tMin.x; tx <= tMax.x; tx++) {
            const bounds = tileToWorldBounds(z, tx, ty);
            const sx = (bounds.left + offsetX - camX) * zoom;
            const sy = (bounds.top - camY) * zoom;
            const sw = (bounds.right - bounds.left) * zoom;
            const sh = (bounds.bottom - bounds.top) * zoom;

            const entry = requestTile(z, tx, ty);
            if (entry.loaded) {
                mapCtx.drawImage(entry.img, sx, sy, sw, sh);
            } else {
                // Try parent tile as fallback
                const pz = z - 1;
                if (pz >= 0) {
                    const ptx = tx >> 1, pty = ty >> 1;
                    const pKey = getTileKey(pz, ptx, pty);
                    const parent = tileCache.get(pKey);
                    if (parent && parent.loaded) {
                        const subX = tx % 2, subY = ty % 2;
                        const srcSize = 256;
                        mapCtx.drawImage(parent.img,
                            subX * srcSize, subY * srcSize, srcSize, srcSize,
                            sx, sy, sw, sh);
                    } else {
                        mapCtx.fillStyle = '#0d1117';
                        mapCtx.fillRect(sx, sy, sw, sh);
                    }
                }
            }
        }
    }
}

function renderMapCanvas() {
    const cam = engine.get_camera();
    const camX = cam[0], camY = cam[1], zoom = cam[2];
    const w = cssW, h = cssH;

    mapCtx.setTransform(dpr, 0, 0, dpr, 0, 0);
    mapCtx.fillStyle = '#0a0e17';
    mapCtx.fillRect(0, 0, w, h);

    const z = Math.max(1, Math.min(17, Math.round(Math.log2(WORLD_W * zoom / 256))));

    // Draw tiles at 3 horizontal offsets
    for (const offset of WRAP_OFFSETS) {
        drawTilesAtOffset(camX, camY, zoom, w, h, z, offset);
    }
}

// ═══════════════════════════════════════════════════════════
//  EARTHQUAKE DATA
// ═══════════════════════════════════════════════════════════

async function fetchEarthquakes(range, minMag) {
    const now = new Date();
    const chunks = [];

    if (range === 'week') {
        const start = new Date(now); start.setDate(start.getDate() - 7);
        chunks.push({ start, end: now });
    } else if (range === 'month') {
        const start = new Date(now); start.setDate(start.getDate() - 30);
        chunks.push({ start, end: now });
    } else if (range === 'year') {
        for (let q = 0; q < 4; q++) {
            const end = new Date(now); end.setMonth(end.getMonth() - q * 3);
            const start = new Date(end); start.setMonth(start.getMonth() - 3);
            chunks.push({ start, end });
        }
    } else if (range === '2year') {
        for (let q = 0; q < 8; q++) {
            const end = new Date(now); end.setMonth(end.getMonth() - q * 3);
            const start = new Date(end); start.setMonth(start.getMonth() - 3);
            chunks.push({ start, end });
        }
    }

    barText.textContent = `Fetching M${minMag}+ earthquakes (${chunks.length} ${chunks.length > 1 ? 'queries' : 'query'})...`;
    barFill.style.width = '10%';

    let allFeatures = [];

    for (let i = 0; i < chunks.length; i++) {
        const { start, end } = chunks[i];
        const startStr = start.toISOString().split('T')[0];
        const endStr = end.toISOString().split('T')[0];

        barText.textContent = `Fetching chunk ${i + 1}/${chunks.length}: ${startStr} → ${endStr}...`;
        barFill.style.width = (10 + (i / chunks.length) * 40).toFixed(0) + '%';

        const url = `https://earthquake.usgs.gov/fdsnws/event/1/query?format=geojson&starttime=${startStr}&endtime=${endStr}&minmagnitude=${minMag}&orderby=magnitude&limit=20000`;
        const resp = await fetch(url);
        const data = await resp.json();
        allFeatures = allFeatures.concat(data.features);
    }

    // Deduplicate by event ID
    const seen = new Set();
    allFeatures = allFeatures.filter(f => {
        if (seen.has(f.id)) return false;
        seen.add(f.id);
        return true;
    });

    barFill.style.width = '50%';
    barText.textContent = `Received ${allFeatures.length.toLocaleString()} earthquakes, rendering...`;

    return allFeatures;
}

// ─── Magnitude groups ───
const MAG_GROUPS = {
    minor:    { min: 1.0, max: 4.0, label: 'Minor' },
    light:    { min: 4.0, max: 5.0, label: 'Light' },
    moderate: { min: 5.0, max: 6.0, label: 'Moderate' },
    strong:   { min: 6.0, max: 7.0, label: 'Strong' },
    major:    { min: 7.0, max: 99,  label: 'Major' },
};

const enabledGroups = new Set(['minor', 'light', 'moderate', 'strong', 'major']);

function magToGroup(mag) {
    if (mag < 4.0) return 'minor';
    if (mag < 5.0) return 'light';
    if (mag < 6.0) return 'moderate';
    if (mag < 7.0) return 'strong';
    return 'major';
}

// ─── Load + render earthquake data using GPU point clouds ───
let currentRange = 'month';
let isLoading = false;
let cachedFeatures = []; // cached after fetch, filtered client-side

function buildPointClouds(features, saveCam) {
    // Save camera before rebuilding
    let prevCam = null;
    if (saveCam) {
        try { prevCam = engine.get_camera(); } catch (_) {}
    }

    engine = new CanvasEngine("EarthquakeExplorer", 1);
    engine.set_viewport(cssW, cssH);

    // Filter by enabled magnitude groups
    const filtered = features.filter(f => enabledGroups.has(magToGroup(f.properties.mag || 0)));
    filtered.sort((a, b) => (a.properties.mag || 0) - (b.properties.mag || 0));

    // Update group counts in legend
    const counts = { minor: 0, light: 0, moderate: 0, strong: 0, major: 0 };
    for (const f of features) counts[magToGroup(f.properties.mag || 0)]++;
    for (const [group, count] of Object.entries(counts)) {
        const el = document.getElementById(`cnt-${group}`);
        if (el) el.textContent = count.toLocaleString();
    }

    // Build glow + solid buffers for 3 copies
    const n = filtered.length;
    const glowBuf = new Float32Array(n * 3 * 8);
    const solidBuf = new Float32Array(n * 3 * 8);

    for (let copy = 0; copy < 3; copy++) {
        const offsetX = WRAP_OFFSETS[copy];
        for (let i = 0; i < n; i++) {
            const f = filtered[i];
            const [lon, lat] = f.geometry.coordinates;
            const mag = f.properties.mag || 1;
            const x = lonToX(lon) + offsetX;
            const y = latToY(lat);
            const size = magToSize(mag);
            const c = magToColor(mag);

            const idx = (copy * n + i) * 8;

            const gs = size * 1.6;
            glowBuf[idx]     = x - gs / 2;
            glowBuf[idx + 1] = y - gs / 2;
            glowBuf[idx + 2] = gs;
            glowBuf[idx + 3] = gs;
            glowBuf[idx + 4] = c.r;
            glowBuf[idx + 5] = c.g;
            glowBuf[idx + 6] = c.b;
            glowBuf[idx + 7] = 0.15;

            solidBuf[idx]     = x - size / 2;
            solidBuf[idx + 1] = y - size / 2;
            solidBuf[idx + 2] = size;
            solidBuf[idx + 3] = size;
            solidBuf[idx + 4] = c.r;
            solidBuf[idx + 5] = c.g;
            solidBuf[idx + 6] = c.b;
            solidBuf[idx + 7] = 0.85;
        }
    }

    if (n > 0) {
        engine.add_point_cloud(gl, glowBuf);
        engine.add_point_cloud(gl, solidBuf);
    }

    // Restore or set camera
    if (prevCam) {
        engine.set_camera(prevCam[0], prevCam[1], prevCam[2]);
    } else {
        const zoom = Math.max(cssW / WORLD_W, cssH / WORLD_H);
        const cx = (WORLD_W / 2) - (cssW / zoom) / 2;
        const cy = (WORLD_H / 2) - (cssH / zoom) / 2;
        engine.set_camera(cx, cy, zoom);
    }
    clampCamera();

    document.getElementById('s-nodes').textContent = n.toLocaleString();
    document.getElementById('info-count').textContent = `Showing: ${n.toLocaleString()} / ${features.length.toLocaleString()} earthquakes`;

    return n;
}

async function loadData(range) {
    isLoading = true;
    loading.style.display = 'flex';
    barFill.style.width = '0%';

    // Always fetch M1.0+ so we have all data for group filtering
    const features = await fetchEarthquakes(range, 1.0);
    cachedFeatures = features;

    barText.textContent = `Building point clouds...`;
    barFill.style.width = '60%';
    await new Promise(r => setTimeout(r, 0));

    const n = buildPointClouds(features, false);

    const rangeLabel = { week: '7 days', month: '30 days', year: '365 days', '2year': '730 days' }[range];
    document.getElementById('info-range').textContent = `Range: ${rangeLabel}`;

    barFill.style.width = '100%';
    barText.textContent = `${features.length.toLocaleString()} earthquakes loaded`;

    await new Promise(r => setTimeout(r, 200));
    isLoading = false;
    loading.style.display = 'none';
}

// ─── Range buttons ───
document.querySelectorAll('.range-btn').forEach(btn => {
    btn.addEventListener('click', () => {
        document.querySelectorAll('.range-btn').forEach(b => b.classList.remove('active'));
        btn.classList.add('active');
        currentRange = btn.dataset.range;
        loadData(currentRange);
    });
});

// ─── Magnitude group toggles ───
document.querySelectorAll('.mag-row[data-mag]').forEach(row => {
    row.addEventListener('click', () => {
        const group = row.dataset.mag;
        if (enabledGroups.has(group)) {
            // Don't allow disabling all groups
            if (enabledGroups.size <= 1) return;
            enabledGroups.delete(group);
            row.classList.remove('active');
            row.classList.add('inactive');
        } else {
            enabledGroups.add(group);
            row.classList.add('active');
            row.classList.remove('inactive');
        }
        // Rebuild point clouds from cache (no re-fetch), preserve camera
        buildPointClouds(cachedFeatures, true);
    });
});

// ─── Camera clamping: never show blank area ───
function clampCamera() {
    const cam = engine.get_camera();
    let [cx, cy, zoom] = [cam[0], cam[1], cam[2]];
    const w = cssW, h = cssH;

    // Min zoom: map must fill viewport vertically (horizontal has 3x wrap)
    const minZoom = h / WORLD_H;
    if (zoom < minZoom) zoom = minZoom;

    // Clamp vertical pan: top edge can't go above 0, bottom edge can't go below WORLD_H
    const viewH = h / zoom;
    if (viewH >= WORLD_H) {
        cy = (WORLD_H - viewH) / 2;
    } else {
        if (cy < 0) cy = 0;
        if (cy + viewH > WORLD_H) cy = WORLD_H - viewH;
    }

    // Clamp horizontal pan: keep within the 3x repeat range [-WORLD_W, 2*WORLD_W]
    const viewW = w / zoom;
    const minX = -WORLD_W;
    const maxX = 2 * WORLD_W;
    if (viewW >= maxX - minX) {
        cx = minX + (maxX - minX - viewW) / 2;
    } else {
        if (cx < minX) cx = minX;
        if (cx + viewW > maxX) cx = maxX - viewW;
    }

    engine.set_camera(cx, cy, zoom);
}

// ─── Render ───
function render() {
    try {
        renderMapCanvas();
        engine.render_webgl(gl, cssW, cssH, dpr);
    } catch (e) {
        console.warn('Render error:', e.message);
    }
}

// ─── HUD loop ───
let frameTimes = [];
let lastTime = 0;

function loop(now) {
    if (lastTime > 0) {
        frameTimes.push(now - lastTime);
        if (frameTimes.length > 60) frameTimes.shift();
    }
    lastTime = now;

    if (!isLoading) {
        const avg = frameTimes.length ? frameTimes.reduce((a, b) => a + b) / frameTimes.length : 0;
        const fps = avg > 0 ? Math.round(1000 / avg) : 0;

        render();

        try {
            const nodes = engine.node_count();
            const pcPts = engine.point_cloud_count();
            document.getElementById('s-nodes').textContent = pcPts > 0
                ? (pcPts / 3).toLocaleString()  // show actual count, not 3x
                : nodes.toLocaleString();
            document.getElementById('s-drawn').textContent = engine.drawn_count().toLocaleString();
            document.getElementById('s-fps').textContent = fps;
            const cam = engine.get_camera();
            document.getElementById('s-zoom').textContent = cam[2].toFixed(2);
        } catch (_) {}
    }

    requestAnimationFrame(loop);
}

// ─── Mouse interaction (on data-canvas, forwarded to engine) ───
let isPanning = false;

dataCanvas.addEventListener('wheel', (e) => {
    e.preventDefault();
    if (e.ctrlKey || e.metaKey) {
        engine.zoom(e.deltaY < 0 ? 1 : -1, e.offsetX, e.offsetY);
        clampCamera();
    } else {
        engine.pan_start(e.offsetX, e.offsetY);
        engine.pan_move((e.offsetX - e.deltaX), (e.offsetY - e.deltaY));
        engine.pan_end();
        clampCamera();
    }
}, { passive: false });

dataCanvas.addEventListener('mousedown', (e) => {
    if (e.button === 0) {
        isPanning = true;
        engine.pan_start(e.offsetX, e.offsetY);
        dataCanvas.style.cursor = 'grabbing';
    }
});

dataCanvas.addEventListener('mousemove', (e) => {
    if (isPanning) {
        engine.pan_move(e.offsetX, e.offsetY);
        clampCamera();
    }
});

dataCanvas.addEventListener('mouseup', () => {
    if (isPanning) {
        isPanning = false;
        engine.pan_end();
        clampCamera();
        dataCanvas.style.cursor = 'grab';
    }
});

// ─── Init ───
await loadData(currentRange);
requestAnimationFrame(loop);
