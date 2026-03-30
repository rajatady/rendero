#!/usr/bin/env python3
"""
Deep ground truth capture from a real website.

Captures everything measurable about a page:
- Layout: bounds, computed styles for every visible element
- Scroll: every scroll container, their scroll dimensions
- Text: line breaks via Range API, resolved fonts
- Interactivity: event listeners, hover targets, focus order
- Responsive: captures at multiple viewport widths
- Stacking: resolved z-index order
- Media: image natural dimensions, background images
- Transitions: elements with CSS transitions/animations

The output is a single JSON file that serves as the oracle.
When the Rendero engine can render this page, we diff against this.

Usage:
    python3 corpus/capture-site.py <url> <site-name>

Examples:
    python3 corpus/capture-site.py https://gumroad.com gumroad
    python3 corpus/capture-site.py https://fin.ai fin
    python3 corpus/capture-site.py https://www.apple.com/macbook-pro/ apple-macbook-pro
"""

import json
import sys
import time
from pathlib import Path

try:
    from playwright.sync_api import sync_playwright
except ImportError:
    print("Install: pip install playwright && playwright install chromium", file=sys.stderr)
    sys.exit(1)

REPO = Path(__file__).resolve().parent.parent
SITES_DIR = REPO / "corpus" / "sites"
TRUTH_DIR = REPO / "corpus" / "ground-truth"

VIEWPORTS = [
    {"width": 1440, "height": 900, "label": "desktop"},
    {"width": 768, "height": 1024, "label": "tablet"},
    {"width": 375, "height": 812, "label": "mobile"},
]

# ── The main extractor runs inside the browser via page.evaluate() ──

EXTRACTOR_JS = """
(scrollPositions) => {
    const STYLE_PROPS = [
        // Layout
        'display', 'position', 'top', 'right', 'bottom', 'left',
        'width', 'height', 'minWidth', 'minHeight', 'maxWidth', 'maxHeight',
        'overflow', 'overflowX', 'overflowY', 'boxSizing', 'float', 'clear',
        // Flexbox
        'flexDirection', 'flexWrap', 'justifyContent', 'alignItems', 'alignContent',
        'flexGrow', 'flexShrink', 'flexBasis', 'alignSelf', 'order', 'flex',
        'gap', 'rowGap', 'columnGap',
        // Grid
        'gridTemplateColumns', 'gridTemplateRows', 'gridColumn', 'gridRow',
        'gridAutoFlow', 'gridAutoColumns', 'gridAutoRows',
        'gridColumnStart', 'gridColumnEnd', 'gridRowStart', 'gridRowEnd',
        // Box model
        'marginTop', 'marginRight', 'marginBottom', 'marginLeft',
        'paddingTop', 'paddingRight', 'paddingBottom', 'paddingLeft',
        'borderTopWidth', 'borderRightWidth', 'borderBottomWidth', 'borderLeftWidth',
        'borderTopStyle', 'borderRightStyle', 'borderBottomStyle', 'borderLeftStyle',
        'borderTopColor', 'borderRightColor', 'borderBottomColor', 'borderLeftColor',
        'borderTopLeftRadius', 'borderTopRightRadius',
        'borderBottomRightRadius', 'borderBottomLeftRadius',
        // Visual
        'backgroundColor', 'color', 'opacity',
        'backgroundImage', 'backgroundSize', 'backgroundPosition', 'backgroundRepeat',
        'boxShadow', 'outline', 'outlineOffset',
        // Text
        'fontSize', 'fontWeight', 'fontFamily', 'fontStyle', 'fontVariant',
        'lineHeight', 'textAlign', 'textDecoration', 'textDecorationColor',
        'textTransform', 'letterSpacing', 'wordSpacing', 'whiteSpace',
        'textOverflow', 'wordBreak', 'overflowWrap', 'textIndent',
        'verticalAlign',
        // Transform
        'transform', 'transformOrigin',
        // Transitions & animations
        'transition', 'transitionProperty', 'transitionDuration',
        'transitionTimingFunction', 'transitionDelay',
        'animation', 'animationName', 'animationDuration',
        // Other
        'zIndex', 'visibility', 'cursor', 'pointerEvents',
        'objectFit', 'objectPosition',
        'contain', 'isolation', 'mixBlendMode',
        'backdropFilter', 'filter',
        'willChange', 'clipPath',
    ];

    const elements = [];
    const scrollContainers = [];
    const images = [];
    const interactiveElements = [];
    const fixedStickyElements = [];
    const textElements = [];
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
        return text.trim().slice(0, 500);
    }

    function getSelector(el) {
        if (el.id) return '#' + CSS.escape(el.id);
        const tag = el.tagName.toLowerCase();
        const cls = el.className && typeof el.className === 'string'
            ? '.' + el.className.trim().split(/\\s+/).slice(0, 2).map(c => CSS.escape(c)).join('.')
            : '';
        return tag + cls;
    }

    function getTextLines(el) {
        // Use Range API to find actual line breaks
        const textNodes = [];
        const walker = document.createTreeWalker(el, NodeFilter.SHOW_TEXT);
        let node;
        while (node = walker.nextNode()) {
            if (node.textContent.trim()) textNodes.push(node);
        }
        if (textNodes.length === 0) return null;

        const lines = [];
        let currentLineTop = null;
        let currentLine = '';

        for (const textNode of textNodes) {
            const range = document.createRange();
            for (let i = 0; i < textNode.textContent.length; i++) {
                range.setStart(textNode, i);
                range.setEnd(textNode, Math.min(i + 1, textNode.textContent.length));
                const rects = range.getClientRects();
                if (rects.length === 0) continue;
                const rect = rects[0];
                const top = Math.round(rect.top);

                if (currentLineTop === null || Math.abs(top - currentLineTop) > 2) {
                    if (currentLine.trim()) {
                        lines.push({
                            text: currentLine.trim().slice(0, 200),
                            y: currentLineTop,
                            height: Math.round(rect.height * 10) / 10,
                        });
                    }
                    currentLineTop = top;
                    currentLine = textNode.textContent[i];
                } else {
                    currentLine += textNode.textContent[i];
                }
            }
        }
        if (currentLine.trim()) {
            lines.push({ text: currentLine.trim().slice(0, 200), y: currentLineTop, height: 0 });
        }

        return lines.length > 0 ? lines : null;
    }

    function getResolvedFont(el) {
        // Canvas trick to get the actual resolved font
        const computed = window.getComputedStyle(el);
        return {
            family: computed.fontFamily,
            size: computed.fontSize,
            weight: computed.fontWeight,
            style: computed.fontStyle,
            lineHeight: computed.lineHeight,
        };
    }

    function getEventListenerTypes(el) {
        // Can't enumerate JS listeners, but we can detect interactive attributes
        const types = [];
        if (el.onclick || el.getAttribute('onclick')) types.push('click');
        if (el.onmouseover || el.getAttribute('onmouseover')) types.push('mouseover');
        if (el.onfocus || el.getAttribute('onfocus')) types.push('focus');
        if (el.tagName === 'A' && el.href) types.push('link');
        if (el.tagName === 'BUTTON') types.push('button');
        if (['INPUT', 'TEXTAREA', 'SELECT'].includes(el.tagName)) types.push('input');
        if (el.getAttribute('role') === 'button') types.push('role-button');
        if (el.getAttribute('tabindex')) types.push('focusable');
        if (el.getAttribute('aria-label') || el.getAttribute('aria-labelledby')) types.push('accessible');
        const cursor = window.getComputedStyle(el).cursor;
        if (cursor === 'pointer') types.push('pointer-cursor');
        return types.length > 0 ? types : null;
    }

    function walk(el, depth, parentPath) {
        if (el.nodeType !== 1) return;
        const tag = el.tagName.toLowerCase();
        if (['script', 'style', 'meta', 'link', 'head', 'noscript', 'template'].includes(tag)) return;

        const id = assignId(el);
        const parentId = el.parentElement && idMap.has(el.parentElement) ? idMap.get(el.parentElement) : null;
        const rect = el.getBoundingClientRect();
        const computed = window.getComputedStyle(el);

        // Collect ALL computed styles — don't filter, store everything
        const styles = {};
        for (const prop of STYLE_PROPS) {
            const kebab = prop.replace(/([A-Z])/g, '-$1').toLowerCase();
            const val = computed.getPropertyValue(kebab);
            if (val !== '' && val !== undefined) {
                styles[prop] = val;
            }
        }

        const selector = getSelector(el);
        const path = parentPath ? parentPath + ' > ' + selector : selector;
        const text = getDirectText(el);

        const entry = {
            id, parentId, depth, tag,
            selector: path,
            htmlId: el.id || null,
            className: el.className && typeof el.className === 'string' ? el.className.slice(0, 200) : null,
            text: text || null,
            bounds: {
                x: Math.round(rect.left * 100) / 100,
                y: Math.round(rect.top * 100) / 100,
                width: Math.round(rect.width * 100) / 100,
                height: Math.round(rect.height * 100) / 100,
            },
            styles,
            childCount: el.children.length,
        };

        // ── Scroll container detection ──
        const isScrollable =
            (el.scrollHeight > el.clientHeight && (computed.overflowY === 'auto' || computed.overflowY === 'scroll')) ||
            (el.scrollWidth > el.clientWidth && (computed.overflowX === 'auto' || computed.overflowX === 'scroll'));

        if (isScrollable) {
            entry.scroll = {
                scrollWidth: el.scrollWidth,
                scrollHeight: el.scrollHeight,
                clientWidth: el.clientWidth,
                clientHeight: el.clientHeight,
                scrollTop: el.scrollTop,
                scrollLeft: el.scrollLeft,
            };
            scrollContainers.push({ id, selector: path, scroll: entry.scroll });
        }

        // ── Fixed / sticky detection ──
        if (computed.position === 'fixed' || computed.position === 'sticky') {
            fixedStickyElements.push({
                id, selector: path, position: computed.position,
                top: computed.top, bottom: computed.bottom,
                left: computed.left, right: computed.right,
                zIndex: computed.zIndex,
            });
        }

        // ── Image detection ──
        if (tag === 'img') {
            images.push({
                id, selector: path,
                src: el.src ? el.src.slice(0, 300) : null,
                naturalWidth: el.naturalWidth,
                naturalHeight: el.naturalHeight,
                renderedWidth: Math.round(rect.width),
                renderedHeight: Math.round(rect.height),
                alt: el.alt || null,
                loading: el.loading || null,
                objectFit: computed.objectFit,
            });
        }

        // ── Background image detection ──
        if (computed.backgroundImage && computed.backgroundImage !== 'none') {
            entry.backgroundImage = {
                value: computed.backgroundImage.slice(0, 500),
                size: computed.backgroundSize,
                position: computed.backgroundPosition,
                repeat: computed.backgroundRepeat,
            };
        }

        // ── Interactive element detection ──
        const interactiveTypes = getEventListenerTypes(el);
        if (interactiveTypes) {
            entry.interactive = interactiveTypes;
            interactiveElements.push({ id, selector: path, types: interactiveTypes, tag });
        }

        // ── Form element details ──
        if (['INPUT', 'TEXTAREA', 'SELECT'].includes(el.tagName)) {
            entry.formElement = {
                type: el.type || null,
                placeholder: el.placeholder || null,
                value: el.value ? el.value.slice(0, 100) : null,
                disabled: el.disabled,
                required: el.required,
                name: el.name || null,
            };
        }

        // ── Text line breaks (only for leaf-ish text elements, skip deep trees) ──
        if (text && text.length > 10 && el.children.length <= 2) {
            const lines = getTextLines(el);
            if (lines && lines.length > 1) {
                entry.textLines = lines;
                textElements.push({ id, selector: path, lineCount: lines.length });
            }
        }

        // ── Resolved font ──
        if (text) {
            entry.resolvedFont = getResolvedFont(el);
        }

        // ── Accessibility ──
        const role = el.getAttribute('role');
        const ariaLabel = el.getAttribute('aria-label');
        const ariaHidden = el.getAttribute('aria-hidden');
        if (role || ariaLabel || ariaHidden) {
            entry.aria = {};
            if (role) entry.aria.role = role;
            if (ariaLabel) entry.aria.label = ariaLabel;
            if (ariaHidden) entry.aria.hidden = ariaHidden;
        }

        // ── Transition / animation ──
        if (computed.transitionProperty && computed.transitionProperty !== 'all' && computed.transitionProperty !== 'none') {
            entry.transition = {
                property: computed.transitionProperty,
                duration: computed.transitionDuration,
                timing: computed.transitionTimingFunction,
                delay: computed.transitionDelay,
            };
        }
        if (computed.animationName && computed.animationName !== 'none') {
            entry.animation = {
                name: computed.animationName,
                duration: computed.animationDuration,
                timing: computed.animationTimingFunction,
                iterationCount: computed.animationIterationCount,
            };
        }

        elements.push(entry);
        for (const child of el.children) walk(child, depth + 1, path);
    }

    // ── Scroll to positions and capture fixed/sticky behavior ──
    const scrollBehavior = [];
    const originalScrollTop = window.scrollY;

    for (const pos of scrollPositions) {
        window.scrollTo(0, pos);
        // Small delay for repaints handled by caller

        const fixedAtScroll = [];
        for (const el of document.querySelectorAll('*')) {
            const cs = window.getComputedStyle(el);
            if (cs.position === 'fixed' || cs.position === 'sticky') {
                const r = el.getBoundingClientRect();
                fixedAtScroll.push({
                    selector: getSelector(el),
                    position: cs.position,
                    bounds: {
                        x: Math.round(r.left),
                        y: Math.round(r.top),
                        width: Math.round(r.width),
                        height: Math.round(r.height),
                    },
                });
            }
        }
        scrollBehavior.push({ scrollY: pos, fixedElements: fixedAtScroll });
    }

    // Restore scroll
    window.scrollTo(0, originalScrollTop);

    // ── Walk the full page ──
    walk(document.body, 0, '');

    // ── Page-level scroll info ──
    const pageScroll = {
        scrollWidth: document.documentElement.scrollWidth,
        scrollHeight: document.documentElement.scrollHeight,
        clientWidth: document.documentElement.clientWidth,
        clientHeight: document.documentElement.clientHeight,
    };

    // ── z-index stacking snapshot ──
    // Get all elements with explicit z-index
    const stackingContexts = [];
    for (const el of document.querySelectorAll('*')) {
        const cs = window.getComputedStyle(el);
        const z = cs.zIndex;
        if (z !== 'auto' && z !== '0') {
            const r = el.getBoundingClientRect();
            stackingContexts.push({
                selector: getSelector(el),
                zIndex: parseInt(z) || 0,
                position: cs.position,
                bounds: { x: Math.round(r.left), y: Math.round(r.top), width: Math.round(r.width), height: Math.round(r.height) },
            });
        }
    }
    stackingContexts.sort((a, b) => a.zIndex - b.zIndex);

    return {
        url: location.href,
        title: document.title,
        timestamp: new Date().toISOString(),
        viewport: {
            width: window.innerWidth,
            height: window.innerHeight,
            dpr: window.devicePixelRatio,
        },
        pageScroll,
        elementCount: elements.length,
        elements,
        scrollContainers,
        scrollBehavior,
        fixedStickyElements,
        stackingContexts,
        images,
        interactiveElements,
        textElements,
    };
}
"""


def capture_viewport(page, name, vp, site_dir, truth_dir):
    """Capture ground truth at one viewport size."""
    label = vp["label"]
    w, h = vp["width"], vp["height"]

    page.set_viewport_size({"width": w, "height": h})
    page.wait_for_timeout(1500)  # let responsive layouts settle

    # Determine scroll positions to test fixed/sticky behavior
    scroll_height = page.evaluate("document.documentElement.scrollHeight")
    scroll_positions = [0]
    if scroll_height > h:
        # Sample: top, 25%, 50%, 75%, bottom
        for pct in [0.25, 0.5, 0.75]:
            scroll_positions.append(int(scroll_height * pct))
        scroll_positions.append(scroll_height - h)

    # Extract ground truth
    report = page.evaluate(EXTRACTOR_JS, scroll_positions)

    # Full-page screenshot
    screenshot_path = site_dir / f"screenshot-{label}.png"
    page.screenshot(path=str(screenshot_path), full_page=True)

    # Viewport screenshot at scroll=0
    page.evaluate("window.scrollTo(0, 0)")
    page.wait_for_timeout(300)
    viewport_path = site_dir / f"viewport-{label}.png"
    page.screenshot(path=str(viewport_path), full_page=False)

    # Scroll to bottom and capture
    if scroll_height > h:
        page.evaluate(f"window.scrollTo(0, {scroll_height - h})")
        page.wait_for_timeout(500)
        bottom_path = site_dir / f"viewport-{label}-bottom.png"
        page.screenshot(path=str(bottom_path), full_page=False)

    # Save ground truth
    report["viewportLabel"] = label
    truth_path = truth_dir / f"{name}-{label}.json"
    truth_path.write_text(json.dumps(report, indent=2))

    size_kb = truth_path.stat().st_size // 1024
    print(f"  [{label}] {w}x{h}: {report['elementCount']} elements, "
          f"scroll={report['pageScroll']['scrollHeight']}px, "
          f"{len(report['scrollContainers'])} scroll containers, "
          f"{len(report['fixedStickyElements'])} fixed/sticky, "
          f"{len(report['images'])} images, "
          f"{len(report['interactiveElements'])} interactive, "
          f"{len(report['textElements'])} multi-line text blocks, "
          f"{len(report['stackingContexts'])} stacking contexts, "
          f"({size_kb} KB)")

    return report


def main():
    if len(sys.argv) < 3:
        print(f"Usage: {sys.argv[0]} <url> <site-name>", file=sys.stderr)
        return 1

    url = sys.argv[1]
    name = sys.argv[2]

    site_dir = SITES_DIR / name
    site_dir.mkdir(parents=True, exist_ok=True)
    TRUTH_DIR.mkdir(parents=True, exist_ok=True)

    print(f"Capturing {url} as '{name}'...")

    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True)
        ctx = browser.new_context(
            viewport={"width": VIEWPORTS[0]["width"], "height": VIEWPORTS[0]["height"]},
            device_scale_factor=1,
            # Accept cookies/dismiss banners more reliably
            locale="en-US",
            timezone_id="America/Los_Angeles",
        )
        page = ctx.new_page()

        # Navigate
        page.goto(url, wait_until="networkidle", timeout=45000)
        page.wait_for_timeout(3000)

        # Try to dismiss cookie banners / consent dialogs
        for selector in [
            'button:has-text("Accept")', 'button:has-text("Got it")',
            'button:has-text("OK")', 'button:has-text("Close")',
            'button:has-text("Agree")', '[aria-label="Close"]',
            'button:has-text("Accept all")', 'button:has-text("Allow")',
        ]:
            try:
                btn = page.locator(selector).first
                if btn.is_visible(timeout=500):
                    btn.click(timeout=1000)
                    page.wait_for_timeout(500)
                    break
            except Exception:
                pass

        # Save MHTML for archival
        mhtml_path = site_dir / "page.mhtml"
        try:
            cdp = ctx.new_cdp_session(page)
            result = cdp.send("Page.captureSnapshot", {"format": "mhtml"})
            mhtml_path.write_text(result["data"])
            print(f"  MHTML: {mhtml_path.stat().st_size // 1024} KB")
        except Exception as e:
            print(f"  MHTML capture failed: {e}")

        # Capture at each viewport size
        summaries = []
        for vp in VIEWPORTS:
            report = capture_viewport(page, name, vp, site_dir, TRUTH_DIR)
            summaries.append({
                "viewport": vp,
                "elementCount": report["elementCount"],
                "scrollHeight": report["pageScroll"]["scrollHeight"],
                "scrollContainers": len(report["scrollContainers"]),
                "fixedSticky": len(report["fixedStickyElements"]),
                "images": len(report["images"]),
                "interactive": len(report["interactiveElements"]),
                "textBlocks": len(report["textElements"]),
                "stackingContexts": len(report["stackingContexts"]),
            })

        # Save site metadata
        meta = {
            "name": name,
            "url": url,
            "capturedAt": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
            "viewports": summaries,
        }
        (site_dir / "meta.json").write_text(json.dumps(meta, indent=2))

        browser.close()

    print(f"\nDone: {name}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
