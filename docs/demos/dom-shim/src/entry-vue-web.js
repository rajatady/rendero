// ═══════════════════════════════════════════════════════════════
// Vue Entry — WEB mode (real browser DOM, zero shim)
// ═══════════════════════════════════════════════════════════════
import { createApp } from 'vue';
import AppleApp from './apple-vue.js';

const app = createApp(AppleApp);
app.mount('#root');
