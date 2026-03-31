// ═══════════════════════════════════════════════════════════════
// Engine Bridge — NATIVE platform (JavaScriptCore + Rust FFI)
// ═══════════════════════════════════════════════════════════════
//
// Drop-in replacement for engine.js. Same exports, same API.
// Instead of calling WASM, calls native C functions exposed by
// Swift via JSContext. The Swift host registers these as global
// functions before loading the JS bundle:
//
//   __rendero_create(name, clientId) → enginePtr
//   __rendero_add_frame(name, x, y, w, h, r, g, b, a) → packedId
//   __rendero_set_node_fill(counter, clientId, r, g, b, a)
//   __rendero_render_pixels(width, height) → base64 RGBA
//   ... etc.
//
// The Swift side holds the engine pointer and wraps each C call.

import { ensureRenderoNamespace, recordNodeRegistration, recordNodeUnregistration, setDebugSurface, traceBridgeOp } from './rendero-api.js';

let _nextId = 1;
let _dirty = true;

// Map from shimNode._engineId → { counter, clientId }
const _nodeRegistry = new Map();

// Operation queue — flushed before render
const _ops = [];

// Render callback — set by Swift when display link fires
let _renderCallback = null;

function _legacyBridge() {
    return {
        dispatch(cmd, args = []) {
            switch (cmd) {
                case 'set_viewport': return __rendero_set_viewport(args[0], args[1]);
                case 'set_camera': return __rendero_set_camera(args[0], args[1], args[2]);
                case 'set_insert_parent': return __rendero_set_insert_parent(args[0], args[1]);
                case 'clear_insert_parent': return __rendero_clear_insert_parent();
                case 'set_node_position': return __rendero_set_node_position(args[0], args[1], args[2], args[3]);
                case 'set_node_layout_position': return __rendero_set_node_layout_position(args[0], args[1], args[2], args[3]);
                case 'set_node_clip_content': return __rendero_set_node_clip_content(args[0], args[1], args[2]);
                case 'set_node_size': return __rendero_set_node_size(args[0], args[1], args[2], args[3]);
                case 'set_node_size_percent': return __rendero_set_node_size_percent(args[0], args[1], args[2], args[3]);
                case 'set_node_size_constraints': return __rendero_set_node_size_constraints(args[0], args[1], args[2], args[3], args[4], args[5]);
                case 'set_node_sizing': return __rendero_set_node_sizing(args[0], args[1], args[2], args[3]);
                case 'set_node_margin': return __rendero_set_node_margin(args[0], args[1], args[2], args[3], args[4], args[5], args[6], args[7], args[8], args[9]);
                case 'set_node_fill': return __rendero_set_node_fill(args[0], args[1], args[2], args[3], args[4], args[5]);
                case 'set_node_corner_radius': return __rendero_set_node_corner_radius(args[0], args[1], args[2], args[3], args[4], args[5]);
                case 'set_node_opacity': return __rendero_set_node_opacity(args[0], args[1], args[2]);
                case 'set_node_font_size': return __rendero_set_node_font_size(args[0], args[1], args[2]);
                case 'set_node_font_weight': return __rendero_set_node_font_weight(args[0], args[1], args[2]);
                case 'set_node_letter_spacing': return __rendero_set_node_letter_spacing(args[0], args[1], args[2]);
                case 'set_node_line_height': return __rendero_set_node_line_height(args[0], args[1], args[2]);
                case 'set_node_stroke': return __rendero_set_node_stroke(args[0], args[1], args[2], args[3], args[4], args[5], args[6]);
                case 'set_node_rotation': return __rendero_set_node_rotation(args[0], args[1], args[2]);
                case 'add_drop_shadow': return __rendero_add_drop_shadow(args[0], args[1], args[2], args[3], args[4], args[5], args[6], args[7], args[8], args[9]);
                case 'set_node_linear_gradient': return __rendero_set_node_linear_gradient(args[0], args[1], args[2], args[3], args[4], args[5], args[6], ...args.slice(7));
                case 'set_auto_layout': return __rendero_set_auto_layout(args[0], args[1], args[2], args[3], args[4], args[5], args[6], args[7], args[8], args[9]);
                case 'select_node': return __rendero_select_node(args[0], args[1]);
                case 'delete_selected': return __rendero_delete_selected();
                case 'request_render': return __rendero_request_render?.();
                default: return undefined;
            }
        },
        dispatchStr(cmd, args = [], text = '') {
            switch (cmd) {
                case 'add_frame_named': return __rendero_add_frame(text, args[0], args[1], args[2], args[3], args[4], args[5], args[6], args[7]);
                case 'add_text_named': {
                    const splitAt = text.indexOf('\0');
                    const name = splitAt >= 0 ? text.slice(0, splitAt) : text;
                    const value = splitAt >= 0 ? text.slice(splitAt + 1) : '';
                    return __rendero_add_text(name, value, args[0], args[1], args[2], args[3], args[4], args[5], args[6]);
                }
                case 'set_node_text': return __rendero_set_node_text(args[0], args[1], text);
                case 'set_node_font_family': return __rendero_set_node_font_family(args[0], args[1], text);
                case 'set_text_align': return __rendero_set_text_align(args[0], args[1], text);
                default: return undefined;
            }
        },
        getCamera() {
            return typeof __rendero_get_camera === 'function' ? __rendero_get_camera() : { x: 0, y: 0, zoom: 1 };
        },
        getNodeBounds(counter, clientId) {
            return typeof __rendero_get_node_bounds === 'function'
                ? __rendero_get_node_bounds(counter, clientId)
                : { x: 0, y: 0, w: 0, h: 0 };
        },
        log(message) {
            if (typeof __rendero_log === 'function') __rendero_log(String(message));
        },
        requestRender() {
            if (typeof __rendero_request_render === 'function') __rendero_request_render();
        },
        screen: {
            width: typeof __screenWidth !== 'undefined' ? __screenWidth : 1280,
            height: typeof __screenHeight !== 'undefined' ? __screenHeight : 800,
            scale: typeof __screenScale !== 'undefined' ? __screenScale : 2,
        },
    };
}

function _bridge() {
    return globalThis.__RenderoHostBridge || _legacyBridge();
}

export function initEngine() {
    globalThis.__RENDERO_NATIVE__ = true;
    globalThis.Rendero?.debug?.reset?.();
    setDebugSurface('rendero_native');
    // On native, the engine is already created by Swift.
    // We just need to set up the render loop callback.
    const bridge = _bridge();
    const { width: w, height: h, scale: dpr } = bridge.screen;
    bridge.dispatch('set_viewport', [w * dpr, h * dpr]);
    bridge.dispatch('set_camera', [0, 0, dpr]);

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
        nativeApi: globalThis.Rendero?.native || {
            storage: {
                get: (key) => globalThis.Rendero?.native?.storage?.get?.(key) ?? null,
                set: (key, value) => globalThis.Rendero?.native?.storage?.set?.(key, value) ?? true,
            },
            clipboard: {
                readText: () => globalThis.Rendero?.native?.clipboard?.readText?.() ?? null,
                writeText: (text) => globalThis.Rendero?.native?.clipboard?.writeText?.(text) ?? true,
            },
            dialogs: {
                alert: (message) => globalThis.Rendero?.native?.dialogs?.alert?.(message) ?? true,
            },
            notifications: {
                requestPermission: () => globalThis.Rendero?.native?.notifications?.requestPermission?.() ?? false,
            },
            haptics: {
                impact: (style) => globalThis.Rendero?.native?.haptics?.impact?.(style) ?? true,
            },
            media: {
                pickImage: () => globalThis.Rendero?.native?.media?.pickImage?.() ?? null,
            },
        },
    });
}

export function getEngine() { return null; } // no JS engine object on native
export function getCanvas() { return null; } // no canvas on native

export function allocId() { return _nextId++; }

export function markDirty() { _dirty = true; }
export function getCamera() {
    const cam = _bridge().getCamera();
    return { x: cam.x || 0, y: cam.y || 0, zoom: cam.zoom || 1 };
}
export function setCamera(x, y, zoom) {
    const current = getCamera();
    _bridge().dispatch('set_camera', [x, y, zoom ?? current.zoom]);
    _dirty = true;
}

export function registerNode(engineId, counter, clientId) {
    _nodeRegistry.set(engineId, { counter, clientId });
    recordNodeRegistration(engineId, counter, clientId);
}

export function unregisterNode(engineId) {
    _nodeRegistry.delete(engineId);
    recordNodeUnregistration(engineId);
}

export function getNodeIds(engineId) {
    return _nodeRegistry.get(engineId) || null;
}

// ─── Engine operations via native FFI ───

function _enqueue(fn) {
    _ops.push(fn);
    _dirty = true;
}

function _flushOps() {
    if (_ops.length === 0) return;
    _bridge().log('FLUSH: ' + _ops.length + ' ops, ' + _nodeRegistry.size + ' nodes registered');
    const batch = _ops.splice(0);
    for (const fn of batch) {
        try { fn(); } catch (e) {
            _bridge().log('Engine op error: ' + e.message);
        }
    }
}

export function setInsertParent(engineId) {
    const ids = _nodeRegistry.get(engineId);
    if (ids && ids.counter >= 0) {
        traceBridgeOp({ kind: 'setInsertParent', engineId, counter: ids.counter, clientId: ids.clientId });
        _bridge().dispatch('set_insert_parent', [ids.counter, ids.clientId]);
    }
}

export function clearInsertParent() {
    traceBridgeOp({ kind: 'clearInsertParent' });
    _bridge().dispatch('clear_insert_parent');
}

export function engineCreateFrame(engineId, name) {
    try {
        const packed = _bridge().dispatchStr('add_frame_named', [0, 0, 0, 0, 0, 0, 0, 0], name);
        const counter = Math.floor(packed / 0x100000000);
        const clientId = packed & 0xFFFFFFFF;
        registerNode(engineId, counter, clientId);
        traceBridgeOp({ kind: 'createFrame', engineId, counter, clientId, name });
        _bridge().dispatch('set_node_clip_content', [counter, clientId, 0]);
        _dirty = true;
        return { counter, clientId };
    } catch (e) {
        _bridge().log('engineCreateFrame failed: ' + name + ' ' + e.message);
        return null;
    }
}

export function engineCreateText(engineId, name, text) {
    try {
        const safeText = text || ' ';
        const packed = _bridge().dispatchStr('add_text_named', [0, 0, 16, 0, 0, 0, 1], name + '\0' + safeText);
        const counter = Math.floor(packed / 0x100000000);
        const clientId = packed & 0xFFFFFFFF;
        registerNode(engineId, counter, clientId);
        traceBridgeOp({ kind: 'createText', engineId, counter, clientId, name, text: safeText });
        _dirty = true;
        return { counter, clientId };
    } catch (e) {
        _bridge().log('engineCreateText failed: ' + name + ' ' + e.message);
        return null;
    }
}

export function engineDeleteNode(engineId) {
    const ids = _nodeRegistry.get(engineId);
    if (!ids) return;
    unregisterNode(engineId);
    traceBridgeOp({ kind: 'deleteNode', engineId, counter: ids.counter, clientId: ids.clientId });
    _enqueue(() => {
        try {
            _bridge().dispatch('select_node', [ids.counter, ids.clientId]);
            _bridge().dispatch('delete_selected', []);
        } catch (e) {}
    });
    _dirty = true;
}

export function engineGetBounds(engineId) {
    _flushOps();
    const ids = _nodeRegistry.get(engineId);
    if (!ids || ids.counter < 0) return { x: 0, y: 0, width: 0, height: 0 };
    try {
        const b = _bridge().getNodeBounds(ids.counter, ids.clientId);
        return { x: b.x || 0, y: b.y || 0, width: b.w || 0, height: b.h || 0 };
    } catch (e) {
        return { x: 0, y: 0, width: 0, height: 0 };
    }
}

export function engineSetProp(engineId, prop, value) {
    const ids = _nodeRegistry.get(engineId);
    traceBridgeOp({
        kind: 'setProp',
        engineId,
        counter: ids?.counter ?? null,
        clientId: ids?.clientId ?? null,
        prop,
        value,
    });
    _enqueue(() => {
        const ids = _nodeRegistry.get(engineId);
        if (!ids || ids.counter < 0) return;
        const { counter, clientId } = ids;
        const bridge = _bridge();

        switch (prop) {
            case 'position':
                bridge.dispatch('set_node_position', [counter, clientId, value.x, value.y]);
                break;
            case 'layoutPosition':
                bridge.dispatch('set_node_layout_position', [counter, clientId, value.x, value.y]);
                break;
            case 'size':
                bridge.dispatch('set_node_size', [counter, clientId, value.w, value.h]);
                break;
            case 'sizeConstraints':
                bridge.dispatch('set_node_size_constraints', [counter, clientId, value.minW, value.minH, value.maxW, value.maxH]);
                break;
            case 'sizing':
                bridge.dispatch('set_node_sizing', [counter, clientId, value.horizontal ?? 0, value.vertical ?? 0]);
                break;
            case 'margin':
                bridge.dispatch('set_node_margin', [counter, clientId, value.top, value.right, value.bottom, value.left]);
                break;
            case 'fill':
                bridge.dispatch('set_node_fill', [counter, clientId, value.r, value.g, value.b, value.a]);
                break;
            case 'cornerRadius':
                bridge.dispatch('set_node_corner_radius', [counter, clientId, value.tl, value.tr, value.br, value.bl]);
                break;
            case 'opacity':
                bridge.dispatch('set_node_opacity', [counter, clientId, value]);
                break;
            case 'text':
                bridge.dispatchStr('set_node_text', [counter, clientId], value);
                break;
            case 'fontSize':
                bridge.dispatch('set_node_font_size', [counter, clientId, value]);
                break;
            case 'fontWeight':
                bridge.dispatch('set_node_font_weight', [counter, clientId, value]);
                break;
            case 'fontFamily':
                bridge.dispatchStr('set_node_font_family', [counter, clientId], value);
                break;
            case 'letterSpacing':
                bridge.dispatch('set_node_letter_spacing', [counter, clientId, value]);
                break;
            case 'lineHeight':
                bridge.dispatch('set_node_line_height', [counter, clientId, value]);
                break;
            case 'textAlign':
                bridge.dispatchStr('set_text_align', [counter, clientId], value);
                break;
            case 'autoLayout':
                bridge.dispatch('set_auto_layout',
                    [counter, clientId, value.direction, value.spacing, value.padTop, value.padRight, value.padBottom, value.padLeft, value.align ?? 0, value.justify ?? 0, value.wrap ?? 0]);
                break;
            case 'stroke':
                bridge.dispatch('set_node_stroke', [counter, clientId, value.r, value.g, value.b, value.a, value.weight]);
                break;
            case 'rotation':
                bridge.dispatch('set_node_rotation', [counter, clientId, value]);
                break;
            case 'shadow':
                bridge.dispatch('add_drop_shadow', [counter, clientId, value.r, value.g, value.b, value.a, value.ox, value.oy, value.blur, value.spread]);
                break;
            case 'linearGradient': {
                const positions = Array.from(value.positions || []);
                const colors = Array.from(value.colors || []);
                bridge.dispatch('set_node_linear_gradient', [
                    counter, clientId,
                    value.startX, value.startY, value.endX, value.endY,
                    positions.length,
                    ...positions,
                    ...colors,
                ]);
                break;
            }
            case 'clipContent':
                bridge.dispatch('set_node_clip_content', [counter, clientId, value ? 1 : 0]);
                break;
            default:
                break;
        }
    });
    _dirty = true;
}

export function hitTest(screenX, screenY) {
    _flushOps();
    try {
        const cam = _bridge().getCamera();
        const camX = cam.x || 0, camY = cam.y || 0, zoom = cam.zoom || 1;
        const worldX = screenX / zoom + camX;
        const worldY = screenY / zoom + camY;

        let hit = null;
        for (const [engineId, ids] of _nodeRegistry) {
            if (ids.counter < 0) continue;
            const b = _bridge().getNodeBounds(ids.counter, ids.clientId);
            if (b) {
                const nx = b.x || 0, ny = b.y || 0, nw = b.w || 0, nh = b.h || 0;
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

// Called by Swift's display link to trigger rendering
let _flushCount = 0;
export function flushAndRender() {
    _flushCount++;
    _flushOps();
    if (_dirty) {
        _dirty = false;
        if (_flushCount <= 3) _bridge().log('RENDER frame=' + _flushCount + ' nodes=' + _nodeRegistry.size);
        _bridge().requestRender();
    }
}
