# Parity Shortcuts

This file tracks temporary shortcuts used to move the browser/native parity work
forward without losing sight of the proper long-term translation model.

## Active Shortcuts

- Native comparison baseline is currently forced to the desktop browser oracle:
  `1440x900` at `DPR 1`.
  Reason:
  apples-to-apples comparison while layout translation is still being fixed.
  Remove when viewport compatibility is benchmarked as its own matrix.

- Native headless verification currently uses a tall forced viewport height.
  Reason:
  make the full page visible in one artifact.
  Consequence:
  headless native page height is not yet a true oracle for browser page height.

- Parent-relative `%` sizing is now carried as explicit layout data, but only
  for simple size cases.
  Reason:
  prevent nested media/card nodes from blowing out to full viewport width.
  Replace with:
  a general containing-block-relative percentage sizing model across width,
  height, min/max, flex-basis, and positioned elements.

- Browser text nodes are still eagerly measured in the shim with no true
  containing-block width for wrapped copy.
  Reason:
  even with `BrowserTextMeasurer` in the WASM trait path, browser Rendero still
  needs a temporary JS-side eager size sync for some text nodes until
  constrained wrapping is fully handled through the shared layout pipeline.
  Consequence:
  constrained paragraphs in feature cards stay single-line too long and inflate
  card widths/heights.

- Frame heights currently get a bottom-up safeguard after Taffy layout.
  Reason:
  prevent containers from ending above their deepest child while page-height
  propagation is still being tightened.
  Consequence:
  page-height parity is much closer, but one top-level wrapper still remains
  slightly short and this safeguard should not become a permanent substitute for
  correct authored/layout inputs.

## Still Missing Properly

- General `%` sizing must stay parent/containing-block relative until layout.
- `vw` / `vh` must stay viewport relative and be tested separately.
- Media-query and browser-environment resolution must live in style resolution,
  not in the engine.
- Live native screenshot capture still depends on Codex macOS permissions.

## Follow-up Test Matrix

After desktop parity is stable, add separate test coverage for:

- desktop / tablet / mobile viewport width and height
- DPR differences
- viewport units (`vw`, `vh`)
- containing-block percentages (`%`)
- media queries
- browser environment traits:
  color scheme, reduced motion, pointer/hover, orientation
