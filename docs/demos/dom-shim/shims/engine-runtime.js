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

let _measureElementRoot = null;

function ensureMeasureElementRoot() {
    if (globalThis.__RENDERO_NATIVE__) return null;
    if (_measureElementRoot) return _measureElementRoot;
    if (typeof document === 'undefined' || !document.body || typeof document.createElement !== 'function') return null;

    const root = document.createElement('div');
    root.style.position = 'absolute';
    root.style.left = '-100000px';
    root.style.top = '0';
    root.style.visibility = 'hidden';
    root.style.pointerEvents = 'none';
    root.style.contain = 'layout style paint';
    root.style.zIndex = '-1';
    document.body.appendChild(root);
    _measureElementRoot = root;
    return root;
}

export function measureTextElementBrowser(tagName, text, styles = {}) {
    if (globalThis.__RENDERO_NATIVE__) return null;

    const root = ensureMeasureElementRoot();
    if (!root) return null;

    const el = document.createElement(tagName || 'div');
    el.textContent = text || ' ';
    el.style.margin = '0';
    el.style.padding = '0';
    el.style.border = '0';
    el.style.boxSizing = 'border-box';
    el.style.position = 'static';
    el.style.display = styles.display || 'inline-block';
    el.style.whiteSpace = styles.whiteSpace || 'normal';
    el.style.font = 'inherit';

    const forwarded = [
        'fontFamily',
        'fontSize',
        'fontWeight',
        'fontStyle',
        'fontStretch',
        'fontVariant',
        'lineHeight',
        'letterSpacing',
        'wordSpacing',
        'textTransform',
        'textIndent',
        'textAlign',
        'width',
        'minWidth',
        'maxWidth',
        'height',
        'minHeight',
        'maxHeight',
        'padding',
        'paddingTop',
        'paddingRight',
        'paddingBottom',
        'paddingLeft',
        'overflowWrap',
        'wordBreak',
        'whiteSpace',
    ];

    for (const key of forwarded) {
        if (styles[key] !== undefined && styles[key] !== null && styles[key] !== '') {
            el.style[key] = String(styles[key]);
        }
    }

    root.appendChild(el);
    const rect = el.getBoundingClientRect();
    root.removeChild(el);

    if (!(rect.width > 0) && !(rect.height > 0)) {
        return null;
    }

    return {
        width: Math.ceil(rect.width),
        height: Math.ceil(rect.height),
    };
}
