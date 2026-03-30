// ═══════════════════════════════════════════════════════════════
// CSSStyleDeclaration — style.* property setters → Rendero engine
// ═══════════════════════════════════════════════════════════════
//
// When react-dom does: element.style.backgroundColor = '#fff'
// This intercepts it and calls engine.set_node_fill(...)
//
// Uses a Proxy so ANY style property triggers sync.

import { engineSetProp, markDirty, measureTextBrowser } from './engine-runtime.js';
import { parseColor, parseLength, parseFontWeight, parseBoxShadow, parseLinearGradient, expandShorthand } from './css-values.js';
import { buildAutoLayout, resolveMargins } from './layout-style.js';

export function createStyleProxy(element) {
    const _values = {};

    // Sync immediately — engineSetProp already queues ops for batch execution.
    // No need for microtask batching (which caused first-frame 0x0 nodes).
    function scheduleSync() {
        syncToEngine(element);
    }

    const handler = {
        get(target, prop) {
            if (prop === '_values') return _values;
            if (prop === '_syncNow') return () => syncToEngine(element);
            if (prop === 'cssText') return _toCssText(_values);
            if (prop === 'setProperty') return (name, val) => {
                const camel = cssToJs(name);
                _values[camel] = val;
                scheduleSync();
            };
            if (prop === 'getPropertyValue') return (name) => {
                return _values[cssToJs(name)] || '';
            };
            if (prop === 'removeProperty') return (name) => {
                const camel = cssToJs(name);
                const old = _values[camel];
                delete _values[camel];
                scheduleSync();
                return old || '';
            };
            if (prop === 'length') return Object.keys(_values).length;
            if (prop === 'item') return (idx) => Object.keys(_values)[idx] || '';
            if (typeof prop === 'symbol') return undefined;

            return _values[prop] || '';
        },
        set(target, prop, value) {
            if (typeof prop === 'symbol') return true;
            if (prop === 'cssText') {
                _parseCssText(value, _values);
                scheduleSync();
                return true;
            }
            // Expand shorthands
            const expanded = expandShorthand(prop, value);
            if (expanded) {
                Object.assign(_values, expanded);
            } else {
                _values[prop] = value;
            }
            scheduleSync();
            return true;
        },
    };

    return new Proxy({}, handler);
}

// Sync current style values to the Rendero engine
function syncToEngine(element) {
    const v = element.style._values;
    const id = element._engineId;
    const isText = element._isTextElement;
    const backgroundValue = v.backgroundImage || v.background || '';
    const backgroundColorValue = v.backgroundColor || v.background || '';

    // Width / Height → engine size
    // Resolve %, vh, vw to actual pixels
    let w = parseLength(v.width) || 0;
    let h = parseLength(v.height) || 0;

    if (typeof __rendero_log === 'function' && (w || h || v.backgroundColor || v.display)) {
        __rendero_log('SYNC id=' + id +
            ' w=' + w + '(' + (v.width||'') + ')' +
            ' h=' + h + '(' + (v.height||'') + ')' +
            ' bg=' + (v.backgroundColor||'') +
            ' display=' + (v.display||''));
    }
    const maxW = parseLength(v.maxWidth);
    const maxH = parseLength(v.maxHeight);
    const minW = parseLength(v.minWidth);
    const minH = parseLength(v.minHeight);
    const wantsFillX = v.flex === '1' || v.flex === '1 1 0' || v.flexGrow === '1' || v.flexGrow === 1;
    const hasExplicitWidth = !!w;
    const hasExplicitHeight = !!h;
    if (w || h) {
        engineSetProp(id, 'size', { w, h });
    }
    if (minW || minH || maxW || maxH) {
        engineSetProp(id, 'sizeConstraints', { minW: minW || 0, minH: minH || 0, maxW: maxW || 0, maxH: maxH || 0 });
    }
    if (wantsFillX) {
        engineSetProp(id, 'sizing', { horizontal: 2, vertical: 1 });
    }

    // Position (only absolute/fixed positioned)
    if (v.position === 'absolute' || v.position === 'fixed') {
        const x = parseLength(v.left) || 0;
        const y = parseLength(v.top) || 0;
        engineSetProp(id, 'layoutPosition', { x, y });
        engineSetProp(id, 'position', { x, y });
    }

    // Background
    if (backgroundColorValue) {
        const c = parseColor(backgroundColorValue);
        if (c) engineSetProp(id, 'fill', { r: c[0], g: c[1], b: c[2], a: c[3] });
    }
    if (backgroundValue) {
        const grad = parseLinearGradient(backgroundValue);
        if (grad) engineSetProp(id, 'linearGradient', grad);
    }

    // Border radius
    if (v.borderRadius) {
        const r = parseLength(v.borderRadius);
        engineSetProp(id, 'cornerRadius', { tl: r, tr: r, br: r, bl: r });
    }
    // Individual corners
    if (v.borderTopLeftRadius || v.borderTopRightRadius || v.borderBottomRightRadius || v.borderBottomLeftRadius) {
        engineSetProp(id, 'cornerRadius', {
            tl: parseLength(v.borderTopLeftRadius) || parseLength(v.borderRadius) || 0,
            tr: parseLength(v.borderTopRightRadius) || parseLength(v.borderRadius) || 0,
            br: parseLength(v.borderBottomRightRadius) || parseLength(v.borderRadius) || 0,
            bl: parseLength(v.borderBottomLeftRadius) || parseLength(v.borderRadius) || 0,
        });
    }

    // Opacity
    if (v.opacity !== undefined && v.opacity !== '') {
        engineSetProp(id, 'opacity', parseFloat(v.opacity));
    }

    const margins = resolveMargins(v);
    if (margins.top || margins.right || margins.bottom || margins.left) {
        engineSetProp(id, 'margin', margins);
    }

    // Layout mapping → engine auto-layout
    const autoLayout = buildAutoLayout(v, {
        isText,
        hasChildren: (element.childNodes?.length || 0) > 0,
    });
    if (autoLayout) {
        engineSetProp(id, 'sizing', {
            horizontal: wantsFillX ? 2 : (hasExplicitWidth ? 0 : 1),
            vertical: hasExplicitHeight ? 0 : 1,
        });
        engineSetProp(id, 'autoLayout', autoLayout);
    }

    // Text properties (for text elements)
    if (isText) {
        if (v.fontSize) engineSetProp(id, 'fontSize', parseLength(v.fontSize));
        if (v.fontWeight) engineSetProp(id, 'fontWeight', parseFontWeight(v.fontWeight));
        if (v.fontFamily) engineSetProp(id, 'fontFamily', v.fontFamily.replace(/['"]/g, ''));
        if (v.textAlign) engineSetProp(id, 'textAlign', v.textAlign);
        if (v.color) {
            const c = parseColor(v.color);
            if (c) engineSetProp(id, 'fill', { r: c[0], g: c[1], b: c[2], a: c[3] });
        }

        // Browser-accurate text measurement (oracle approach).
        // Use canvas.measureText() to get the real dimensions the browser would use,
        // then send as the node's size. This overrides the Rust heuristic.
        const textContent = element.textContent || '';
        if (textContent.trim()) {
            const measured = measureTextBrowser(
                textContent,
                parseLength(v.fontSize) || 16,
                v.fontWeight || '400',
                v.fontFamily || '',
            );
            if (measured) {
                // Use line-height if set, otherwise use measured height
                const lh = v.lineHeight ? parseLength(v.lineHeight) : 0;
                const textH = lh > 0 ? lh : measured.height;
                engineSetProp(id, 'size', { w: measured.width, h: textH });
            }
        }
    }

    // Box shadow
    if (v.boxShadow) {
        const s = parseBoxShadow(v.boxShadow);
        if (s) engineSetProp(id, 'shadow', s);
    }

    // Border (simplified — just stroke)
    if (v.borderWidth || v.border) {
        const borderStr = v.border || '';
        const bw = parseLength(v.borderWidth) || parseLength(borderStr.split(/\s+/)[0]) || 1;
        const bc = parseColor(v.borderColor) || parseColor(borderStr.split(/\s+/).slice(2).join(' ')) || [0, 0, 0, 1];
        engineSetProp(id, 'stroke', {
            r: bc[0], g: bc[1], b: bc[2], a: bc[3], weight: bw,
        });
    }

    // Transform: rotation
    if (v.transform) {
        const rotMatch = v.transform.match(/rotate\(([-\d.]+)deg\)/);
        if (rotMatch) engineSetProp(id, 'rotation', parseFloat(rotMatch[1]));
    }

    // Overflow
    if (v.overflow === 'hidden') {
        engineSetProp(id, 'clipContent', true);
    }

    markDirty();
}

// Convert CSS property name (kebab-case) to JS camelCase
function cssToJs(name) {
    return name.replace(/-([a-z])/g, (_, c) => c.toUpperCase());
}

function _toCssText(values) {
    return Object.entries(values)
        .map(([k, v]) => `${jsToCss(k)}: ${v}`)
        .join('; ');
}

function jsToCss(name) {
    return name.replace(/([A-Z])/g, '-$1').toLowerCase();
}

function _parseCssText(text, values) {
    // Clear existing
    for (const k of Object.keys(values)) delete values[k];
    if (!text) return;
    const rules = text.split(';').filter(s => s.trim());
    for (const rule of rules) {
        const [name, ...rest] = rule.split(':');
        if (name && rest.length) {
            const camel = cssToJs(name.trim());
            values[camel] = rest.join(':').trim();
        }
    }
}
