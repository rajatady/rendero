import { transformSync } from 'esbuild';
import { readFileSync, writeFileSync } from 'fs';

// Which file to compile — pass as argument or default to App.jsx
const source = process.argv[2] || 'src/App.jsx';

const jsx = readFileSync(source, 'utf8');

const result = transformSync(jsx, {
    loader: 'jsx',
    jsxFactory: 'h',
    jsxFragment: 'Fragment',
});

const code = result.code.replace(/from\s+"\.\.\/renderer\.js"/g, 'from "./renderer.js"');
writeFileSync('app.js', code);
console.log(`✓ ${source} → app.js`);
