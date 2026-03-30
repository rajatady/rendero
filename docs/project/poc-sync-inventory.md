# `poc/rendering-dom` Sync Inventory

This document inventories every file changed on `poc/rendering-dom` after
commit `a8da30d` so the branch can be synced by file content rather than by
replaying the other branch's git history.

Source range:

- base: `a8da30d` — `Add current-status.md — blocking issues, runnable path, bugs found`
- tip: `poc/rendering-dom`

Commits in range:

1. `3df0d6f` — `Add Lightning CSS, Parley text, layout oracle, merge codex rendering fixes`
2. `59bd2d2` — `Browser text measurement oracle, viewport-aware layout, remove heuristic overrides`
3. `4b1b2a3` — `Wire system font resolution into renderer via fontdb`
4. `0209e63` — `Update CLAUDE.md and docs/README with current state`
5. `139e1d6` — `Fix architecture diagram: include WebGL2 backend`
6. `3f23678` — `Update all project docs: CLAUDE.md → AGENTS.md, current-status.md refreshed`

Inventory totals:

- `98` changed files
- approximately `6.8M` inserted lines
- most of the size is ground-truth corpus data

## Repo Root

- [AGENTS.md](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/AGENTS.md)
- [CLAUDE.md](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/CLAUDE.md)
- [Cargo.lock](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/Cargo.lock)
- [Cargo.toml](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/Cargo.toml)
- [serve.rendero.json](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/serve.rendero.json)

## Accuracy Artifacts

- [accuracy/apple-comparison.json](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/accuracy/apple-comparison.json)
- [accuracy/apple-engine.json](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/accuracy/apple-engine.json)
- [accuracy/apple-web.json](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/accuracy/apple-web.json)

## Corpus Tools

- [corpus/capture-site.py](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/corpus/capture-site.py)
- [corpus/dashboard.json](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/corpus/dashboard.json)
- [corpus/dashboard.py](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/corpus/dashboard.py)

## Corpus Ground Truth

- [corpus/ground-truth/apple-macbook-pro-desktop.json](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/corpus/ground-truth/apple-macbook-pro-desktop.json)
- [corpus/ground-truth/apple-macbook-pro-mobile.json](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/corpus/ground-truth/apple-macbook-pro-mobile.json)
- [corpus/ground-truth/apple-macbook-pro-tablet.json](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/corpus/ground-truth/apple-macbook-pro-tablet.json)
- [corpus/ground-truth/fin-desktop.json](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/corpus/ground-truth/fin-desktop.json)
- [corpus/ground-truth/fin-mobile.json](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/corpus/ground-truth/fin-mobile.json)
- [corpus/ground-truth/fin-tablet.json](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/corpus/ground-truth/fin-tablet.json)
- [corpus/ground-truth/fin.json](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/corpus/ground-truth/fin.json)
- [corpus/ground-truth/github-desktop.json](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/corpus/ground-truth/github-desktop.json)
- [corpus/ground-truth/github-mobile.json](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/corpus/ground-truth/github-mobile.json)
- [corpus/ground-truth/github-tablet.json](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/corpus/ground-truth/github-tablet.json)
- [corpus/ground-truth/gumroad-desktop.json](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/corpus/ground-truth/gumroad-desktop.json)
- [corpus/ground-truth/gumroad-mobile.json](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/corpus/ground-truth/gumroad-mobile.json)
- [corpus/ground-truth/gumroad-tablet.json](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/corpus/ground-truth/gumroad-tablet.json)
- [corpus/ground-truth/hacker-news-desktop.json](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/corpus/ground-truth/hacker-news-desktop.json)
- [corpus/ground-truth/hacker-news-mobile.json](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/corpus/ground-truth/hacker-news-mobile.json)
- [corpus/ground-truth/hacker-news-tablet.json](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/corpus/ground-truth/hacker-news-tablet.json)
- [corpus/ground-truth/linear-desktop.json](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/corpus/ground-truth/linear-desktop.json)
- [corpus/ground-truth/linear-mobile.json](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/corpus/ground-truth/linear-mobile.json)
- [corpus/ground-truth/linear-tablet.json](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/corpus/ground-truth/linear-tablet.json)
- [corpus/ground-truth/tailadmin-desktop.json](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/corpus/ground-truth/tailadmin-desktop.json)
- [corpus/ground-truth/tailadmin-mobile.json](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/corpus/ground-truth/tailadmin-mobile.json)
- [corpus/ground-truth/tailadmin-tablet.json](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/corpus/ground-truth/tailadmin-tablet.json)

## Corpus Site Captures

- [corpus/sites/apple-macbook-pro/meta.json](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/corpus/sites/apple-macbook-pro/meta.json)
- [corpus/sites/apple-macbook-pro/page.mhtml](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/corpus/sites/apple-macbook-pro/page.mhtml)
- [corpus/sites/fin/meta.json](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/corpus/sites/fin/meta.json)
- [corpus/sites/fin/page.mhtml](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/corpus/sites/fin/page.mhtml)
- [corpus/sites/github/meta.json](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/corpus/sites/github/meta.json)
- [corpus/sites/github/page.mhtml](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/corpus/sites/github/page.mhtml)
- [corpus/sites/gumroad/meta.json](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/corpus/sites/gumroad/meta.json)
- [corpus/sites/gumroad/page.mhtml](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/corpus/sites/gumroad/page.mhtml)
- [corpus/sites/hacker-news/meta.json](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/corpus/sites/hacker-news/meta.json)
- [corpus/sites/hacker-news/page.mhtml](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/corpus/sites/hacker-news/page.mhtml)
- [corpus/sites/linear/meta.json](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/corpus/sites/linear/meta.json)
- [corpus/sites/linear/page.mhtml](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/corpus/sites/linear/page.mhtml)
- [corpus/sites/tailadmin/meta.json](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/corpus/sites/tailadmin/meta.json)
- [corpus/sites/tailadmin/page.mhtml](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/corpus/sites/tailadmin/page.mhtml)

## Core Engine

- [crates/core/src/layout.rs](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/crates/core/src/layout.rs)
- [crates/core/src/node.rs](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/crates/core/src/node.rs)
- [crates/core/src/properties.rs](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/crates/core/src/properties.rs)
- [crates/core/src/providers.rs](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/crates/core/src/providers.rs)
- [crates/core/src/taffy_layout.rs](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/crates/core/src/taffy_layout.rs)

## New Crates

- [crates/css/Cargo.toml](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/crates/css/Cargo.toml)
- [crates/css/src/lib.rs](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/crates/css/src/lib.rs)
- [crates/text/Cargo.toml](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/crates/text/Cargo.toml)
- [crates/text/src/lib.rs](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/crates/text/src/lib.rs)

## Native / Renderer / WASM

- [crates/native-ffi/src/lib.rs](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/crates/native-ffi/src/lib.rs)
- [crates/native-shell/src/engine_bridge.rs](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/crates/native-shell/src/engine_bridge.rs)
- [crates/native-shell/src/main.rs](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/crates/native-shell/src/main.rs)
- [crates/native-shell/src/providers.rs](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/crates/native-shell/src/providers.rs)
- [crates/native-shell/src/quickjs_runtime.rs](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/crates/native-shell/src/quickjs_runtime.rs)
- [crates/renderer/src/text.rs](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/crates/renderer/src/text.rs)
- [crates/wasm/Cargo.toml](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/crates/wasm/Cargo.toml)
- [crates/wasm/src/fig_import.rs](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/crates/wasm/src/fig_import.rs)
- [crates/wasm/src/lib.rs](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/crates/wasm/src/lib.rs)

## DOM Shim Accuracy Tooling

- [docs/demos/dom-shim/.DS_Store](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/docs/demos/dom-shim/.DS_Store)
- [docs/demos/dom-shim/accuracy/extract-ground-truth.js](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/docs/demos/dom-shim/accuracy/extract-ground-truth.js)
- [docs/demos/dom-shim/accuracy/layout-accuracy.html](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/docs/demos/dom-shim/accuracy/layout-accuracy.html)
- [docs/demos/dom-shim/accuracy/test-corpus.js](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/docs/demos/dom-shim/accuracy/test-corpus.js)

## DOM Shim Runtime

- [docs/demos/dom-shim/build.mjs](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/docs/demos/dom-shim/build.mjs)
- [docs/demos/dom-shim/index.html](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/docs/demos/dom-shim/index.html)
- [docs/demos/dom-shim/shims/document.js](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/docs/demos/dom-shim/shims/document.js)
- [docs/demos/dom-shim/shims/element.js](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/docs/demos/dom-shim/shims/element.js)
- [docs/demos/dom-shim/shims/engine-native.js](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/docs/demos/dom-shim/shims/engine-native.js)
- [docs/demos/dom-shim/shims/engine-runtime.js](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/docs/demos/dom-shim/shims/engine-runtime.js)
- [docs/demos/dom-shim/shims/engine.js](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/docs/demos/dom-shim/shims/engine.js)
- [docs/demos/dom-shim/shims/index.js](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/docs/demos/dom-shim/shims/index.js)
- [docs/demos/dom-shim/shims/layout-style.js](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/docs/demos/dom-shim/shims/layout-style.js)
- [docs/demos/dom-shim/shims/node.js](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/docs/demos/dom-shim/shims/node.js)
- [docs/demos/dom-shim/shims/rendero-api.js](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/docs/demos/dom-shim/shims/rendero-api.js)
- [docs/demos/dom-shim/shims/style.js](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/docs/demos/dom-shim/shims/style.js)
- [docs/demos/dom-shim/shims/text-node.js](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/docs/demos/dom-shim/shims/text-node.js)
- [docs/demos/dom-shim/shims/window.js](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/docs/demos/dom-shim/shims/window.js)
- [docs/demos/dom-shim/src/entry-macos-vue.js](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/docs/demos/dom-shim/src/entry-macos-vue.js)
- [docs/demos/dom-shim/src/entry-macos.jsx](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/docs/demos/dom-shim/src/entry-macos.jsx)

## Generated WASM Package

- [docs/pkg/rendero.d.ts](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/docs/pkg/rendero.d.ts)
- [docs/pkg/rendero.js](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/docs/pkg/rendero.js)
- [docs/pkg/rendero_bg.wasm](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/docs/pkg/rendero_bg.wasm)
- [docs/pkg/rendero_bg.wasm.d.ts](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/docs/pkg/rendero_bg.wasm.d.ts)

## Project Docs

- [docs/project/README.md](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/docs/project/README.md)
- [docs/project/current-status.md](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/docs/project/current-status.md)

## Scripts

- [scripts/capture-engine-truth.py](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/scripts/capture-engine-truth.py)
- [scripts/capture-ground-truth.py](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/scripts/capture-ground-truth.py)
- [scripts/capture_dom_shim.py](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/scripts/capture_dom_shim.py)
- [scripts/compare-layout.py](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/scripts/compare-layout.py)
- [scripts/install-hooks.sh](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/scripts/install-hooks.sh)
- [scripts/pre-commit](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/scripts/pre-commit)
- [scripts/verify_rendero_runtime.sh](/Users/kumardivyarajat/.codex/worktrees/fa89/rendero/scripts/verify_rendero_runtime.sh)

## Sync Strategy

Because the goal is content sync rather than preserving the other branch's git
history, files should be synced in this order:

1. code and config required for compilation
2. DOM-shim runtime and verification scripts
3. docs
4. large accuracy and corpus artifacts

This keeps the branch buildable while still eventually syncing every file listed
above.
