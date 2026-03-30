// ═══════════════════════════════════════════════════════════════
// Window Shim — getComputedStyle, matchMedia, scrollTo, etc.
// ═══════════════════════════════════════════════════════════════
//
// Provides the window-level APIs that react-dom and Vue expect.
// On web these are real browser APIs. On native they're shimmed.

import { ShimEvent } from './events.js';

export function createWindowShim(shimDocument) {
    // When running in browser for the shim demo,
    // we enhance the real window with getComputedStyle that works on shim nodes

    const shimGetComputedStyle = (element) => {
        // Return the element's style proxy as "computed" style
        // Real computed style resolves inheritance/cascade —
        // we just return the directly-set values for now
        if (element && element.style) {
            return element.style;
        }
        // Fallback: return empty style
        return new Proxy({}, {
            get: () => '',
            set: () => true,
        });
    };

    const shimMatchMedia = (query) => {
        // Simplified media query matching
        const matches = _evalMediaQuery(query);
        return {
            matches,
            media: query,
            addEventListener: () => {},
            removeEventListener: () => {},
            addListener: () => {},
            removeListener: () => {},
            onchange: null,
        };
    };

    const shimScrollTo = (x, y) => {
        // Would scroll the engine camera
    };

    return {
        getComputedStyle: shimGetComputedStyle,
        matchMedia: shimMatchMedia,
        scrollTo: shimScrollTo,
        scroll: shimScrollTo,
        scrollBy: (x, y) => {},
        getSelection: () => ({
            removeAllRanges: () => {},
            addRange: () => {},
            getRangeAt: () => ({ commonAncestorContainer: shimDocument.body }),
            rangeCount: 0,
        }),
        // Properties
        innerWidth: typeof window !== 'undefined' ? window.innerWidth : 1280,
        innerHeight: typeof window !== 'undefined' ? window.innerHeight : 800,
        outerWidth: typeof window !== 'undefined' ? window.outerWidth : 1280,
        outerHeight: typeof window !== 'undefined' ? window.outerHeight : 800,
        devicePixelRatio: typeof window !== 'undefined' ? window.devicePixelRatio : 2,
        navigator: typeof navigator !== 'undefined' ? navigator : {
            userAgent: 'Rendero/1.0',
            platform: 'Rendero',
            language: 'en-US',
            languages: ['en-US'],
        },
        location: typeof location !== 'undefined' ? location : {
            href: '', hostname: '', pathname: '/', protocol: 'https:', hash: '',
        },
        history: typeof history !== 'undefined' ? history : {
            pushState: () => {}, replaceState: () => {}, back: () => {}, forward: () => {},
        },
        // Events
        Event: ShimEvent,
        CustomEvent: ShimEvent,
    };
}

function _evalMediaQuery(query) {
    const w = typeof window !== 'undefined' ? window.innerWidth : 1280;
    // (prefers-color-scheme: dark)
    if (query.includes('prefers-color-scheme: dark')) return false;
    if (query.includes('prefers-color-scheme: light')) return true;
    // (min-width: Xpx)
    const minW = query.match(/min-width:\s*(\d+)px/);
    if (minW) return w >= parseInt(minW[1]);
    // (max-width: Xpx)
    const maxW = query.match(/max-width:\s*(\d+)px/);
    if (maxW) return w <= parseInt(maxW[1]);
    return false;
}
