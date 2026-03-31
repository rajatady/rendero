// Shared Rendero JS namespace.
//
// This is the app-visible surface for engine access and future platform APIs.
// Legacy __rendero_* globals remain bridge internals only.

function notImplemented(domain, method) {
    return () => {
        const message = `Rendero.${domain}.${method} is not implemented in this host`;
        if (typeof console !== 'undefined' && console.warn) console.warn(message);
        return null;
    };
}

const DEBUG_STATE_KEY = '__RENDERO_DEBUG_STATE__';
const LEGACY_DEBUG_KEY = '__RENDERO_DEBUG__';

function syncLegacyDebugState() {
    const g = globalThis;
    const legacy = g[LEGACY_DEBUG_KEY];
    if (!legacy || typeof legacy !== 'object') return;
    legacy.layered = g[DEBUG_STATE_KEY] || null;
    legacy.getLayeredState = () => g[DEBUG_STATE_KEY] || null;
}

function getDebugState() {
    const g = globalThis;
    if (!g[DEBUG_STATE_KEY]) {
        g[DEBUG_STATE_KEY] = {
            surface: null,
            bridgeOps: [],
            nodes: {},
            styles: {},
        };
    }
    syncLegacyDebugState();
    return g[DEBUG_STATE_KEY];
}

function cloneDebugState() {
    return JSON.parse(JSON.stringify(getDebugState()));
}

export function setDebugSurface(surface) {
    const state = getDebugState();
    state.surface = surface;
    syncLegacyDebugState();
}

export function traceBridgeOp(op) {
    const state = getDebugState();
    state.bridgeOps.push({
        index: state.bridgeOps.length,
        ...op,
    });
    syncLegacyDebugState();
}

export function recordNodeRegistration(engineId, counter, clientId) {
    getDebugState().nodes[String(engineId)] = { engineId, counter, clientId };
    syncLegacyDebugState();
}

export function recordNodeUnregistration(engineId) {
    delete getDebugState().nodes[String(engineId)];
    delete getDebugState().styles[String(engineId)];
    syncLegacyDebugState();
}

export function recordShimStyle(engineId, snapshot) {
    if (engineId == null) return;
    getDebugState().styles[String(engineId)] = snapshot;
    syncLegacyDebugState();
}

export function resetDebugState() {
    globalThis[DEBUG_STATE_KEY] = {
        surface: null,
        bridgeOps: [],
        nodes: {},
        styles: {},
    };
    syncLegacyDebugState();
}

export function ensureRenderoNamespace({ engineBridge = null, nativeApi = null } = {}) {
    const g = globalThis;
    const existing = g.Rendero || {};

    const namespace = {
        ...existing,
        engine: {
            ...(existing.engine || {}),
            allocId: () => engineBridge?.allocId?.() ?? 0,
            markDirty: () => engineBridge?.markDirty?.(),
            hitTest: (x, y) => engineBridge?.hitTest?.(x, y) ?? null,
            getNodeIds: (engineId) => engineBridge?.getNodeIds?.(engineId) ?? null,
            getNodeBounds: (engineId) => engineBridge?.engineGetBounds?.(engineId) ?? { x: 0, y: 0, width: 0, height: 0 },
            getCamera: () => engineBridge?.getCamera?.() ?? { x: 0, y: 0, zoom: 1 },
            setCamera: (x, y, zoom) => engineBridge?.setCamera?.(x, y, zoom) ?? null,
        },
        native: {
            storage: nativeApi?.storage || existing.native?.storage || {
                get: notImplemented('native.storage', 'get'),
                set: notImplemented('native.storage', 'set'),
            },
            clipboard: nativeApi?.clipboard || existing.native?.clipboard || {
                readText: notImplemented('native.clipboard', 'readText'),
                writeText: notImplemented('native.clipboard', 'writeText'),
            },
            dialogs: nativeApi?.dialogs || existing.native?.dialogs || {
                alert: notImplemented('native.dialogs', 'alert'),
            },
            notifications: nativeApi?.notifications || existing.native?.notifications || {
                requestPermission: notImplemented('native.notifications', 'requestPermission'),
            },
            haptics: nativeApi?.haptics || existing.native?.haptics || {
                impact: notImplemented('native.haptics', 'impact'),
            },
            media: nativeApi?.media || existing.native?.media || {
                pickImage: notImplemented('native.media', 'pickImage'),
            },
        },
        debug: {
            getSnapshot: () => cloneDebugState(),
            reset: () => resetDebugState(),
        },
    };

    g.Rendero = namespace;
    return namespace;
}
