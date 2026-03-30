// ═══════════════════════════════════════════════════════════════
// Engine Bridge — shared Rendero engine instance + node registry
// ═══════════════════════════════════════════════════════════════
//
// CRITICAL: The WASM engine uses &mut self borrows internally.
// We must NEVER call two engine methods concurrently. All engine
// calls are funneled through a serial queue and flushed once per
// frame, right before render.

import { ensureRenderoNamespace } from './rendero-api.js';
import { setViewport as setCssViewport } from './css-values.js';

let _engine = null;
let _canvas = null;
let _ctx = null;
let _dirty = true;
let _nextId = 1;
let _renderer = null;

// Map from shimNode._engineId → { counter, clientId }
const _nodeRegistry = new Map();

// Pending engine operations — executed sequentially before render
const _ops = [];

function createCanvas2dRenderer() {
    return {
        name: 'canvas2d',
        render(engine, ctx, canvas) {
            const dpr = window.devicePixelRatio || 1;
            engine.render_canvas2d(ctx, canvas.width, canvas.height, dpr);
        },
    };
}

function createRasterRenderer() {
    return {
        name: 'raster',
        render(engine, ctx, canvas) {
            const pixels = engine.render(canvas.width, canvas.height);
            const clamped = pixels instanceof Uint8ClampedArray ? pixels : new Uint8ClampedArray(pixels);
            const image = new ImageData(clamped, canvas.width, canvas.height);
            ctx.setTransform(1, 0, 0, 1, 0, 0);
            ctx.putImageData(image, 0, 0);
        },
    };
}

function createBrowserRenderer() {
    const requested = globalThis.__RENDERO_BROWSER_RENDERER__ || 'raster';
    return requested === 'raster' ? createRasterRenderer() : createCanvas2dRenderer();
}

function switchRenderer(nextName, reason) {
    if (_renderer?.name === nextName) return;
    _renderer = nextName === 'raster' ? createRasterRenderer() : createCanvas2dRenderer();
    console.warn(`Switching browser renderer to ${_renderer.name}`, reason || '');
    _dirty = true;
}

export function initEngine(engine, canvas) {
    globalThis.__RENDERO_NATIVE__ = false;
    _engine = engine;
    _canvas = canvas;
    _ctx = canvas.getContext('2d');
    _renderer = createBrowserRenderer();

    const dpr = window.devicePixelRatio || 1;
    canvas.width = window.innerWidth * dpr;
    canvas.height = window.innerHeight * dpr;
    setCssViewport(canvas.clientWidth || window.innerWidth, canvas.clientHeight || window.innerHeight);
    engine.set_viewport(canvas.width, canvas.height);
    engine.set_camera(0, 0, dpr);

    window.addEventListener('resize', () => {
        const dpr = window.devicePixelRatio || 1;
        canvas.width = window.innerWidth * dpr;
        canvas.height = window.innerHeight * dpr;
        setCssViewport(canvas.clientWidth || window.innerWidth, canvas.clientHeight || window.innerHeight);
        _engine.set_viewport(canvas.width, canvas.height);
        _engine.set_camera(0, 0, dpr);
        _dirty = true;
    });

    _startRenderLoop();

    ensureRenderoNamespace({
        engineBridge: {
            allocId,
            markDirty,
            hitTest,
            getNodeIds,
            engineGetBounds,
            getCamera,
            setCamera,
        },
        nativeApi: {
            storage: {
                get: (key) => {
                    try { return window.localStorage?.getItem(key); } catch { return null; }
                },
                set: (key, value) => {
                    try {
                        window.localStorage?.setItem(key, value);
                        return true;
                    } catch {
                        return false;
                    }
                },
            },
            clipboard: {
                readText: () => navigator.clipboard?.readText?.() ?? null,
                writeText: (text) => navigator.clipboard?.writeText?.(text) ?? null,
            },
            dialogs: {
                alert: (message) => {
                    window.alert?.(message);
                    return true;
                },
            },
            notifications: {
                requestPermission: () => window.Notification?.requestPermission?.() ?? null,
            },
            haptics: {
                impact: () => true,
            },
            media: {
                pickImage: () => null,
            },
        },
    });
}

export function getEngine() { return _engine; }
export function getCanvas() { return _canvas; }
export function allocId() { return _nextId++; }
export function markDirty() { _dirty = true; }
export function getCamera() {
    if (!_engine) return { x: 0, y: 0, zoom: 1 };
    const [x, y, zoom] = _engine.get_camera();
    return { x, y, zoom };
}
export function setCamera(x, y, zoom) {
    if (!_engine) return;
    const current = getCamera();
    _engine.set_camera(x, y, zoom ?? current.zoom);
    _dirty = true;
}

export function registerNode(engineId, counter, clientId) {
    _nodeRegistry.set(engineId, { counter, clientId });
}
export function unregisterNode(engineId) {
    _nodeRegistry.delete(engineId);
}
export function getNodeIds(engineId) {
    return _nodeRegistry.get(engineId) || null;
}

// ─── All engine mutations go through _ops queue ───

function _enqueue(fn) {
    _ops.push(fn);
    _dirty = true;
}

function _flushOps() {
    if (_ops.length === 0) return;
    const batch = _ops.splice(0);
    for (const fn of batch) {
        try { fn(); } catch (e) {
            console.warn('Engine op error:', e.message);
        }
    }
}

// Set insert parent before creating a child node
export function setInsertParent(engineId) {
    const ids = _nodeRegistry.get(engineId);
    if (ids && ids.counter >= 0) {
        _engine.set_insert_parent(ids.counter, ids.clientId);
    }
}

export function clearInsertParent() {
    _engine.clear_insert_parent();
}

// Create a Frame node in the engine
// We pre-allocate a slot in the registry and enqueue the actual creation.
// Returns a placeholder — the real counter/clientId are set when the op runs.
export function engineCreateFrame(engineId, name) {
    try {
        const ids = _engine.add_frame(name, 0, 0, 0, 0, 0, 0, 0, 0);
        const result = { counter: ids[0], clientId: ids[1] };
        registerNode(engineId, ids[0], ids[1]);
        _engine.set_node_clip_content(ids[0], ids[1], false);
        _dirty = true;
        return result;
    } catch (e) {
        console.warn('engineCreateFrame failed:', name, e.message);
        return null;
    }
}

// Create a Text node in the engine
export function engineCreateText(engineId, name, text) {
    try {
        const safeText = text || ' ';
        const ids = _engine.add_text(name, safeText, 0, 0, 16, 0, 0, 0, 1);
        const result = { counter: ids[0], clientId: ids[1] };
        registerNode(engineId, ids[0], ids[1]);
        _dirty = true;
        return result;
    } catch (e) {
        console.warn('engineCreateText failed:', name, e.message);
        return null;
    }
}

// Delete a node from the engine
export function engineDeleteNode(engineId) {
    const ids = _nodeRegistry.get(engineId);
    if (!ids) return;
    unregisterNode(engineId);
    _enqueue(() => {
        try {
            _engine.select_node(ids.counter, ids.clientId);
            _engine.delete_selected();
        } catch (e) {
            // Node might already be deleted (parent was deleted first)
        }
    });
    _dirty = true;
}

// Get world bounds for a node
export function engineGetBounds(engineId) {
    // Must flush first so bounds reflect current state
    _flushOps();
    const ids = _nodeRegistry.get(engineId);
    if (!ids || ids.counter < 0) return { x: 0, y: 0, width: 0, height: 0 };
    try {
        const b = _engine.get_node_world_bounds(ids.counter, ids.clientId);
        if (!b || b.length < 4) return { x: 0, y: 0, width: 0, height: 0 };
        return { x: b[0], y: b[1], width: b[2], height: b[3] };
    } catch (e) {
        return { x: 0, y: 0, width: 0, height: 0 };
    }
}

// Apply a single engine property (queued)
export function engineSetProp(engineId, prop, value) {
    _enqueue(() => {
        const ids = _nodeRegistry.get(engineId);
        if (!ids || ids.counter < 0) return;
        const { counter, clientId } = ids;
        const e = _engine;

        switch (prop) {
            case 'position':
                e.set_node_position(counter, clientId, value.x, value.y);
                break;
            case 'layoutPosition':
                e.set_node_layout_position(counter, clientId, value.x, value.y);
                break;
            case 'size':
                e.set_node_size(counter, clientId, value.w, value.h);
                break;
            case 'sizeConstraints':
                e.set_node_size_constraints(counter, clientId, value.minW, value.minH, value.maxW, value.maxH);
                break;
            case 'sizing':
                e.set_node_sizing(counter, clientId, value.horizontal ?? 0, value.vertical ?? 0);
                break;
            case 'margin':
                e.set_node_margin(counter, clientId, value.top, value.right, value.bottom, value.left);
                break;
            case 'fill':
                e.set_node_fill(counter, clientId, value.r, value.g, value.b, value.a);
                break;
            case 'cornerRadius':
                e.set_node_corner_radius(counter, clientId, value.tl, value.tr, value.br, value.bl);
                break;
            case 'opacity':
                e.set_node_opacity(counter, clientId, value);
                break;
            case 'text':
                e.set_node_text(counter, clientId, value);
                break;
            case 'fontSize':
                e.set_node_font_size(counter, clientId, value);
                break;
            case 'fontWeight':
                e.set_node_font_weight(counter, clientId, value);
                break;
            case 'fontFamily':
                e.set_node_font_family(counter, clientId, value);
                break;
            case 'textAlign':
                e.set_text_align(counter, clientId, value);
                break;
            case 'autoLayout':
                e.set_auto_layout(counter, clientId,
                    value.direction, value.spacing,
                    value.padTop, value.padRight, value.padBottom, value.padLeft,
                    value.align ?? 0, value.justify ?? 0, value.wrap ?? 0);
                break;
            case 'stroke':
                e.set_node_stroke(counter, clientId,
                    value.r, value.g, value.b, value.a, value.weight);
                break;
            case 'rotation':
                e.set_node_rotation(counter, clientId, value);
                break;
            case 'shadow':
                e.add_drop_shadow(counter, clientId,
                    value.r, value.g, value.b, value.a,
                    value.ox, value.oy, value.blur, value.spread);
                break;
            case 'linearGradient':
                e.set_node_linear_gradient(counter, clientId,
                    value.startX, value.startY, value.endX, value.endY,
                    value.positions, value.colors);
                break;
            case 'clipContent':
                e.set_node_clip_content(counter, clientId, !!value);
                break;
        }
    });
    _dirty = true;
}

// Hit test: screen coords → engineId or null
export function hitTest(screenX, screenY) {
    _flushOps();
    try {
        const cam = _engine.get_camera();
        const [camX, camY, zoom] = cam;
        const worldX = screenX / zoom + camX;
        const worldY = screenY / zoom + camY;

        let hit = null;
        for (const [engineId, ids] of _nodeRegistry) {
            if (ids.counter < 0) continue;
            const b = _engine.get_node_world_bounds(ids.counter, ids.clientId);
            if (b && b.length === 4) {
                const [nx, ny, nw, nh] = b;
                if (worldX >= nx && worldX <= nx + nw && worldY >= ny && worldY <= ny + nh) {
                    hit = engineId;
                }
            }
        }
        return hit;
    } catch (e) {
        return null;
    }
}

export function flushAndRender() {
    _flushOps();

    if (_dirty) {
        _dirty = false;
        _renderer.render(_engine, _ctx, _canvas);
        globalThis.__RENDERO_ON_FRAME__?.();
    }
}

function _startRenderLoop() {
    const loop = () => {
        try {
            flushAndRender();
        } catch (e) {
            const message = e?.message || String(e);
            if (_renderer?.name === 'canvas2d') {
                switchRenderer('raster', message);
            } else {
                console.warn('Render error:', message);
            }
        }
        requestAnimationFrame(loop);
    };
    loop();
}
