// ═══════════════════════════════════════════════════════════════
// Ground Truth Extractor
// ═══════════════════════════════════════════════════════════════
//
// Inject this into ANY web page to extract layout ground truth.
// Walks every element in the DOM, captures:
//   - Computed bounds (x, y, width, height)
//   - Computed styles (every CSS property the browser resolved)
//   - Element identity (tag, classes, id, text content)
//   - Parent-child relationships
//
// Output: JSON array of element records.
// Usage: paste into browser console, or load via script tag.
//
// This is the ORACLE. The browser is always right.
// Our engine's job is to match these numbers.

(function extractGroundTruth() {
    const STYLE_PROPERTIES = [
        // Layout
        'display', 'position', 'top', 'right', 'bottom', 'left',
        'width', 'height', 'minWidth', 'minHeight', 'maxWidth', 'maxHeight',
        'overflow', 'overflowX', 'overflowY', 'boxSizing',
        // Flexbox
        'flexDirection', 'flexWrap', 'justifyContent', 'alignItems', 'alignContent',
        'flexGrow', 'flexShrink', 'flexBasis', 'alignSelf', 'order',
        'gap', 'rowGap', 'columnGap',
        // Spacing
        'marginTop', 'marginRight', 'marginBottom', 'marginLeft',
        'paddingTop', 'paddingRight', 'paddingBottom', 'paddingLeft',
        // Border
        'borderTopWidth', 'borderRightWidth', 'borderBottomWidth', 'borderLeftWidth',
        'borderTopStyle', 'borderRightStyle', 'borderBottomStyle', 'borderLeftStyle',
        'borderTopColor', 'borderRightColor', 'borderBottomColor', 'borderLeftColor',
        'borderRadius', 'borderTopLeftRadius', 'borderTopRightRadius',
        'borderBottomRightRadius', 'borderBottomLeftRadius',
        // Visual
        'backgroundColor', 'color', 'opacity',
        'backgroundImage', 'backgroundSize', 'backgroundPosition',
        // Text
        'fontSize', 'fontWeight', 'fontFamily', 'fontStyle',
        'lineHeight', 'textAlign', 'textDecoration', 'letterSpacing', 'wordSpacing',
        'whiteSpace', 'textOverflow', 'wordBreak', 'overflowWrap',
        // Transform
        'transform', 'transformOrigin',
        // Other
        'zIndex', 'visibility', 'cursor', 'pointerEvents',
        'boxShadow', 'textShadow',
    ];

    const elements = [];
    let nextId = 0;
    const idMap = new WeakMap();

    function assignId(el) {
        if (!idMap.has(el)) {
            idMap.set(el, nextId++);
        }
        return idMap.get(el);
    }

    function getTextContent(el) {
        // Direct text content only (not children's text)
        let text = '';
        for (const child of el.childNodes) {
            if (child.nodeType === Node.TEXT_NODE) {
                text += child.textContent;
            }
        }
        return text.trim().slice(0, 200); // Cap at 200 chars
    }

    function walk(el, depth) {
        if (el.nodeType !== Node.ELEMENT_NODE) return;

        // Skip invisible elements (script, style, meta, etc.)
        const tag = el.tagName.toLowerCase();
        if (['script', 'style', 'meta', 'link', 'head', 'noscript', 'template'].includes(tag)) return;

        const id = assignId(el);
        const parentId = el.parentElement ? (idMap.has(el.parentElement) ? idMap.get(el.parentElement) : null) : null;

        // Bounds relative to viewport
        const rect = el.getBoundingClientRect();
        const bounds = {
            x: Math.round(rect.left * 100) / 100,
            y: Math.round(rect.top * 100) / 100,
            width: Math.round(rect.width * 100) / 100,
            height: Math.round(rect.height * 100) / 100,
        };

        // Computed styles
        const computed = window.getComputedStyle(el);
        const styles = {};
        for (const prop of STYLE_PROPERTIES) {
            const value = computed.getPropertyValue(
                prop.replace(/([A-Z])/g, '-$1').toLowerCase()
            );
            if (value && value !== 'none' && value !== 'normal' && value !== 'auto'
                && value !== '0px' && value !== '0' && value !== 'rgba(0, 0, 0, 0)'
                && value !== 'visible' && value !== 'static' && value !== 'content-box'
                && value !== 'baseline' && value !== 'row' && value !== 'nowrap'
                && value !== 'stretch' && value !== 'start') {
                styles[prop] = value;
            }
        }

        // Always capture these even if "default"
        styles.display = computed.display;
        styles.position = computed.position;
        styles.boxSizing = computed.boxSizing;

        const record = {
            id,
            parentId,
            depth,
            tag,
            htmlId: el.id || null,
            className: el.className ? String(el.className).slice(0, 100) : null,
            text: getTextContent(el),
            bounds,
            styles,
            childCount: el.children.length,
        };

        elements.push(record);

        // Recurse children
        for (const child of el.children) {
            walk(child, depth + 1);
        }
    }

    // Start from body
    walk(document.body, 0);

    const report = {
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

    // Make available globally
    window.__GROUND_TRUTH__ = report;

    // Log summary
    console.log(`[Ground Truth] Extracted ${elements.length} elements from ${location.href}`);
    console.log(`[Ground Truth] Viewport: ${report.viewport.width}x${report.viewport.height} @${report.viewport.dpr}x`);
    console.log(`[Ground Truth] Scroll height: ${report.viewport.scrollHeight}px`);
    console.log(`[Ground Truth] Access via window.__GROUND_TRUTH__`);

    return report;
})();
