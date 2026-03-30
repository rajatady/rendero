// ═══════════════════════════════════════════════════════════════
// macOS Native Entry — DOM shim → Rendero (native Rust engine)
// ═══════════════════════════════════════════════════════════════
//
// This runs in JavaScriptCore inside the macOS app.
// No browser, no canvas, no WASM. Same React app, same DOM shim.
//
// Swift pre-registers __rendero_* functions before evaluating this.

// ═══════════════════════════════════════════════════════════════
// macOS Native Entry — DOM shim → Rendero (native Rust engine)
// ═══════════════════════════════════════════════════════════════
//
// This runs in JavaScriptCore inside the macOS app.
// No browser, no canvas, no WASM. Same React app, same DOM shim.
//
// Swift pre-registers __rendero_* functions before evaluating this.
//
// We import the shim components directly (not through index.js)
// to avoid pulling in the WASM engine.js imports.

import { initEngine } from '../shims/engine-native.js';
import { ShimDocument } from '../shims/document.js';
import { createWindowShim } from '../shims/window.js';
import { setViewport } from '../shims/css-values.js';
import React from 'react';
import { createRoot } from 'react-dom/client';
import AppleApp from './apple-react.jsx';

// Set viewport from Swift-provided screen dimensions
const vpW = (typeof __screenWidth !== 'undefined') ? __screenWidth : 1024;
const vpH = (typeof __screenHeight !== 'undefined') ? __screenHeight : 768;
setViewport(vpW, vpH);

// Init the native engine bridge
initEngine();

// Create the shim document
const shimDoc = new ShimDocument();
shimDoc._connectToEngine();
const windowShim = createWindowShim(shimDoc);
Object.assign(globalThis, windowShim, {
    innerWidth: vpW,
    innerHeight: vpH,
    outerWidth: vpW,
    outerHeight: vpH,
    devicePixelRatio: 1,
});

// Use globalThis as defaultView — it has our polyfilled HTMLIFrameElement etc.
// React's getActiveElementDeep does `element instanceof containerInfo.HTMLIFrameElement`
// where containerInfo = container.ownerDocument.defaultView.
shimDoc.defaultView = globalThis;
globalThis.document = shimDoc;
globalThis.window.document = shimDoc;

// Create root container
const container = shimDoc.createElement('div');
container.id = 'root';
shimDoc.body.appendChild(container);

// Mount React
const root = createRoot(container);
root.render(React.createElement(AppleApp));

// Tell Swift we're ready
if (typeof __rendero_log === 'function') {
    __rendero_log('React app mounted via DOM shim');
}

// Export flush function for Swift's display link.
import { flushAndRender as _flush } from '../shims/engine-native.js';
if (typeof self !== 'undefined') self.__shimFlushAndRender = _flush;
else if (typeof global !== 'undefined') global.__shimFlushAndRender = _flush;
