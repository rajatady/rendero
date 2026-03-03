/**
 * main.js — Infinite Canvas Landing Page
 *
 * The page IS an infinite canvas. Pan and zoom to explore islands
 * of content floating in a galaxy of 500K particles. The engine's
 * own navigation system IS the product demo.
 */
import init, { CanvasEngine } from '../pkg/rendero.js';
import { buildIslands, ISLANDS } from './islands.js';
import { createParticles, updateParticles } from './particles.js';

await init();

// ─── Engines ───
const engineA = new CanvasEngine("Particles", 1);
const engineB = new CanvasEngine("Content", 2);

// ─── Canvas setup ───
const canvasGL = document.getElementById('canvas-gl');
const canvas2D = document.getElementById('canvas-2d');
const gl = canvasGL.getContext('webgl2', { alpha: true, premultipliedAlpha: false, antialias: false });
const ctx = canvas2D.getContext('2d');
if (!gl) throw new Error('WebGL2 required');

// ─── DPR & resize ───
const dpr = window.devicePixelRatio || 1;
let cssW, cssH;

function resize() {
    cssW = window.innerWidth;
    cssH = window.innerHeight;
    for (const c of [canvasGL, canvas2D]) {
        c.width = cssW * dpr;
        c.height = cssH * dpr;
        c.style.width = cssW + 'px';
        c.style.height = cssH + 'px';
    }
    engineA.set_viewport(cssW, cssH);
    engineB.set_viewport(cssW, cssH);
}
resize();
window.addEventListener('resize', resize);

// ─── Wait for fonts ───
await document.fonts.ready;

// ─── Build content ───
const { clickRegions } = await buildIslands(engineB);
const particleState = createParticles(engineA, gl, cssW, cssH);

// ─── Hit test helper ───
function hitTestClick(screenX, screenY) {
    const [camX, camY, camZ] = engineA.get_camera();
    const worldX = camX + screenX / camZ;
    const worldY = camY + screenY / camZ;
    for (const r of clickRegions) {
        if (worldX >= r.x && worldX <= r.x + r.w && worldY >= r.y && worldY <= r.y + r.h) {
            return r;
        }
    }
    return null;
}

// ─── Mouse Pan / Zoom ───
let isPanning = false;

canvas2D.addEventListener('mousedown', e => {
    const hit = hitTestClick(e.clientX, e.clientY);
    if (hit && hit.action === 'url') {
        window.open(hit.target, '_blank');
        return;
    }
    isPanning = true;
    engineA.pan_start(e.clientX, e.clientY);
    engineB.pan_start(e.clientX, e.clientY);
    canvas2D.style.cursor = 'grabbing';
});

canvas2D.addEventListener('mousemove', e => {
    if (isPanning) {
        engineA.pan_move(e.clientX, e.clientY);
        engineB.pan_move(e.clientX, e.clientY);
        particleState.exploding = false;
    } else {
        const hit = hitTestClick(e.clientX, e.clientY);
        canvas2D.style.cursor = hit ? 'pointer' : 'grab';

        // Cursor proximity explode for text particles
        const [camX, camY, camZ] = engineA.get_camera();
        const wx = camX + e.clientX / camZ;
        const wy = camY + e.clientY / camZ;
        const tb = particleState;
        const pad = 80;
        if (wx > tb.textBoundsX - pad && wx < tb.textBoundsX + tb.textBoundsW + pad &&
            wy > tb.textBoundsY - pad && wy < tb.textBoundsY + tb.textBoundsH + pad) {
            particleState.exploding = true;
            particleState.explodeX = wx;
            particleState.explodeY = wy;
        } else {
            particleState.exploding = false;
        }
    }
});

canvas2D.addEventListener('mouseup', () => {
    if (isPanning) {
        isPanning = false;
        engineA.pan_end();
        engineB.pan_end();
        canvas2D.style.cursor = 'grab';
    }
});

canvas2D.addEventListener('mouseleave', () => {
    if (isPanning) {
        isPanning = false;
        engineA.pan_end();
        engineB.pan_end();
    }
    particleState.exploding = false;
});

canvas2D.addEventListener('wheel', e => {
    e.preventDefault();
    if (e.ctrlKey || e.metaKey) {
        const dir = e.deltaY < 0 ? 1 : -1;
        engineA.zoom(dir, e.clientX, e.clientY);
        engineB.zoom(dir, e.clientX, e.clientY);
    } else {
        engineA.pan_start(e.clientX, e.clientY);
        engineA.pan_move(e.clientX - e.deltaX, e.clientY - e.deltaY);
        engineA.pan_end();
        engineB.pan_start(e.clientX, e.clientY);
        engineB.pan_move(e.clientX - e.deltaX, e.clientY - e.deltaY);
        engineB.pan_end();
    }
}, { passive: false });

// ─── Touch: Pan + Pinch-to-Zoom ───
let touchPanning = false;
let touchStartDist = 0;
let touchStartZoom = 1;
let touchMidX = 0, touchMidY = 0;
let touchStartX = 0, touchStartY = 0;
let touchMoved = false;

function touchDist(t1, t2) {
    const dx = t1.clientX - t2.clientX;
    const dy = t1.clientY - t2.clientY;
    return Math.sqrt(dx * dx + dy * dy);
}

canvas2D.addEventListener('touchstart', e => {
    e.preventDefault();
    touchMoved = false;

    if (e.touches.length === 1) {
        const t = e.touches[0];
        touchStartX = t.clientX;
        touchStartY = t.clientY;
        touchPanning = true;
        engineA.pan_start(t.clientX, t.clientY);
        engineB.pan_start(t.clientX, t.clientY);
    } else if (e.touches.length === 2) {
        // Switch to pinch — end any single-finger pan first
        if (touchPanning) {
            engineA.pan_end();
            engineB.pan_end();
            touchPanning = false;
        }
        const t0 = e.touches[0], t1 = e.touches[1];
        touchStartDist = touchDist(t0, t1);
        const [, , z] = engineA.get_camera();
        touchStartZoom = z;
        touchMidX = (t0.clientX + t1.clientX) / 2;
        touchMidY = (t0.clientY + t1.clientY) / 2;
        // Start pan from midpoint for simultaneous pan+zoom
        engineA.pan_start(touchMidX, touchMidY);
        engineB.pan_start(touchMidX, touchMidY);
    }
}, { passive: false });

canvas2D.addEventListener('touchmove', e => {
    e.preventDefault();
    touchMoved = true;

    if (e.touches.length === 1 && touchPanning) {
        const t = e.touches[0];
        engineA.pan_move(t.clientX, t.clientY);
        engineB.pan_move(t.clientX, t.clientY);
    } else if (e.touches.length === 2) {
        const t0 = e.touches[0], t1 = e.touches[1];
        const newDist = touchDist(t0, t1);
        const newMidX = (t0.clientX + t1.clientX) / 2;
        const newMidY = (t0.clientY + t1.clientY) / 2;

        // Pan from midpoint movement
        engineA.pan_move(newMidX, newMidY);
        engineB.pan_move(newMidX, newMidY);

        // Zoom from pinch distance change
        const scale = newDist / touchStartDist;
        const newZoom = Math.max(0.05, Math.min(4, touchStartZoom * scale));
        const [camX, camY] = engineA.get_camera();
        engineA.set_camera(camX, camY, newZoom);
        engineB.set_camera(camX, camY, newZoom);
    }
}, { passive: false });

canvas2D.addEventListener('touchend', e => {
    e.preventDefault();

    if (e.touches.length === 0) {
        if (touchPanning) {
            engineA.pan_end();
            engineB.pan_end();
            touchPanning = false;
        }
        // Tap detection: if finger barely moved, treat as click
        if (!touchMoved) {
            const hit = hitTestClick(touchStartX, touchStartY);
            if (hit && hit.action === 'url') {
                window.open(hit.target, '_blank');
            } else {
                // Check if tap is on text area → mobile explode
                const [camX, camY, camZ] = engineA.get_camera();
                const wx = camX + touchStartX / camZ;
                const wy = camY + touchStartY / camZ;
                const tb = particleState;
                const pad = 100;
                if (wx > tb.textBoundsX - pad && wx < tb.textBoundsX + tb.textBoundsW + pad &&
                    wy > tb.textBoundsY - pad && wy < tb.textBoundsY + tb.textBoundsH + pad) {
                    particleState.mobileExplode = true;
                    particleState.mobileExplodeStart = performance.now();
                }
            }
        }
    } else if (e.touches.length === 1) {
        // Went from 2 fingers to 1 — end pinch, restart single pan
        engineA.pan_end();
        engineB.pan_end();
        const t = e.touches[0];
        touchPanning = true;
        engineA.pan_start(t.clientX, t.clientY);
        engineB.pan_start(t.clientX, t.clientY);
    }
}, { passive: false });

// ─── Responsive zoom ───
// Content designed for ~800px wide. Scale down on narrower screens.
function contentZoom() {
    return Math.min(1.0, cssW / 800);
}

// ─── Fly-To Animation ───
let flyAnim = null;

function flyTo(targetX, targetY, targetZoom, duration = 900, onComplete = null) {
    const [startX, startY, startZ] = engineA.get_camera();
    const startTime = performance.now();
    flyAnim = { startX, startY, startZ, targetX, targetY, targetZoom, startTime, duration, onComplete };
}

function updateFlyTo(now) {
    if (!flyAnim) return;
    const { startX, startY, startZ, targetX, targetY, targetZoom, startTime, duration, onComplete } = flyAnim;
    const t = Math.min((now - startTime) / duration, 1);
    const ease = 1 - Math.pow(1 - t, 3); // easeOutCubic

    const x = startX + (targetX - startX) * ease;
    const y = startY + (targetY - startY) * ease;
    const z = startZ + (targetZoom - startZ) * ease;

    engineA.set_camera(x, y, z);
    engineB.set_camera(x, y, z);

    if (t >= 1) {
        flyAnim = null;
        if (onComplete) onComplete();
    }
}

// ─── Nav Buttons ───
const navEl = document.getElementById('nav');
const navBtns = navEl.querySelectorAll('.nav-btn');

navBtns.forEach(btn => {
    btn.addEventListener('click', () => {
        const key = btn.dataset.island;
        const island = ISLANDS[key];
        if (!island) return;

        // Fly to island center, offset so it's centered on screen
        const z = contentZoom();
        const targetX = island.x - cssW / 2 / z;
        const targetY = island.y - cssH / 2 / z;
        flyTo(targetX, targetY, z, 900);

        // Update active state
        navBtns.forEach(b => b.classList.remove('active'));
        btn.classList.add('active');
    });
});

// ─── FPS tracking ───
let frameCount = 0;
let lastFpsTime = performance.now();
let currentFps = 0;
const fpsVal = document.getElementById('fps-val');
const ptsVal = document.getElementById('pts-val');
const zoomBadge = document.getElementById('zoom-badge');

// ─── Initial camera: zoomed out, showing constellation ───
const INITIAL_ZOOM = 0.12;
const initialCamX = -cssW / 2 / INITIAL_ZOOM;
const initialCamY = -cssH / 2 / INITIAL_ZOOM;
engineA.set_camera(initialCamX, initialCamY, INITIAL_ZOOM);
engineB.set_camera(initialCamX, initialCamY, INITIAL_ZOOM);

// Sequence: brief pause → fly in → particle convergence → nav appears
setTimeout(() => {
    const z = contentZoom();
    flyTo(-cssW / 2 / z, -cssH / 2 / z, z, 2000, () => {
        // Fly-in complete → start particle convergence at full zoom
        particleState.startConverge = true;
        // Show nav after convergence finishes
        setTimeout(() => navEl.classList.add('visible'), 2600);
    });
}, 600);

// ─── Render Loop ───
function frame(now) {
    requestAnimationFrame(frame);

    // Fly-to animation
    updateFlyTo(now);

    // Particle animation
    updateParticles(particleState, engineA, gl, now);

    // Render
    engineA.render_webgl(gl, cssW, cssH, dpr);
    ctx.clearRect(0, 0, canvas2D.width, canvas2D.height);
    engineB.render_canvas2d(ctx, cssW, cssH, dpr);

    // FPS
    frameCount++;
    const elapsed = now - lastFpsTime;
    if (elapsed >= 500) {
        currentFps = Math.round(frameCount / (elapsed / 1000));
        frameCount = 0;
        lastFpsTime = now;
        fpsVal.textContent = currentFps;
        ptsVal.textContent = engineA.point_cloud_count().toLocaleString();
        const [, , z] = engineA.get_camera();
        zoomBadge.textContent = `${Math.round(z * 100)}%`;
    }
}

requestAnimationFrame(frame);
