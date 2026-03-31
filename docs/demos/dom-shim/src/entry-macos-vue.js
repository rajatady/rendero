// ═══════════════════════════════════════════════════════════════
// macOS Native Entry — DOM shim → Rendero (native Rust engine), Vue path
// ═══════════════════════════════════════════════════════════════

import { initEngine, flushAndRender as _flush } from '../shims/engine-native.js';
import { ShimDocument } from '../shims/document.js';
import { createWindowShim, installWindowScrollShim } from '../shims/window.js';
import { setViewport } from '../shims/css-values.js';
import { createApp } from 'vue';
import AppleApp from './apple-vue.js';

const vpW = (typeof __screenWidth !== 'undefined') ? __screenWidth : 1024;
const vpH = (typeof __screenHeight !== 'undefined') ? __screenHeight : 768;
const vpScale = (typeof __screenScale !== 'undefined') ? __screenScale : 1;
setViewport(vpW, vpH);

initEngine();

const shimDoc = new ShimDocument();
shimDoc._connectToEngine();
const windowShim = createWindowShim(shimDoc);
Object.assign(globalThis, windowShim, {
    innerWidth: vpW,
    innerHeight: vpH,
    outerWidth: vpW,
    outerHeight: vpH,
    devicePixelRatio: vpScale,
});
installWindowScrollShim(globalThis, shimDoc);
shimDoc.defaultView = globalThis;
globalThis.document = shimDoc;
globalThis.window.document = shimDoc;

const container = shimDoc.createElement('div');
container.id = 'root';
shimDoc.body.appendChild(container);

try {
    const app = createApp(AppleApp);
    app.mount(container);

    if (typeof __rendero_log === 'function') {
        __rendero_log('Vue app mounted via DOM shim');
    }
} catch (error) {
    if (typeof __rendero_log === 'function') {
        const message = error && error.stack ? error.stack : String(error);
        __rendero_log('Vue mount failed: ' + message);
    }
    throw error;
}

if (typeof self !== 'undefined') self.__shimFlushAndRender = _flush;
else if (typeof global !== 'undefined') global.__shimFlushAndRender = _flush;
