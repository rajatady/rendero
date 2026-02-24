import init, { CanvasEngine } from '../../pkg/rendero.js';

await init();

const canvas = document.getElementById('canvas');
const ctx = canvas.getContext('2d');
const loading = document.getElementById('loading');
const fpsStat = document.getElementById('fps-stat');
const frameStat = document.getElementById('frame-stat');
const nodesStat = document.getElementById('nodes-stat');
const drawnStat = document.getElementById('drawn-stat');
const createStat = document.getElementById('create-stat');
const slider = document.getElementById('node-slider');
const nodeTarget = document.getElementById('node-target');
const generateBtn = document.getElementById('generate-btn');
const benchmarkBtn = document.getElementById('benchmark-btn');
const resultsPanel = document.getElementById('results-panel');
const resultsBody = document.getElementById('results-body');

// Node count tiers
const TIERS = [1000, 5000, 10000, 80000, 200000, 800000, 2000000];
const TIER_LABELS = ['1K', '5K', '10K', '80K', '200K', '800K', '2M'];

let app = new CanvasEngine("Stress Test", 1);

// --- Canvas sizing ---
function resize() {
    canvas.width = window.innerWidth;
    canvas.height = window.innerHeight;
    app.set_viewport(canvas.width, canvas.height);
}
resize();
window.addEventListener('resize', () => { resize(); render(); });

// --- Apple-style artboard generator ---
const products = [
    { name: "iPhone 16 Pro Max", tc: [0.85,0.75,0.55], bg: [0,0,0] },
    { name: "MacBook Air", tc: [0.07,0.07,0.07], bg: [0.96,0.97,0.98] },
    { name: "iPad Pro", tc: [1,1,1], bg: [0,0,0] },
    { name: "Apple Watch Ultra", tc: [0.9,0.6,0.2], bg: [0,0,0] },
    { name: "AirPods Pro", tc: [1,1,1], bg: [0.96,0.96,0.96] },
    { name: "Apple Vision Pro", tc: [0.85,0.85,0.87], bg: [0,0,0] },
    { name: "iMac 24\"", tc: [0.2,0.5,0.9], bg: [1,1,1] },
    { name: "Mac Studio", tc: [0.5,0.5,0.5], bg: [0,0,0] },
    { name: "MacBook Pro 16\"", tc: [1,1,1], bg: [0.07,0.07,0.07] },
    { name: "iPad Air", tc: [0.3,0.4,0.8], bg: [1,1,1] },
];

const AW = 1440, AH = 900, AG = 100;
const NODES_PER_ARTBOARD = 20;

function generateNodes(targetCount) {
    // Reset engine
    app = new CanvasEngine("Stress Test", 1);
    resize();

    const artboardCount = Math.ceil(targetCount / NODES_PER_ARTBOARD);
    const cols = Math.ceil(Math.sqrt(artboardCount * (AW / AH)));
    const rows = Math.ceil(artboardCount / cols);

    const t0 = performance.now();
    let idx = 0;
    for (let row = 0; row < rows && idx < artboardCount; row++) {
        for (let col = 0; col < cols && idx < artboardCount; col++) {
            const p = products[idx % products.length];
            const x = col * (AW + AG);
            const y = row * (AH + AG);
            const [br, bg, bb] = p.bg;
            const [tr, tg, tb] = p.tc;
            const dark = br + bg + bb < 1.5;
            const s = dark ? 0.7 : 0.3;

            const frameId = app.add_frame(`A${idx}-${p.name}`, x, y, AW, AH, br, bg, bb, 1.0);
            app.set_insert_parent(frameId[0], frameId[1]);

            // Nav bar (7 nodes)
            app.add_rectangle(`A${idx}-Nav`, 0, 0, AW, 52, 0.1, 0.1, 0.1, 0.92);
            app.add_text(`A${idx}-Logo`, "Apple", 80, 14, 20.0, 1, 1, 1, 1);
            app.add_text(`A${idx}-NStore`, "Store", 180, 18, 13.0, 0.85, 0.85, 0.85, 1);
            app.add_text(`A${idx}-NMac`, "Mac", 260, 18, 13.0, 0.85, 0.85, 0.85, 1);
            app.add_text(`A${idx}-NiPad`, "iPad", 330, 18, 13.0, 0.85, 0.85, 0.85, 1);
            app.add_text(`A${idx}-NiPhn`, "iPhone", 400, 18, 13.0, 0.85, 0.85, 0.85, 1);
            app.add_text(`A${idx}-NWatch`, "Watch", 490, 18, 13.0, 0.85, 0.85, 0.85, 1);

            // Hero (4 nodes)
            app.add_text(`A${idx}-Title`, p.name, 400, 200, 56.0, tr, tg, tb, 1);
            app.add_text(`A${idx}-Sub`, `The all-new ${p.name}.`, 300, 290, 24.0, s, s, s, 1);
            app.add_text(`A${idx}-CTA1`, "Learn more >", 560, 350, 18.0, 0.25, 0.55, 1, 1);
            app.add_text(`A${idx}-CTA2`, "Buy >", 760, 350, 18.0, 0.25, 0.55, 1, 1);

            // Product area (3 nodes)
            app.add_rounded_rect(`A${idx}-Prod`, 500, 420, 440, 340, tr*0.3+0.1, tg*0.3+0.1, tb*0.3+0.1, 1, 24);
            app.add_rectangle(`A${idx}-Badge`, 520, 430, 100, 30, 0.25, 0.55, 1.0, 1);
            app.add_text(`A${idx}-Price`, `From $${999 + (col % 10) * 100}`, 650, 780, 16.0, s, s, s, 1);

            // Footer (3 nodes)
            app.add_rectangle(`A${idx}-Foot`, 0, AH-80, AW, 80, 0.96, 0.96, 0.96, 1);
            app.add_text(`A${idx}-Copy`, "© 2025 Apple Inc.", 550, AH-50, 12.0, 0.5, 0.5, 0.5, 1);
            app.add_ellipse(`A${idx}-Dot`, AW-60, AH-55, 20, 20, 0.8, 0.8, 0.8, 1);

            app.clear_insert_parent();
            idx++;
        }
    }
    const createMs = performance.now() - t0;
    const totalNodes = app.node_count();

    createStat.innerHTML = `Create: <b>${createMs.toFixed(0)}ms</b>`;
    nodesStat.innerHTML = `Nodes: <b>${formatNum(totalNodes)}</b>`;

    // Zoom to fit
    app.zoom_to_fit();

    return { createMs, totalNodes };
}

// --- FPS tracking ---
let frameTimes = [];
let lastTime = 0;

function updateFPS(now) {
    if (lastTime > 0) {
        const dt = now - lastTime;
        frameTimes.push(dt);
        if (frameTimes.length > 60) frameTimes.shift();
        const avg = frameTimes.reduce((a, b) => a + b) / frameTimes.length;
        const fps = 1000 / avg;

        fpsStat.className = fps >= 60 ? 'stat fps' : fps >= 30 ? 'stat warn' : 'stat bad';
        fpsStat.innerHTML = `FPS: <b>${fps.toFixed(0)}</b>`;
        frameStat.innerHTML = `Frame: <b>${avg.toFixed(1)}ms</b>`;
        drawnStat.innerHTML = `Drawn: <b>${formatNum(app.drawn_count())}</b>`;
    }
    lastTime = now;
}

function getMeanFPS() {
    if (frameTimes.length < 10) return 0;
    const avg = frameTimes.reduce((a, b) => a + b) / frameTimes.length;
    return 1000 / avg;
}

function getMeanFrameTime() {
    if (frameTimes.length < 10) return 0;
    return frameTimes.reduce((a, b) => a + b) / frameTimes.length;
}

// --- Rendering ---
function render() {
    app.render_canvas2d(ctx, canvas.width, canvas.height);
}

let rafId = 0;
function loop(now) {
    render();
    updateFPS(now);
    rafId = requestAnimationFrame(loop);
}

// --- Mouse interaction (pan/zoom) ---
let isPanning = false;

canvas.addEventListener('wheel', (e) => {
    e.preventDefault();
    app.zoom(-e.deltaY * 0.003, e.clientX, e.clientY);
}, { passive: false });

canvas.addEventListener('mousedown', (e) => {
    if (e.button === 0 || e.button === 1) {
        isPanning = true;
        app.pan_start(e.clientX, e.clientY);
    }
});

canvas.addEventListener('mousemove', (e) => {
    if (isPanning) app.pan_move(e.clientX, e.clientY);
});

canvas.addEventListener('mouseup', () => {
    if (isPanning) { isPanning = false; app.pan_end(); }
});

// --- Slider ---
slider.addEventListener('input', () => {
    nodeTarget.textContent = TIER_LABELS[slider.value];
});

generateBtn.addEventListener('click', () => {
    const tier = TIERS[slider.value];
    generateBtn.disabled = true;
    generateBtn.textContent = 'Generating...';
    // Allow UI to update before blocking
    requestAnimationFrame(() => {
        frameTimes = [];
        lastTime = 0;
        generateNodes(tier);
        generateBtn.disabled = false;
        generateBtn.textContent = 'Generate';
    });
});

// --- Auto Benchmark ---
benchmarkBtn.addEventListener('click', async () => {
    benchmarkBtn.disabled = true;
    generateBtn.disabled = true;
    benchmarkBtn.textContent = 'Running...';
    resultsBody.innerHTML = '';
    resultsPanel.style.display = 'block';

    const benchTiers = [1000, 10000, 80000, 200000, 800000, 2000000];
    const benchLabels = ['1K', '10K', '80K', '200K', '800K', '2M'];

    for (let i = 0; i < benchTiers.length; i++) {
        const tier = benchTiers[i];
        const label = benchLabels[i];

        // Generate
        frameTimes = [];
        lastTime = 0;
        const { createMs, totalNodes } = generateNodes(tier);

        // Warm up: render 30 frames
        for (let f = 0; f < 30; f++) {
            render();
            await new Promise(r => requestAnimationFrame(r));
        }

        // Measure: render 60 frames
        frameTimes = [];
        lastTime = 0;
        for (let f = 0; f < 60; f++) {
            const now = performance.now();
            render();
            updateFPS(now);
            await new Promise(r => requestAnimationFrame(r));
        }

        const fps = getMeanFPS();
        const frameTime = getMeanFrameTime();

        const row = document.createElement('tr');
        const fpsClass = fps >= 60 ? 'good' : fps >= 30 ? 'ok' : 'slow';
        row.innerHTML = `<td>${label} (${formatNum(totalNodes)})</td><td>${createMs.toFixed(0)}</td><td class="${fpsClass}">${fps.toFixed(0)}</td><td>${frameTime.toFixed(1)}</td>`;
        resultsBody.appendChild(row);
    }

    benchmarkBtn.disabled = false;
    generateBtn.disabled = false;
    benchmarkBtn.textContent = 'Run Benchmark';
});

// --- Helpers ---
function formatNum(n) {
    if (n >= 1000000) return (n / 1000000).toFixed(1) + 'M';
    if (n >= 1000) return (n / 1000).toFixed(n >= 10000 ? 0 : 1) + 'K';
    return String(n);
}

// --- Init ---
loading.style.display = 'none';

// Default: generate 10K nodes
generateNodes(TIERS[2]);
rafId = requestAnimationFrame(loop);
