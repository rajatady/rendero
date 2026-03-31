# Codex Current State

This document snapshots the current state of the `codex/rendering-dom-parity`
worktree before integrating newer `poc/rendering-dom` commits.

## Branch / Base

- Worktree: `/Users/kumardivyarajat/.codex/worktrees/fa89/rendero`
- Branch: `codex/rendering-dom-parity`
- Current committed base: `861f015` — `Build Rendero DOM-shim runtime baseline and parity verification loop`

## What Was Added After `861f015`

These changes existed in the working tree and are being committed as a safety
snapshot before any history integration:

- Native bridge parity:
  - forward `fontFamily`, `textAlign`, `stroke`, `rotation`, `shadow`, and
    `linearGradient` through the native DOM-shim bridge
  - files:
    - `crates/native-shell/src/engine_bridge.rs`
    - `docs/demos/dom-shim/shims/engine-native.js`

- Shared scroll model:
  - add a window scroll shim that synchronizes `window.scrollY` with the
    Rendero camera
  - use the same scroll path in browser Rendero and native
  - files:
    - `docs/demos/dom-shim/shims/window.js`
    - `docs/demos/dom-shim/shims/rendero-api.js`
    - `docs/demos/dom-shim/shims/index.js`
    - `docs/demos/dom-shim/src/entry-macos.jsx`
    - `docs/demos/dom-shim/src/entry-macos-vue.js`
    - `crates/native-shell/src/main.rs`

- Native scroll fix:
  - correct the wheel delta sign in the Rust host so downward scrolling moves
    the page downward instead of effectively scrolling upward from the top
  - file:
    - `crates/native-shell/src/main.rs`

- Browser verification improvements:
  - capture both top-of-page and scrolled-state screenshots for React Web and
    React Rendero
  - record browser scroll/camera state in the artifact JSON
  - file:
    - `scripts/capture_dom_shim.py`

- Renderer text experiment:
  - switch the CPU text renderer toward system font resolution via `fontdb`
    instead of always using embedded Roboto Mono
  - file:
    - `crates/renderer/src/text.rs`

## Verified Behavior

### Browser React Rendero

- top-of-page render works
- scrolled render works
- scroll position and engine camera are synchronized

Latest verified browser state from the parity artifacts:

- `scrollY = 1500`
- `scrollHeight = 3057`
- `Rendero.engine.getCamera().y = 1500`

This proves browser scroll parity is working, not just layout at the top of the
page.

### Native Rendero

- headless native render works and produces a full-page artifact
- native visual fidelity improved after style bridge parity changes
- native wheel scrolling had a host-side sign bug, now fixed in `main.rs`

### Browser Render Backend

- browser Rendero currently defaults to the raster path
- Canvas2D backend was tested and gives much better text rendering, but still
  has viewport / scale / clipping issues that must be normalized before it can
  become the default again

## Main Remaining Gaps

- Browser Rendero text fidelity is still behind React Web
- Native text and spacing fidelity are still behind React Web
- Browser Canvas2D backend needs viewport / DPR / camera normalization
- Text measurement and layout need to be reconciled with the newer
  `poc/rendering-dom` work (`3df0d6f`, `59bd2d2`)

## Integration Risk Notes

The live `poc/rendering-dom` branch is ahead and has overlapping changes in the
same areas:

- `crates/native-shell/src/engine_bridge.rs`
- `crates/native-shell/src/main.rs`
- `crates/renderer/src/text.rs`
- `docs/demos/dom-shim/shims/engine-native.js`
- `docs/demos/dom-shim/shims/engine.js`
- `docs/demos/dom-shim/shims/window.js`

This means pulling in the later `poc/rendering-dom` commits will not
fast-forward cleanly. Expect real integration work and likely merge or
cherry-pick conflicts.

## Recommended Next Integration Order

1. Commit this safety snapshot
2. Bring in `a8da30d` trivially
3. Review and integrate `3df0d6f` carefully
4. Review and integrate `59bd2d2` carefully
5. Reconcile the overlapping text / layout / native host changes
