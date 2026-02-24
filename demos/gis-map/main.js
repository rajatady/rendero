import init, { CanvasEngine } from '../../pkg/rendero.js';
await init();

const engine = new CanvasEngine("GIS Map", 1);
const canvas = document.getElementById('canvas');
const ctx = canvas.getContext('2d');
const loading = document.getElementById('loading');
const progress = document.getElementById('progress');
const tooltip = document.getElementById('tooltip');

// ─── Canvas sizing ───
function resize() {
    canvas.width = window.innerWidth;
    canvas.height = window.innerHeight;
    engine.set_viewport(canvas.width, canvas.height);
}
resize();
window.addEventListener('resize', () => { resize(); render(); });

// ─── Seeded PRNG ───
let seed = 42;
function rand() { seed = (seed * 1664525 + 1013904223) & 0xffffffff; return (seed >>> 0) / 4294967296; }
function randRange(a, b) { return a + rand() * (b - a); }

// ─── Building types ───
const TYPES = [
    { name: 'Residential', colors: [[0.78,0.66,0.51], [0.82,0.72,0.58], [0.75,0.63,0.48], [0.85,0.76,0.62]] },
    { name: 'Commercial', colors: [[0.42,0.61,0.82], [0.48,0.65,0.85], [0.38,0.55,0.78], [0.50,0.68,0.88]] },
    { name: 'Industrial', colors: [[0.54,0.54,0.54], [0.50,0.50,0.50], [0.58,0.58,0.58], [0.46,0.46,0.46]] },
    { name: 'Park', colors: [[0.42,0.75,0.41], [0.38,0.70,0.38], [0.46,0.78,0.45], [0.35,0.65,0.35]] },
];

// ─── City generation ───
const BLOCK_COLS = 25, BLOCK_ROWS = 10;
const BLOCK_W = 400, BLOCK_H = 300;
const STREET_W = 40;
const TOTAL_W = BLOCK_COLS * (BLOCK_W + STREET_W);
const TOTAL_H = BLOCK_ROWS * (BLOCK_H + STREET_W);

let totalBuildings = 0;

async function generateCity() {
    // Streets (background rectangles)
    for (let row = 0; row < BLOCK_ROWS; row++) {
        for (let col = 0; col < BLOCK_COLS; col++) {
            const bx = col * (BLOCK_W + STREET_W);
            const by = row * (BLOCK_H + STREET_W);
            // Horizontal street below block
            if (row < BLOCK_ROWS - 1) {
                engine.add_rectangle(`st-h-${row}-${col}`, bx, by + BLOCK_H, BLOCK_W, STREET_W, 0.82, 0.80, 0.76, 1);
            }
            // Vertical street right of block
            if (col < BLOCK_COLS - 1) {
                engine.add_rectangle(`st-v-${row}-${col}`, bx + BLOCK_W, by, STREET_W, BLOCK_H, 0.82, 0.80, 0.76, 1);
            }
        }
    }

    // Buildings
    for (let row = 0; row < BLOCK_ROWS; row++) {
        for (let col = 0; col < BLOCK_COLS; col++) {
            const bx = col * (BLOCK_W + STREET_W);
            const by = row * (BLOCK_H + STREET_W);
            const blockType = Math.floor(rand() * 4);
            const numBuildings = 15 + Math.floor(rand() * 16); // 15-30

            for (let i = 0; i < numBuildings; i++) {
                // 70% chance to be the block's dominant type
                const typeIdx = rand() < 0.7 ? blockType : Math.floor(rand() * 4);
                const type = TYPES[typeIdx];
                const colorVariant = type.colors[Math.floor(rand() * type.colors.length)];

                // Building size varies by type
                let w, h;
                if (typeIdx === 0) { w = randRange(10, 25); h = randRange(10, 20); }      // Residential: small
                else if (typeIdx === 1) { w = randRange(20, 45); h = randRange(15, 35); }  // Commercial: medium
                else if (typeIdx === 2) { w = randRange(30, 70); h = randRange(20, 50); }  // Industrial: large
                else { w = randRange(15, 40); h = randRange(15, 40); }                     // Park

                const x = bx + randRange(5, BLOCK_W - w - 5);
                const y = by + randRange(5, BLOCK_H - h - 5);

                engine.add_rectangle(
                    `b-${type.name[0]}-${row}-${col}-${i}`,
                    x, y, w, h,
                    colorVariant[0], colorVariant[1], colorVariant[2], 1
                );
                totalBuildings++;
            }

            // Update progress every 10 blocks
            if ((row * BLOCK_COLS + col) % 10 === 0) {
                progress.textContent = `${totalBuildings.toLocaleString()} buildings...`;
                await new Promise(r => setTimeout(r, 0));
            }
        }
    }

    progress.textContent = `${totalBuildings.toLocaleString()} buildings — done`;
    engine.zoom_to_fit();
    loading.style.display = 'none';
}

// ─── Render + HUD ───
let frameTimes = [];
let lastTime = 0;

function render() {
    engine.render_canvas2d(ctx, canvas.width, canvas.height);
}

function loop(now) {
    if (lastTime > 0) {
        frameTimes.push(now - lastTime);
        if (frameTimes.length > 60) frameTimes.shift();
    }
    lastTime = now;
    const avg = frameTimes.length ? frameTimes.reduce((a, b) => a + b) / frameTimes.length : 0;
    const fps = avg > 0 ? Math.round(1000 / avg) : 0;
    render();
    document.getElementById('hud-nodes').textContent = engine.node_count().toLocaleString();
    document.getElementById('hud-drawn').textContent = engine.drawn_count().toLocaleString();
    document.getElementById('hud-fps').textContent = fps;
    const cam = engine.get_camera();
    document.getElementById('hud-zoom').textContent = cam[2].toFixed(2);
    requestAnimationFrame(loop);
}

// ─── Mouse interaction ───
let isPanning = false;

canvas.addEventListener('wheel', (e) => {
    e.preventDefault();
    if (e.ctrlKey || e.metaKey) {
        engine.zoom(e.deltaY < 0 ? 1 : -1, e.offsetX, e.offsetY);
    } else {
        engine.pan_start(e.offsetX, e.offsetY);
        engine.pan_move(e.offsetX - e.deltaX, e.offsetY - e.deltaY);
        engine.pan_end();
    }
}, { passive: false });

canvas.addEventListener('mousedown', (e) => {
    if (e.button === 0) {
        // Try to select a building
        const hit = engine.mouse_down(e.offsetX, e.offsetY, false);
        if (hit) {
            const sel = engine.get_selected();
            if (sel.length >= 2) {
                try {
                    const info = JSON.parse(engine.get_node_info(sel[0], sel[1]));
                    const name = info.name || '';
                    const typeLetter = name.split('-')[1] || '';
                    const typeMap = { R: 'Residential', C: 'Commercial', I: 'Industrial', P: 'Park' };
                    const typeName = typeMap[typeLetter] || 'Building';
                    tooltip.style.display = 'block';
                    tooltip.style.left = (e.clientX + 12) + 'px';
                    tooltip.style.top = (e.clientY - 30) + 'px';
                    tooltip.innerHTML = `<b>${typeName}</b><br>${Math.round(info.width)}×${Math.round(info.height)}`;
                } catch (_) {}
            }
            engine.mouse_up();
            render();
            return;
        }
        engine.mouse_up();
        tooltip.style.display = 'none';
        // Start pan
        isPanning = true;
        engine.pan_start(e.offsetX, e.offsetY);
        canvas.style.cursor = 'grabbing';
    }
});

canvas.addEventListener('mousemove', (e) => {
    if (isPanning) engine.pan_move(e.offsetX, e.offsetY);
});

canvas.addEventListener('mouseup', () => {
    if (isPanning) { isPanning = false; engine.pan_end(); canvas.style.cursor = 'grab'; }
});

// ─── Init ───
await generateCity();
requestAnimationFrame(loop);
