#!/usr/bin/env python3
"""
Capture layout ground truth from any web page.

Extracts every element's computed bounds and styles from the browser,
saves as JSON. This is the ORACLE — browser is always right.

Usage:
    python3 scripts/capture-ground-truth.py <url> <output.json>

Examples:
    python3 scripts/capture-ground-truth.py http://localhost:5555/demos/dom-shim/ accuracy/apple-web.json
    python3 scripts/capture-ground-truth.py https://news.ycombinator.com accuracy/hn-web.json
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


EXTRACTOR_JS = """
() => {
    const STYLE_PROPERTIES = [
        'display', 'position', 'top', 'right', 'bottom', 'left',
        'width', 'height', 'minWidth', 'minHeight', 'maxWidth', 'maxHeight',
        'overflow', 'overflowX', 'overflowY', 'boxSizing',
        'flexDirection', 'flexWrap', 'justifyContent', 'alignItems', 'alignContent',
        'flexGrow', 'flexShrink', 'flexBasis', 'alignSelf', 'order',
        'gap', 'rowGap', 'columnGap',
        'marginTop', 'marginRight', 'marginBottom', 'marginLeft',
        'paddingTop', 'paddingRight', 'paddingBottom', 'paddingLeft',
        'borderTopWidth', 'borderRightWidth', 'borderBottomWidth', 'borderLeftWidth',
        'borderRadius', 'borderTopLeftRadius', 'borderTopRightRadius',
        'borderBottomRightRadius', 'borderBottomLeftRadius',
        'backgroundColor', 'color', 'opacity',
        'backgroundImage',
        'fontSize', 'fontWeight', 'fontFamily', 'fontStyle',
        'lineHeight', 'textAlign',
        'transform',
        'zIndex',
        'boxShadow',
    ];

    const elements = [];
    let nextId = 0;
    const idMap = new WeakMap();

    function assignId(el) {
        if (!idMap.has(el)) idMap.set(el, nextId++);
        return idMap.get(el);
    }

    function getDirectText(el) {
        let text = '';
        for (const child of el.childNodes) {
            if (child.nodeType === 3) text += child.textContent;
        }
        return text.trim().slice(0, 200);
    }

    function walk(el, depth) {
        if (el.nodeType !== 1) return;
        const tag = el.tagName.toLowerCase();
        if (['script', 'style', 'meta', 'link', 'head', 'noscript', 'template'].includes(tag)) return;

        const id = assignId(el);
        const parentId = el.parentElement && idMap.has(el.parentElement) ? idMap.get(el.parentElement) : null;
        const rect = el.getBoundingClientRect();
        const computed = window.getComputedStyle(el);

        const styles = {};
        const computedStyle = {};
        for (const propName of computed) {
            computedStyle[propName] = computed.getPropertyValue(propName);
        }
        for (const prop of STYLE_PROPERTIES) {
            const kebab = prop.replace(/([A-Z])/g, '-$1').toLowerCase();
            const val = computed.getPropertyValue(kebab);
            if (val && val !== 'none' && val !== 'normal' && val !== 'auto'
                && val !== '0px' && val !== '0' && val !== 'rgba(0, 0, 0, 0)'
                && val !== 'visible' && val !== 'static' && val !== 'content-box') {
                styles[prop] = val;
            }
        }
        // Always include these
        styles.display = computed.display;
        styles.position = computed.position;
        styles.boxSizing = computed.boxSizing;

        elements.push({
            id, parentId, depth, tag,
            htmlId: el.id || null,
            className: el.className ? String(el.className).slice(0, 100) : null,
            text: getDirectText(el),
            bounds: {
                x: Math.round(rect.left * 10) / 10,
                y: Math.round(rect.top * 10) / 10,
                width: Math.round(rect.width * 10) / 10,
                height: Math.round(rect.height * 10) / 10,
            },
            computedStyle,
            styles,
            childCount: el.children.length,
        });

        for (const child of el.children) walk(child, depth + 1);
    }

    // Start from the #root element if it exists, otherwise body
    const startEl = document.getElementById('root') || document.body;
    walk(startEl, 0);

    return {
        url: location.href,
        title: document.title,
        timestamp: new Date().toISOString(),
        viewport: {
            width: window.innerWidth,
            height: window.innerHeight,
            dpr: window.devicePixelRatio,
            scrollHeight: document.documentElement.scrollHeight,
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

    # Viewport size — standardize for reproducibility
    viewport_width = int(sys.argv[3]) if len(sys.argv) > 3 else 1440
    viewport_height = int(sys.argv[4]) if len(sys.argv) > 4 else 900

    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True)
        page = browser.new_page(
            viewport={"width": viewport_width, "height": viewport_height},
            device_scale_factor=1,
        )

        # Add cache-busting
        cache_bust = int(time.time() * 1000)
        separator = "&" if "?" in url else "?"
        page.goto(f"{url}{separator}v={cache_bust}", wait_until="networkidle")

        # Wait for React to render
        page.wait_for_timeout(2000)

        # Extract ground truth
        report = page.evaluate(EXTRACTOR_JS)

        # Save
        output_path.write_text(json.dumps(report, indent=2))
        print(f"[Ground Truth] {report['elementCount']} elements from {url}")
        print(f"[Ground Truth] Viewport: {report['viewport']['width']}x{report['viewport']['height']}")
        print(f"[Ground Truth] Scroll height: {report['viewport']['scrollHeight']}px")
        print(f"[Ground Truth] Saved to {output_path}")

        # Also take a screenshot for visual reference
        screenshot_path = output_path.with_suffix('.png')
        page.screenshot(path=str(screenshot_path), full_page=True)
        print(f"[Ground Truth] Screenshot: {screenshot_path}")

        browser.close()

    return 0


if __name__ == "__main__":
    sys.exit(main())
