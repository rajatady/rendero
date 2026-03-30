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
    };

    g.Rendero = namespace;
    return namespace;
}
