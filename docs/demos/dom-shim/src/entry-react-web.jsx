// ═══════════════════════════════════════════════════════════════
// React Entry — WEB mode (real browser DOM, zero shim)
// ═══════════════════════════════════════════════════════════════
import React from 'react';
import { createRoot } from 'react-dom/client';
import AppleApp from './apple-react.jsx';

const root = createRoot(document.getElementById('root'));
root.render(React.createElement(AppleApp));
