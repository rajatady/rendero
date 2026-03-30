// ═══════════════════════════════════════════════════════════════
// DOM Shim — Barrel Export
// ═══════════════════════════════════════════════════════════════
//
// Usage:
//   import { installShim } from './shims/index.js';
//
//   // Mode 1: Full shim (native — replaces document/window)
//   const { document, window } = await installShim(canvasElement);
//
//   // Mode 2: Web mode (no shim — returns real document/window)
//   // Just don't call installShim. Use the browser normally.
//
// The shim intercepts ALL DOM operations and routes them to
// the Rendero WASM engine. react-dom / Vue don't know the
// difference — they think they're in a browser.

import { initEngine, hitTest, markDirty } from './engine.js';
import { ShimDocument } from './document.js';
import { ShimElement } from './element.js';
import { ShimTextNode } from './text-node.js';
import { ShimEvent, ShimMouseEvent, ShimKeyboardEvent } from './events.js';
import { createWindowShim, installWindowScrollShim } from './window.js';

async function loadBrowserEngineModule() {
    const version = globalThis.__RENDERO_WASM_ASSET_VERSION__;
    const baseUrl = globalThis.__RENDERO_WASM_MODULE_URL__ || '/pkg/rendero.js';
    const suffix = version ? `${baseUrl.includes('?') ? '&' : '?'}v=${encodeURIComponent(version)}` : '';
    return import(`${baseUrl}${suffix}`);
}

function versionedAssetUrl(baseUrl) {
    const version = globalThis.__RENDERO_WASM_ASSET_VERSION__;
    if (!version) return baseUrl;
    return `${baseUrl}${baseUrl.includes('?') ? '&' : '?'}v=${encodeURIComponent(version)}`;
}

export async function installShim(canvas) {
    // Init WASM engine
    const { default: init, CanvasEngine } = await loadBrowserEngineModule();
    const wasmUrl = versionedAssetUrl(globalThis.__RENDERO_WASM_BINARY_URL__ || '/pkg/rendero_bg.wasm');
    await init(wasmUrl);
    const engine = new CanvasEngine('DOMShim', 1);
    initEngine(engine, canvas);

    // Create shim document
    const shimDoc = new ShimDocument();
    shimDoc._connectToEngine();

    // In browser: use the real window as defaultView so react-dom's
    // instanceof checks (HTMLIFrameElement etc.) work.
    // On native (no browser): use our standalone window shim.
    const isInBrowser = typeof window !== 'undefined' && typeof window.HTMLElement !== 'undefined';
    const windowRef = isInBrowser ? window : createWindowShim(shimDoc);
    shimDoc.defaultView = windowRef;
    installWindowScrollShim(windowRef, shimDoc, { patchRealWindow: isInBrowser });
    globalThis.__RENDERO_ON_FRAME__ = () => {
        windowRef.__renderoUpdateScrollMetrics?.();
    };

    // Wire up real canvas events → shim event dispatch
    _wireCanvasEvents(canvas, shimDoc);

    if (typeof globalThis !== 'undefined') {
        globalThis.__RENDERO_DEBUG__ = {
            engine,
            shimDocument: shimDoc,
            canvas,
        };
    }

    return {
        document: shimDoc,
        window: windowRef,
        engine,
        // Helper to get the root container for React/Vue
        getContainer() {
            // Create a root div inside body (like <div id="root">)
            const root = shimDoc.createElement('div');
            root.id = 'root';
            shimDoc.body.appendChild(root);
            return root;
        },
    };
}

// ─── Native mode (JavaScriptCore, no browser) ───
// Called from entry-macos.jsx. No canvas, no WASM.
// Engine already created by Swift. __rendero_* functions pre-registered.

export function installShimNative() {
    // Import native engine bridge (bundled by esbuild)
    // The native engine functions are global (__rendero_*)
    const { initEngine: initNativeEngine } = require('./engine-native.js');
    initNativeEngine();

    const shimDoc = new ShimDocument();
    shimDoc._connectToEngine();

    // Use our standalone window shim (no browser window)
    const windowShim = createWindowShim(shimDoc);
    shimDoc.defaultView = windowShim;
    installWindowScrollShim(windowShim, shimDoc, { patchRealWindow: false });
    globalThis.__RENDERO_ON_FRAME__ = () => {
        windowShim.__renderoUpdateScrollMetrics?.();
    };

    return {
        document: shimDoc,
        window: windowShim,
        getContainer() {
            const root = shimDoc.createElement('div');
            root.id = 'root';
            shimDoc.body.appendChild(root);
            return root;
        },
    };
}

// Re-export for direct use
export { ShimDocument } from './document.js';
export { ShimElement } from './element.js';
export { ShimTextNode } from './text-node.js';
export { ShimEvent, ShimMouseEvent } from './events.js';

function _wireCanvasEvents(canvas, shimDoc) {
    // Mouse click → find hit element → dispatch click event
    canvas.addEventListener('mousedown', (e) => {
        const hitId = hitTest(e.clientX, e.clientY);
        if (hitId) {
            const target = _findElementByEngineId(shimDoc.body, hitId);
            if (target) {
                const shimEvent = new ShimMouseEvent('click', {
                    clientX: e.clientX,
                    clientY: e.clientY,
                    button: e.button,
                    bubbles: true,
                });
                target.dispatchEvent(shimEvent);
            }
        }
    });

    // Mouse move → mouseover/mouseout
    let lastHover = null;
    canvas.addEventListener('mousemove', (e) => {
        const hitId = hitTest(e.clientX, e.clientY);
        const target = hitId ? _findElementByEngineId(shimDoc.body, hitId) : null;

        if (lastHover !== target) {
            if (lastHover) {
                lastHover.dispatchEvent(new ShimMouseEvent('mouseout', { clientX: e.clientX, clientY: e.clientY, bubbles: true }));
                lastHover.dispatchEvent(new ShimMouseEvent('mouseleave', { clientX: e.clientX, clientY: e.clientY, bubbles: false }));
            }
            if (target) {
                target.dispatchEvent(new ShimMouseEvent('mouseover', { clientX: e.clientX, clientY: e.clientY, bubbles: true }));
                target.dispatchEvent(new ShimMouseEvent('mouseenter', { clientX: e.clientX, clientY: e.clientY, bubbles: false }));
            }
            lastHover = target;
        }
    });

    // Touch → click (mobile)
    let touchStartY = 0;
    let touchMoved = false;
    canvas.addEventListener('touchstart', (e) => {
        if (e.touches.length === 1) {
            touchStartY = e.touches[0].clientY;
            touchMoved = false;
        }
    }, { passive: true });

    canvas.addEventListener('touchmove', (e) => {
        if (e.touches.length === 1 && Math.abs(e.touches[0].clientY - touchStartY) > 5) {
            touchMoved = true;
        }
    }, { passive: true });

    canvas.addEventListener('touchend', (e) => {
        if (!touchMoved && e.changedTouches.length === 1) {
            const t = e.changedTouches[0];
            const hitId = hitTest(t.clientX, t.clientY);
            if (hitId) {
                const target = _findElementByEngineId(shimDoc.body, hitId);
                if (target) {
                    target.dispatchEvent(new ShimMouseEvent('click', {
                        clientX: t.clientX, clientY: t.clientY, bubbles: true,
                    }));
                }
            }
        }
    });

    // Scroll
    canvas.addEventListener('wheel', (e) => {
        e.preventDefault();
        if (typeof window !== 'undefined' && typeof window.scrollBy === 'function') {
            window.scrollBy(0, e.deltaY);
        }
        window.__renderoUpdateScrollMetrics?.();
        window.__renderoSyncScrollFromViewport?.();
        markDirty();
    }, { passive: false });
}

// Walk the shim tree to find element by engine ID
function _findElementByEngineId(root, engineId) {
    if (root._engineId === engineId) return root;
    for (const child of root.childNodes) {
        if (child.nodeType === 1 || child.nodeType === 3) {
            const found = _findElementByEngineId(child, engineId);
            if (found) return found;
        }
    }
    return null;
}
