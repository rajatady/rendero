import { build } from 'esbuild';

// The WASM module lives at docs/pkg/rendero.js
// From dist/ (where bundles land), that's ../../../pkg/rendero.js
// We alias the source-level import path to the correct output-relative path.
const renderoPlugin = {
    name: 'rendero-external',
    setup(build) {
        build.onResolve({ filter: /rendero\.js$/ }, () => ({
            path: '../../../pkg/rendero.js',
            external: true,
        }));
    },
};

const common = {
    bundle: true,
    format: 'esm',
    target: 'es2020',
    minify: false,
    sourcemap: true,
    plugins: [renderoPlugin],
    define: {
        __VUE_OPTIONS_API__: 'true',
        __VUE_PROD_DEVTOOLS__: 'false',
        __VUE_PROD_HYDRATION_MISMATCH_DETAILS__: 'false',
    },
};

// Native bundle config — no external WASM, self-contained for JavaScriptCore
const nativeCommon = {
    bundle: true,
    format: 'iife',       // IIFE for JavaScriptCore (no ES module support)
    target: 'es2020',
    minify: false,
    sourcemap: false,
    // engine-native.js uses globals (__rendero_*), no WASM import needed
    define: {
        __VUE_OPTIONS_API__: 'true',
        __VUE_PROD_DEVTOOLS__: 'false',
        __VUE_PROD_HYDRATION_MISMATCH_DETAILS__: 'false',
    },
};

// Build all 5 entry points
await Promise.all([
    build({
        ...common,
        entryPoints: ['src/entry-react-web.jsx'],
        outfile: 'dist/react-web.js',
        jsx: 'automatic',
    }),
    build({
        ...common,
        entryPoints: ['src/entry-react-native.jsx'],
        outfile: 'dist/react-native.js',
        jsx: 'automatic',
    }),
    build({
        ...common,
        entryPoints: ['src/entry-vue-web.js'],
        outfile: 'dist/vue-web.js',
    }),
    build({
        ...common,
        entryPoints: ['src/entry-vue-native.js'],
        outfile: 'dist/vue-native.js',
    }),
    build({
        ...nativeCommon,
        entryPoints: ['src/entry-macos.jsx'],
        outfile: 'dist/macos-react-bundle.js',
        jsx: 'automatic',
    }),
    build({
        ...nativeCommon,
        entryPoints: ['src/entry-macos-vue.js'],
        outfile: 'dist/macos-vue-bundle.js',
        jsx: 'automatic',
    }),
    // Legacy compatibility path: default macOS bundle remains React.
    build({
        ...nativeCommon,
        entryPoints: ['src/entry-macos.jsx'],
        outfile: 'dist/macos-bundle.js',
        jsx: 'automatic',
    }),
]);

console.log('✓ Built browser/native bundles for React and Vue → dist/');
