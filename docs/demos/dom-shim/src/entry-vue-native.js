// ═══════════════════════════════════════════════════════════════
// Vue Entry — NATIVE mode (DOM shim → Rendero engine)
// ═══════════════════════════════════════════════════════════════

import { installShim } from '../shims/index.js';
import { createApp } from 'vue';
import AppleApp from './apple-vue.js';

async function main() {
    const canvas = document.getElementById('canvas');
    const shim = await installShim(canvas);

    // Vue's createApp(...).mount() needs a DOM node or selector.
    // Our shim container IS a DOM node (ShimElement).
    const container = shim.getContainer();
    const app = createApp(AppleApp);
    app.mount(container);

    document.getElementById('mode-label').textContent = 'Vue + DOM Shim → Rendero';
}

main();
