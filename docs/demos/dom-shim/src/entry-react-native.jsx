// ═══════════════════════════════════════════════════════════════
// React Entry — NATIVE mode (DOM shim → Rendero engine)
// ═══════════════════════════════════════════════════════════════
//
// This is the magic: we install the DOM shim, then hand the
// shim's document to react-dom. React thinks it's a browser.
// Every DOM call routes to the Rendero WASM engine.

import { installShim } from '../shims/index.js';
import React from 'react';
import { createRoot } from 'react-dom/client';
import AppleApp from './apple-react.jsx';

async function main() {
    const canvas = document.getElementById('canvas');
    const shim = await installShim(canvas);

    // Get a container element from the shim document
    const container = shim.getContainer();

    // react-dom's createRoot accepts any DOM node.
    // It doesn't check if it's a "real" browser node.
    // Our ShimElement quacks like a duck → it works.
    const root = createRoot(container);
    root.render(React.createElement(AppleApp));

    document.getElementById('mode-label').textContent = 'React + DOM Shim → Rendero';
}

main();
