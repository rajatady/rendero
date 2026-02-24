import init, { CanvasEngine } from '../../pkg/rendero.js';
await init();

const engine = new CanvasEngine("Whiteboard", 1);
const canvas = document.getElementById('canvas');
const ctx = canvas.getContext('2d');
const hud = document.getElementById('hud');

// ─── State ───
let tool = 'select';
let strokeWidth = 2;
let color = { r: 0.1, g: 0.1, b: 0.18 }; // #1a1a2e
let drawingPoints = [];
let dragStart = null;
let nameCounter = 0;
let isPanning = false;

// ─── Canvas sizing ───
function resize() {
    const toolbarW = 56;
    const colorbarH = 48;
    canvas.width = window.innerWidth - toolbarW;
    canvas.height = window.innerHeight - colorbarH;
    canvas.style.width = canvas.width + 'px';
    canvas.style.height = canvas.height + 'px';
    engine.set_viewport(canvas.width, canvas.height);
}
resize();
window.addEventListener('resize', () => { resize(); render(); });

// ─── Tools ───
document.querySelectorAll('.tbtn[data-tool]').forEach(btn => {
    btn.addEventListener('click', () => setTool(btn.dataset.tool));
});

function setTool(t) {
    tool = t;
    document.querySelectorAll('.tbtn[data-tool]').forEach(b => b.classList.toggle('active', b.dataset.tool === t));
    canvas.style.cursor = t === 'select' ? 'default' : t === 'eraser' ? 'crosshair' : 'crosshair';
}

// ─── Stroke width ───
document.querySelectorAll('.stroke-btn').forEach(btn => {
    btn.addEventListener('click', () => {
        strokeWidth = +btn.dataset.stroke;
        document.querySelectorAll('.stroke-btn').forEach(b => b.classList.toggle('active', b === btn));
    });
});

// ─── Colors ───
document.querySelectorAll('.cswatch').forEach(sw => {
    sw.addEventListener('click', () => {
        setColor(sw.dataset.color);
        document.querySelectorAll('.cswatch').forEach(s => s.classList.toggle('active', s === sw));
    });
});
document.getElementById('color-picker').addEventListener('input', (e) => {
    setColor(e.target.value);
    document.querySelectorAll('.cswatch').forEach(s => s.classList.remove('active'));
});

function setColor(hex) {
    const r = parseInt(hex.slice(1, 3), 16) / 255;
    const g = parseInt(hex.slice(3, 5), 16) / 255;
    const b = parseInt(hex.slice(5, 7), 16) / 255;
    color = { r, g, b };
}

// ─── World coords ───
function toWorld(sx, sy) {
    const cam = engine.get_camera();
    return { x: cam[0] + sx / cam[2], y: cam[1] + sy / cam[2] };
}

// ─── Mouse: drawing ───
canvas.addEventListener('pointerdown', (e) => {
    // Middle click or scroll pan
    if (e.button === 1) { isPanning = true; engine.pan_start(e.offsetX, e.offsetY); canvas.style.cursor = 'grabbing'; return; }
    if (e.button !== 0) return;

    if (tool === 'pencil') {
        drawingPoints = [{ x: e.offsetX, y: e.offsetY }];
        canvas.setPointerCapture(e.pointerId);
        return;
    }
    if (tool === 'rect' || tool === 'ellipse') {
        dragStart = { sx: e.offsetX, sy: e.offsetY };
        canvas.setPointerCapture(e.pointerId);
        return;
    }
    if (tool === 'text') {
        const text = prompt('Enter text:');
        if (text) {
            const w = toWorld(e.offsetX, e.offsetY);
            engine.add_text(`text-${nameCounter++}`, text, w.x, w.y, 24, color.r, color.g, color.b, 1);
            render();
        }
        return;
    }
    if (tool === 'eraser') {
        const hit = engine.mouse_down(e.offsetX, e.offsetY, false);
        if (hit) { engine.delete_selected(); render(); }
        engine.mouse_up();
        return;
    }
    if (tool === 'select') {
        engine.mouse_down(e.offsetX, e.offsetY, e.shiftKey);
        render();
    }
});

canvas.addEventListener('pointermove', (e) => {
    if (isPanning) { engine.pan_move(e.offsetX, e.offsetY); render(); return; }

    if (tool === 'pencil' && drawingPoints.length > 0) {
        drawingPoints.push({ x: e.offsetX, y: e.offsetY });
        // Live preview: draw on top of engine render
        render();
        ctx.save();
        ctx.strokeStyle = `rgb(${Math.round(color.r*255)},${Math.round(color.g*255)},${Math.round(color.b*255)})`;
        ctx.lineWidth = strokeWidth;
        ctx.lineCap = 'round';
        ctx.lineJoin = 'round';
        ctx.beginPath();
        ctx.moveTo(drawingPoints[0].x, drawingPoints[0].y);
        for (let i = 1; i < drawingPoints.length; i++) {
            ctx.lineTo(drawingPoints[i].x, drawingPoints[i].y);
        }
        ctx.stroke();
        ctx.restore();
        return;
    }

    if ((tool === 'rect' || tool === 'ellipse') && dragStart) {
        render();
        ctx.save();
        ctx.strokeStyle = `rgb(${Math.round(color.r*255)},${Math.round(color.g*255)},${Math.round(color.b*255)})`;
        ctx.lineWidth = 2;
        ctx.setLineDash([4, 4]);
        const x = Math.min(dragStart.sx, e.offsetX);
        const y = Math.min(dragStart.sy, e.offsetY);
        const w = Math.abs(e.offsetX - dragStart.sx);
        const h = Math.abs(e.offsetY - dragStart.sy);
        if (tool === 'rect') {
            ctx.strokeRect(x, y, w, h);
        } else {
            ctx.beginPath();
            ctx.ellipse(x + w / 2, y + h / 2, w / 2, h / 2, 0, 0, Math.PI * 2);
            ctx.stroke();
        }
        ctx.restore();
        return;
    }

    if (tool === 'select' && e.buttons === 1) {
        engine.mouse_move(e.offsetX, e.offsetY);
        if (engine.needs_render()) render();
    }
});

canvas.addEventListener('pointerup', (e) => {
    if (isPanning) { isPanning = false; engine.pan_end(); canvas.style.cursor = tool === 'select' ? 'default' : 'crosshair'; return; }

    if (tool === 'pencil' && drawingPoints.length > 1) {
        // Simplify path
        const minDist = strokeWidth * 3;
        const simplified = [drawingPoints[0]];
        for (let i = 1; i < drawingPoints.length; i++) {
            const last = simplified[simplified.length - 1];
            const dx = drawingPoints[i].x - last.x;
            const dy = drawingPoints[i].y - last.y;
            if (dx * dx + dy * dy >= minDist * minDist) {
                simplified.push(drawingPoints[i]);
            }
        }
        // Always include last point
        if (simplified.length > 1) {
            simplified.push(drawingPoints[drawingPoints.length - 1]);
        }

        // Convert to world coords
        const worldPts = simplified.map(p => toWorld(p.x, p.y));

        // Bounding box
        let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
        for (const p of worldPts) {
            minX = Math.min(minX, p.x); minY = Math.min(minY, p.y);
            maxX = Math.max(maxX, p.x); maxY = Math.max(maxY, p.y);
        }
        const w = Math.max(maxX - minX, 1);
        const h = Math.max(maxY - minY, 1);

        // Build path commands relative to bounding box origin
        const cmds = [];
        cmds.push(0, worldPts[0].x - minX, worldPts[0].y - minY); // MoveTo
        for (let i = 1; i < worldPts.length; i++) {
            cmds.push(1, worldPts[i].x - minX, worldPts[i].y - minY); // LineTo
        }

        engine.add_vector(`stroke-${nameCounter++}`, minX, minY, w, h, color.r, color.g, color.b, 1, new Float32Array(cmds));
        drawingPoints = [];
        render();
        return;
    }

    if ((tool === 'rect' || tool === 'ellipse') && dragStart) {
        const start = toWorld(dragStart.sx, dragStart.sy);
        const end = toWorld(e.offsetX, e.offsetY);
        const x = Math.min(start.x, end.x);
        const y = Math.min(start.y, end.y);
        let w = Math.abs(end.x - start.x);
        let h = Math.abs(end.y - start.y);
        if (w < 5 && h < 5) { w = 80; h = 80; }

        if (tool === 'rect') {
            engine.add_rectangle(`rect-${nameCounter++}`, x, y, w, h, color.r, color.g, color.b, 1);
        } else {
            engine.add_ellipse(`ellipse-${nameCounter++}`, x, y, w, h, color.r, color.g, color.b, 1);
        }
        dragStart = null;
        render();
        return;
    }

    if (tool === 'select') {
        engine.mouse_up();
    }
});

// ─── Wheel: pan/zoom ───
canvas.addEventListener('wheel', (e) => {
    e.preventDefault();
    if (e.ctrlKey || e.metaKey) {
        engine.zoom(e.deltaY < 0 ? 1 : -1, e.offsetX, e.offsetY);
    } else {
        engine.pan_start(e.offsetX, e.offsetY);
        engine.pan_move(e.offsetX - e.deltaX, e.offsetY - e.deltaY);
        engine.pan_end();
    }
    render();
}, { passive: false });

// ─── Keyboard ───
window.addEventListener('keydown', (e) => {
    const cmd = e.metaKey || e.ctrlKey;
    if (!cmd && !e.altKey) {
        const map = { v: 'select', p: 'pencil', r: 'rect', o: 'ellipse', t: 'text', x: 'eraser' };
        if (map[e.key]) { setTool(map[e.key]); return; }
    }
    if (cmd && e.key === 'z' && !e.shiftKey) { e.preventDefault(); engine.undo(); render(); }
    if (cmd && (e.key === 'Z' || (e.key === 'z' && e.shiftKey))) { e.preventDefault(); engine.redo(); render(); }
    if (e.key === 'Delete' || e.key === 'Backspace') { engine.delete_selected(); render(); }
});

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
    hud.textContent = `${engine.node_count()} nodes | ${fps} fps | ${engine.drawn_count()} drawn`;
    requestAnimationFrame(loop);
}

requestAnimationFrame(loop);
