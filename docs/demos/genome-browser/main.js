import init, { CanvasEngine } from '../../pkg/rendero.js';
await init();

const engine = new CanvasEngine("GenomeBrowser", 1);

// ─── DOM refs ───
const canvas = document.getElementById('canvas');
const labelCanvas = document.getElementById('label-canvas');
const rulerCanvas = document.getElementById('ruler');
const ideogramCanvas = document.getElementById('ideogram-canvas');
const ideogramViewport = document.getElementById('ideogram-viewport');
const loading = document.getElementById('loading');
const barFill = document.getElementById('bar-fill');
const barText = document.getElementById('bar-text');
const chrSelect = document.getElementById('chr-select');
const trackLabels = document.getElementById('track-labels');
const tooltip = document.getElementById('gene-tooltip');

// ─── Contexts ───
const gl = canvas.getContext('webgl2', { alpha: false, premultipliedAlpha: false, antialias: false });
if (!gl) { barText.textContent = 'WebGL2 not available.'; throw new Error('WebGL2 required'); }
const labelCtx = labelCanvas.getContext('2d');
const rulerCtx = rulerCanvas.getContext('2d');
const ideoCtx = ideogramCanvas.getContext('2d');

// ─── Layout constants (must match extract_genome.py) ───
const SCALE = 1000; // 1 world unit = 1000 bp
const CYTOBAND_H = 8;
const GENE_PLUS_Y = 12;
const GENE_MINUS_Y = 28;
const EXON_PLUS_Y = 14;
const EXON_MINUS_Y = 30;
const GENE_H = 4;
const EXON_H = 6;
const CHR_BLOCK_H = 50;
const CHR_GAP = 20;

// ─── Sizing ───
const dpr = window.devicePixelRatio || 1;
let cssW, cssH;
function resize() {
    const wrap = document.getElementById('canvas-wrap');
    cssW = wrap.clientWidth; cssH = wrap.clientHeight;
    canvas.width = cssW * dpr; canvas.height = cssH * dpr;
    canvas.style.width = cssW + 'px'; canvas.style.height = cssH + 'px';
    labelCanvas.width = cssW * dpr; labelCanvas.height = cssH * dpr;
    labelCanvas.style.width = cssW + 'px'; labelCanvas.style.height = cssH + 'px';
    rulerCanvas.width = cssW * dpr; rulerCanvas.height = 22 * dpr;
    rulerCanvas.style.width = cssW + 'px'; rulerCanvas.style.height = '22px';
    engine.set_viewport(cssW, cssH);
    const ideoBar = document.getElementById('ideogram-bar');
    ideogramCanvas.width = ideoBar.clientWidth;
    ideogramCanvas.height = ideoBar.clientHeight;
}
resize();
window.addEventListener('resize', () => { resize(); clampCamera(); });

// ─── Load metadata ───
barText.textContent = 'Loading metadata...';
const meta = await fetch('data/meta.json').then(r => r.json());

let currentChrIdx = 0;

// ─── Per-chromosome gene metadata (loaded lazily) ───
const geneData = {}; // chrIdx → [{name, start, end, strand, cds_start, cds_end, exons, x, w, y}]

async function loadGeneData(chrIdx) {
    if (geneData[chrIdx]) return geneData[chrIdx];
    const chr = meta.chromosomes[chrIdx];
    if (!chr.genes_file) return [];
    const data = await fetch(`data/${chr.genes_file}`).then(r => r.json());
    geneData[chrIdx] = data;
    return data;
}

// ─── Build chromosome selector ───
meta.chromosomes.forEach((chr, i) => {
    const opt = document.createElement('option');
    opt.value = i;
    opt.textContent = `${chr.name} (${(chr.size_bp / 1e6).toFixed(0)} Mb, ${chr.genes} genes)`;
    chrSelect.appendChild(opt);
});
chrSelect.addEventListener('change', () => jumpToChromosome(parseInt(chrSelect.value)));

// ─── Camera clamping ───
function clampCamera() {
    const cam = engine.get_camera();
    let [cx, cy, zoom] = cam;
    const chr = meta.chromosomes[currentChrIdx];
    const vw = cssW / zoom;
    const vh = cssH / zoom;
    cx = Math.max(-200, Math.min(cx, chr.w + 200 - vw));
    cy = Math.max(chr.y - 20, Math.min(cy, chr.y + chr.h + 20 - vh));
    engine.set_camera(cx, cy, zoom);
}

// ─── Navigate to chromosome ───
function jumpToChromosome(idx) {
    currentChrIdx = idx;
    chrSelect.value = idx;
    const chr = meta.chromosomes[idx];
    const vw = cssW;
    const vh = cssH;
    const yZoom = vh / (chr.h + 10);
    const xZoom = vw / Math.min(chr.w, 20000);
    let zoom = Math.max(0.02, Math.min(256, Math.min(yZoom, xZoom)));
    const cy = chr.y + chr.h / 2 - vh / zoom / 2;
    const cx = -10;
    engine.set_camera(cx, cy, zoom);
    clampCamera();
    // Eagerly load gene metadata for this chromosome
    loadGeneData(idx);
}

// ─── Update info displays ───
function updateInfo() {
    const chr = meta.chromosomes[currentChrIdx];
    const cam = engine.get_camera();
    const leftBp = Math.max(0, cam[0] * SCALE);
    const rightBp = Math.min(chr.size_bp, (cam[0] + cssW / cam[2]) * SCALE);
    document.getElementById('pos-display').innerHTML =
        `${chr.name}:<b>${formatBp(leftBp)}</b> – <b>${formatBp(rightBp)}</b> (${formatBp(rightBp - leftBp)})`;
}

// ─── Ideogram (cytoband minimap) ───
function renderIdeogram() {
    const chr = meta.chromosomes[currentChrIdx];
    const w = ideogramCanvas.width;
    const h = ideogramCanvas.height;
    ideoCtx.clearRect(0, 0, w, h);

    const pad = 20;
    const drawW = w - pad * 2;
    const chrLen = chr.w;

    ideoCtx.fillStyle = 'rgba(255,255,255,0.03)';
    ideoCtx.beginPath();
    ideoCtx.roundRect(pad, 4, drawW, h - 8, 6);
    ideoCtx.fill();

    // Viewport indicator
    const cam = engine.get_camera();
    const viewLeft = Math.max(0, cam[0]);
    const viewRight = cam[0] + cssW / cam[2];
    const vl = pad + (viewLeft / chrLen) * drawW;
    const vr = pad + (Math.min(viewRight, chrLen) / chrLen) * drawW;
    ideogramViewport.style.left = Math.max(pad, vl) + 'px';
    ideogramViewport.style.width = Math.max(3, vr - vl) + 'px';

    // Tick marks
    ideoCtx.fillStyle = 'rgba(255,255,255,0.15)';
    ideoCtx.font = '9px "JetBrains Mono", monospace';
    ideoCtx.textAlign = 'center';
    const tickStep = chrLen > 200000 ? 50000 : chrLen > 100000 ? 25000 : 10000;
    for (let pos = 0; pos <= chrLen; pos += tickStep) {
        const x = pad + (pos / chrLen) * drawW;
        ideoCtx.fillRect(x, h - 4, 1, 3);
        if (pos % (tickStep * 2) === 0) {
            ideoCtx.fillText(formatBp(pos * SCALE), x, 10);
        }
    }
}

// ─── Ideogram click + drag scrubbing ───
let ideoScrubbing = false;

function ideoScrub(clientX) {
    const rect = ideogramCanvas.getBoundingClientRect();
    const pad = 20;
    const drawW = rect.width - pad * 2;
    const frac = Math.max(0, Math.min(1, (clientX - rect.left - pad) / drawW));
    const chr = meta.chromosomes[currentChrIdx];
    const worldX = frac * chr.w;
    const cam = engine.get_camera();
    const vw = cssW / cam[2];
    engine.set_camera(worldX - vw / 2, cam[1], cam[2]);
    clampCamera();
}

const ideoBar = document.getElementById('ideogram-bar');
ideoBar.addEventListener('mousedown', (e) => {
    ideoScrubbing = true;
    ideoScrub(e.clientX);
});
window.addEventListener('mousemove', (e) => {
    if (ideoScrubbing) ideoScrub(e.clientX);
});
window.addEventListener('mouseup', () => { ideoScrubbing = false; });

// ─── Load chromosome data ───
barText.textContent = `Loading ${meta.total_points.toLocaleString()} annotations...`;
const totalChrs = meta.chromosomes.length;
let loadedChrs = 0;

for (const chr of meta.chromosomes) {
    const resp = await fetch(`data/${chr.file}`);
    const buf = await resp.arrayBuffer();
    const f32 = new Float32Array(buf);
    engine.add_point_cloud(gl, f32);
    engine.render_webgl(gl, cssW, cssH, dpr);
    loadedChrs++;
    barFill.style.width = (loadedChrs / totalChrs * 100) + '%';
    barText.textContent = `${chr.name} loaded (${loadedChrs}/${totalChrs})`;
}

// ─── Done loading ───
loading.style.display = 'none';
document.getElementById('s-genes').textContent = meta.total_genes.toLocaleString();
document.getElementById('s-exons').textContent = meta.total_exons.toLocaleString();
jumpToChromosome(0);

// Preload all gene metadata in background
for (let i = 0; i < meta.chromosomes.length; i++) {
    loadGeneData(i);
}

window._engine = engine;
window._meta = meta;
window._geneData = geneData;

// ─── Click-to-inspect: find gene at world coordinate ───
function findGeneAt(worldX, worldY) {
    const genes = geneData[currentChrIdx];
    if (!genes) return null;
    const chr = meta.chromosomes[currentChrIdx];

    let best = null;
    let bestDist = Infinity;

    for (const g of genes) {
        // Check if worldX is within gene bounds
        if (worldX < g.x || worldX > g.x + g.w) continue;
        // Check Y proximity (gene body or exon tracks)
        const dy = Math.abs(worldY - g.y);
        if (dy < 12 && dy < bestDist) {
            bestDist = dy;
            best = g;
        }
    }
    return best;
}

function showTooltip(gene, screenX, screenY) {
    document.getElementById('gt-name').textContent = gene.name;
    document.getElementById('gt-pos').textContent =
        `${formatBp(gene.start)} – ${formatBp(gene.end)}`;
    document.getElementById('gt-strand').innerHTML =
        gene.strand === '+' ?
        '<span class="gt-strand-plus">+ (forward)</span>' :
        '<span class="gt-strand-minus">− (reverse)</span>';
    document.getElementById('gt-cds').textContent =
        gene.cds_start === gene.cds_end ? 'non-coding' :
        `${formatBp(gene.cds_start)} – ${formatBp(gene.cds_end)}`;
    document.getElementById('gt-exons').textContent = gene.exons;
    document.getElementById('gt-len').textContent = formatBp(gene.end - gene.start);

    // Position tooltip near click, but keep on screen
    const tw = 320, th = 160;
    let tx = screenX + 16;
    let ty = screenY - 20;
    if (tx + tw > window.innerWidth) tx = screenX - tw - 16;
    if (ty + th > window.innerHeight) ty = window.innerHeight - th - 10;
    if (ty < 0) ty = 10;
    tooltip.style.left = tx + 'px';
    tooltip.style.top = ty + 'px';
    tooltip.classList.add('visible');
}

function hideTooltip() {
    tooltip.classList.remove('visible');
}

// ─── Mouse: pan & zoom ───
let isPanning = false;
let panStartX = 0, panStartY = 0;
let didPan = false;

canvas.addEventListener('mousedown', (e) => {
    if (e.button === 0) {
        isPanning = true;
        didPan = false;
        panStartX = e.clientX;
        panStartY = e.clientY;
        engine.pan_start(e.clientX, e.clientY);
    }
});
canvas.addEventListener('mousemove', (e) => {
    if (isPanning) {
        engine.pan_move(e.clientX, e.clientY);
        clampCamera();
        const dx = e.clientX - panStartX;
        const dy = e.clientY - panStartY;
        if (Math.abs(dx) + Math.abs(dy) > 4) didPan = true;
    }
});
canvas.addEventListener('mouseup', (e) => {
    if (isPanning) {
        isPanning = false;
        engine.pan_end();
        clampCamera();

        // If it was a click (not a pan), try to find a gene
        if (!didPan) {
            const cam = engine.get_camera();
            const rect = canvas.getBoundingClientRect();
            const sx = e.clientX - rect.left;
            const sy = e.clientY - rect.top;
            const worldX = cam[0] + sx / cam[2];
            const worldY = cam[1] + sy / cam[2];
            const gene = findGeneAt(worldX, worldY);
            if (gene) {
                showTooltip(gene, e.clientX, e.clientY);
            } else {
                hideTooltip();
            }
        }
    }
});
canvas.addEventListener('mouseleave', () => {
    if (isPanning) { isPanning = false; engine.pan_end(); clampCamera(); }
});
canvas.addEventListener('wheel', (e) => {
    e.preventDefault();
    engine.zoom(e.deltaY < 0 ? 1 : -1, e.clientX, e.clientY);
    clampCamera();
    hideTooltip();
}, { passive: false });

// ─── Keyboard shortcuts ───
document.addEventListener('keydown', (e) => {
    if (e.target.tagName === 'SELECT') return;
    const cam = engine.get_camera();
    const panStep = cssW / cam[2] * 0.3;
    if (e.key === 'ArrowLeft') { engine.set_camera(cam[0] - panStep, cam[1], cam[2]); clampCamera(); e.preventDefault(); }
    if (e.key === 'ArrowRight') { engine.set_camera(cam[0] + panStep, cam[1], cam[2]); clampCamera(); e.preventDefault(); }
    if (e.key === 'ArrowUp' || e.key === 'k') {
        e.preventDefault();
        jumpToChromosome(Math.max(0, currentChrIdx - 1));
    }
    if (e.key === 'ArrowDown' || e.key === 'j') {
        e.preventDefault();
        jumpToChromosome(Math.min(meta.chromosomes.length - 1, currentChrIdx + 1));
    }
    if (e.key === '+' || e.key === '=') { engine.zoom(1, cssW / 2, cssH / 2); clampCamera(); }
    if (e.key === '-') { engine.zoom(-1, cssW / 2, cssH / 2); clampCamera(); }
    if (e.key === '0') jumpToChromosome(currentChrIdx);
    if (e.key === 'Escape') hideTooltip();
});

// ─── Ruler rendering ───
function renderRuler() {
    const w = rulerCanvas.width;
    const h = rulerCanvas.height;
    rulerCtx.clearRect(0, 0, w, h);

    const cam = engine.get_camera();
    const zoom = cam[2];
    const leftWorld = cam[0];
    const rightWorld = cam[0] + w / zoom;
    const leftBp = leftWorld * SCALE;
    const rightBp = rightWorld * SCALE;
    const spanBp = rightBp - leftBp;

    const targetTicks = 8;
    const rawStep = spanBp / targetTicks;
    const mag = Math.pow(10, Math.floor(Math.log10(rawStep)));
    let step;
    if (rawStep / mag < 2) step = mag;
    else if (rawStep / mag < 5) step = mag * 2;
    else step = mag * 5;

    const firstTick = Math.ceil(leftBp / step) * step;

    rulerCtx.fillStyle = 'rgba(255,255,255,0.03)';
    rulerCtx.fillRect(0, 0, w, h);

    rulerCtx.font = '9px "JetBrains Mono", monospace';
    rulerCtx.textAlign = 'center';

    for (let bp = firstTick; bp <= rightBp; bp += step) {
        const x = (bp / SCALE - leftWorld) * zoom;
        rulerCtx.fillStyle = 'rgba(255,255,255,0.08)';
        rulerCtx.fillRect(x, h - 6, 1, 6);
        rulerCtx.fillStyle = 'rgba(255,255,255,0.3)';
        rulerCtx.fillText(formatBp(bp), x, 10);
    }

    rulerCtx.fillStyle = 'rgba(255,255,255,0.06)';
    rulerCtx.fillRect(0, h - 1, w, 1);
}

// ─── Format base pair position ───
function formatBp(bp) {
    const abs = Math.abs(bp);
    if (abs >= 1e9) return (bp / 1e9).toFixed(2) + ' Gb';
    if (abs >= 1e6) return (bp / 1e6).toFixed(2) + ' Mb';
    if (abs >= 1e3) return (bp / 1e3).toFixed(1) + ' kb';
    return bp.toFixed(0) + ' bp';
}

// ─── Gene name labels (from metadata) ───
function renderGeneLabels() {
    const cam = engine.get_camera();
    const [camX, camY, zoom] = cam;
    const w = labelCanvas.width;
    const h = labelCanvas.height;
    labelCtx.clearRect(0, 0, w, h);

    // Draw chromosome watermark
    const chr = meta.chromosomes[currentChrIdx];
    labelCtx.save();
    labelCtx.globalAlpha = 0.06;
    labelCtx.font = `700 ${Math.min(120, h * 0.3)}px 'DM Sans', system-ui`;
    labelCtx.textAlign = 'center';
    labelCtx.fillStyle = '#fff';
    labelCtx.fillText(chr.name, w / 2, h / 2 + 30);
    labelCtx.restore();

    // Show gene names when zoomed in enough
    if (zoom < 0.3) return;

    const genes = geneData[currentChrIdx];
    if (!genes) return;

    const fontSize = Math.max(7, Math.min(12, zoom * 4));
    labelCtx.font = `500 ${fontSize}px 'JetBrains Mono', monospace`;
    labelCtx.textAlign = 'center';

    const leftWorld = camX;
    const rightWorld = camX + w / zoom;

    // Avoid overlapping labels: track last drawn X per track
    let lastLabelXPlus = -Infinity;
    let lastLabelXMinus = -Infinity;
    const minGap = 60; // pixels between label centers

    for (const g of genes) {
        // Quick cull: skip genes outside viewport
        if (g.x + g.w < leftWorld || g.x > rightWorld) continue;

        const cx = g.x + g.w / 2; // center of gene in world coords
        const sx = (cx - camX) * zoom; // screen X

        const isPlus = g.strand === '+';
        // Position label above the gene body
        const labelWorldY = g.y - 2;
        const sy = (labelWorldY - camY) * zoom;

        if (sy < -20 || sy > h + 20 || sx < -100 || sx > w + 100) continue;

        // Check overlap
        if (isPlus) {
            if (sx - lastLabelXPlus < minGap) continue;
            lastLabelXPlus = sx;
        } else {
            if (sx - lastLabelXMinus < minGap) continue;
            lastLabelXMinus = sx;
        }

        // Shadow
        labelCtx.fillStyle = 'rgba(8,10,16,0.85)';
        labelCtx.fillText(g.name, sx + 1, sy + 1);
        // Text
        labelCtx.fillStyle = isPlus ? 'rgba(91,156,246,0.85)' : 'rgba(61,191,138,0.85)';
        labelCtx.fillText(g.name, sx, sy);
    }
}

// ─── Render loop ───
let lastTime = performance.now();
let frames = 0;
let fps = 0;

function render() {
    try { engine.render_webgl(gl, cssW, cssH, dpr); } catch (e) {}

    renderRuler();
    renderGeneLabels();
    renderIdeogram();
    updateInfo();

    frames++;
    const now = performance.now();
    if (now - lastTime > 500) {
        fps = Math.round(frames / (now - lastTime) * 1000);
        frames = 0;
        lastTime = now;
        document.getElementById('s-fps').textContent = fps;
    }
    requestAnimationFrame(render);
}
requestAnimationFrame(render);
