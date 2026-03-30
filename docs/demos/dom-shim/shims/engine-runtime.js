import * as browserEngine from './engine.js';
import * as nativeEngine from './engine-native.js';

function activeEngine() {
    return globalThis.__RENDERO_NATIVE__ ? nativeEngine : browserEngine;
}

export function initEngine(...args) {
    return activeEngine().initEngine(...args);
}

export function getEngine(...args) {
    return activeEngine().getEngine(...args);
}

export function getCanvas(...args) {
    return activeEngine().getCanvas(...args);
}

export function allocId(...args) {
    return activeEngine().allocId(...args);
}

export function markDirty(...args) {
    return activeEngine().markDirty(...args);
}

export function registerNode(...args) {
    return activeEngine().registerNode(...args);
}

export function unregisterNode(...args) {
    return activeEngine().unregisterNode(...args);
}

export function getNodeIds(...args) {
    return activeEngine().getNodeIds(...args);
}

export function setInsertParent(...args) {
    return activeEngine().setInsertParent(...args);
}

export function clearInsertParent(...args) {
    return activeEngine().clearInsertParent(...args);
}

export function engineCreateFrame(...args) {
    return activeEngine().engineCreateFrame(...args);
}

export function engineCreateText(...args) {
    return activeEngine().engineCreateText(...args);
}

export function engineDeleteNode(...args) {
    return activeEngine().engineDeleteNode(...args);
}

export function engineGetBounds(...args) {
    return activeEngine().engineGetBounds(...args);
}

export function engineSetProp(...args) {
    return activeEngine().engineSetProp(...args);
}

export function hitTest(...args) {
    return activeEngine().hitTest(...args);
}

export function flushAndRender(...args) {
    return activeEngine().flushAndRender(...args);
}

// ─── Browser-based text measurement (oracle) ───
// Uses canvas.measureText() for accurate text dimensions on web.
// On native, falls back to null (engine uses its own measurer).

let _measureCanvas = null;
let _measureCtx = null;

export function measureTextBrowser(text, fontSize, fontWeight, fontFamily) {
    if (globalThis.__RENDERO_NATIVE__) return null; // native path uses Rust measurer

    if (!_measureCanvas) {
        if (typeof document === 'undefined' || typeof document.createElement !== 'function') return null;
        _measureCanvas = document.createElement('canvas');
        _measureCtx = _measureCanvas.getContext('2d');
    }
    if (!_measureCtx) return null;

    const weight = fontWeight || '400';
    const family = fontFamily || '-apple-system, system-ui, sans-serif';
    const size = fontSize || 16;
    _measureCtx.font = `${weight} ${size}px ${family}`;

    const metrics = _measureCtx.measureText(text || ' ');
    const width = metrics.width;
    // Height: use fontBoundingBox (full line height) if available,
    // otherwise fall back to fontSize * 1.2 (standard line-height approximation).
    // actualBoundingBox is too tight — it only covers visible glyph bounds.
    let height;
    if (metrics.fontBoundingBoxAscent !== undefined && metrics.fontBoundingBoxDescent !== undefined) {
        height = metrics.fontBoundingBoxAscent + metrics.fontBoundingBoxDescent;
    } else {
        height = size * 1.2;
    }

    return { width: Math.ceil(width), height: Math.ceil(height) };
}
