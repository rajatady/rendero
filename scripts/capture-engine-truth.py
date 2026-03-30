#!/usr/bin/env python3
"""
Capture layout results from the Rendero engine (WASM canvas mode).

Loads the demo page, switches to React (Rendero) mode, extracts
engine node bounds, saves as JSON for comparison with browser ground truth.

Usage:
    python3 scripts/capture-engine-truth.py <url> <output.json>
"""

import json
import sys
import time
from pathlib import Path

try:
    from playwright.sync_api import sync_playwright
except ImportError:
    print("Install playwright: pip install playwright && playwright install chromium", file=sys.stderr)
    sys.exit(1)


ENGINE_EXTRACTOR_JS = """
() => {
    // The engine exposes node bounds through the Rendero namespace
    const debug = window.__RENDERO_DEBUG__;
    if (!debug || !debug.engine) {
        return { error: 'Rendero engine not found. Is the page in Rendero mode?' };
    }

    const engine = debug.engine;
    const shimDoc = debug.shimDocument;
    if (!shimDoc) {
        return { error: 'Shim document not found.' };
    }

    const elements = [];
    let nextId = 0;

    function walkShimNode(node, depth, parentId) {
        const id = nextId++;

        // Get engine bounds if available
        let bounds = { x: 0, y: 0, width: 0, height: 0 };
        if (node._engineId != null) {
            try {
                // Try the Rendero namespace first
                const b = window.Rendero?.engine?.getNodeBounds?.(node._engineId);
                if (b) {
                    bounds = {
                        x: Math.round((b.x || 0) * 10) / 10,
                        y: Math.round((b.y || 0) * 10) / 10,
                        width: Math.round((b.width || 0) * 10) / 10,
                        height: Math.round((b.height || 0) * 10) / 10,
                    };
                }
            } catch (e) {
                // Ignore
            }
        }

        // Get style values that were set
        const styles = {};
        if (node.style && node.style._values) {
            const v = node.style._values;
            for (const [key, val] of Object.entries(v)) {
                if (val) styles[key] = String(val);
            }
        }

        elements.push({
            id,
            parentId,
            depth,
            tag: node.tagName ? node.tagName.toLowerCase() : '#text',
            engineId: node._engineId || null,
            text: node._isTextElement ? (node.textContent || '').slice(0, 200) : '',
            bounds,
            styles,
            childCount: node.childNodes ? node.childNodes.length : 0,
        });

        // Walk children
        if (node.childNodes) {
            for (const child of node.childNodes) {
                if (child.nodeType === 1 || (child.tagName && child.tagName !== '#text')) {
                    walkShimNode(child, depth + 1, id);
                }
            }
        }
    }

    // Start from shim document body
    const body = shimDoc.body;
    if (body) {
        walkShimNode(body, 0, null);
    }

    return {
        url: location.href,
        title: document.title,
        timestamp: new Date().toISOString(),
        viewport: {
            width: window.innerWidth,
            height: window.innerHeight,
        },
        elementCount: elements.length,
        elements,
    };
}
"""


def main():
    if len(sys.argv) < 3:
        print(f"Usage: {sys.argv[0]} <url> <output.json>", file=sys.stderr)
        return 1

    url = sys.argv[1]
    output_path = Path(sys.argv[2]).resolve()
    output_path.parent.mkdir(parents=True, exist_ok=True)

    viewport_width = int(sys.argv[3]) if len(sys.argv) > 3 else 1440
    viewport_height = int(sys.argv[4]) if len(sys.argv) > 4 else 900

    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True)
        page = browser.new_page(
            viewport={"width": viewport_width, "height": viewport_height},
            device_scale_factor=1,
        )

        cache_bust = int(time.time() * 1000)
        separator = "&" if "?" in url else "?"
        page.goto(f"{url}{separator}v={cache_bust}", wait_until="networkidle")
        page.wait_for_timeout(2000)

        # Switch to React (Rendero) mode by clicking the button
        try:
            rendero_btn = page.get_by_role("button", name="React (Rendero)")
            rendero_btn.click()
            page.wait_for_timeout(3000)
        except Exception as e:
            print(f"[Engine] Could not switch to Rendero mode: {e}", file=sys.stderr)

        # Take screenshot
        screenshot_path = output_path.with_suffix('.png')
        page.screenshot(path=str(screenshot_path), full_page=False)
        print(f"[Engine] Screenshot: {screenshot_path}")

        # Extract engine bounds
        report = page.evaluate(ENGINE_EXTRACTOR_JS)

        if 'error' in report:
            print(f"[Engine] Error: {report['error']}", file=sys.stderr)
            return 1

        output_path.write_text(json.dumps(report, indent=2))
        print(f"[Engine] {report['elementCount']} elements from Rendero engine")
        print(f"[Engine] Saved to {output_path}")

        browser.close()

    return 0


if __name__ == "__main__":
    sys.exit(main())
