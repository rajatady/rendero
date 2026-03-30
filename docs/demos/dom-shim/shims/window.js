// ═══════════════════════════════════════════════════════════════
// Window Shim — getComputedStyle, matchMedia, scrollTo, etc.
// ═══════════════════════════════════════════════════════════════
//
// Provides the window-level APIs that react-dom and Vue expect.
// On web these are real browser APIs. On native they're shimmed.

import { ShimEvent } from './events.js';

function ensureScrollState() {
    if (!globalThis.__RENDERO_SCROLL_STATE__) {
        globalThis.__RENDERO_SCROLL_STATE__ = {
            x: 0,
            y: 0,
            contentHeight: 0,
            syncingViewport: false,
        };
    }
    return globalThis.__RENDERO_SCROLL_STATE__;
}

function getScrollMetrics(shimDocument) {
    const collectBottom = (node) => {
        if (!node) return 0;
        let maxBottom = 0;
        if (node._engineId != null) {
            const bounds = globalThis.Rendero?.engine?.getNodeBounds?.(node._engineId);
            if (bounds) {
                maxBottom = Math.max(maxBottom, (bounds.y || 0) + (bounds.height || 0));
            }
        }
        for (const child of node.childNodes || []) {
            maxBottom = Math.max(maxBottom, collectBottom(child));
        }
        return maxBottom;
    };
    const viewportHeight = globalThis.innerHeight || 800;
    const contentBottom = collectBottom(shimDocument?.body);
    const contentHeight = Math.max(viewportHeight, contentBottom);
    return { viewportHeight, contentHeight };
}

function clampScrollY(nextY, shimDocument) {
    const state = ensureScrollState();
    const metrics = getScrollMetrics(shimDocument);
    state.contentHeight = metrics.contentHeight;
    const maxY = Math.max(0, metrics.contentHeight - metrics.viewportHeight);
    return Math.max(0, Math.min(nextY, maxY));
}

function syncCameraToScroll(y) {
    const cam = globalThis.Rendero?.engine?.getCamera?.() || { x: 0, y: 0, zoom: 1 };
    globalThis.Rendero?.engine?.setCamera?.(cam.x || 0, y, cam.zoom || 1);
}

export function installWindowScrollShim(windowRef, shimDocument, { patchRealWindow = false } = {}) {
    const state = ensureScrollState();

    const updateMetrics = () => {
        const metrics = getScrollMetrics(shimDocument);
        state.contentHeight = metrics.contentHeight;
        if (patchRealWindow && typeof document !== 'undefined') {
            let spacer = document.getElementById('rendero-scroll-spacer');
            if (!spacer) {
                spacer = document.createElement('div');
                spacer.id = 'rendero-scroll-spacer';
                spacer.setAttribute('aria-hidden', 'true');
                document.body.appendChild(spacer);
            }
            spacer.style.height = `${metrics.contentHeight}px`;
            spacer.style.width = '1px';
            spacer.style.pointerEvents = 'none';
            spacer.style.opacity = '0';
        }
        return metrics;
    };

    const dispatchScroll = () => {
        if (typeof windowRef.dispatchEvent === 'function') {
            windowRef.dispatchEvent(new ShimEvent('scroll', { bubbles: false, cancelable: false }));
        } else if (typeof window !== 'undefined' && windowRef === window) {
            window.dispatchEvent(new Event('scroll'));
        }
    };

    if (patchRealWindow && typeof window !== 'undefined') {
        const syncFromViewport = () => {
            if (state.syncingViewport) return;
            updateMetrics();
            state.x = window.scrollX || 0;
            state.y = clampScrollY(window.scrollY || 0, shimDocument);
            syncCameraToScroll(state.y);
        };
        window.addEventListener('scroll', syncFromViewport, { passive: true });
        windowRef.__renderoUpdateScrollMetrics = updateMetrics;
        windowRef.__renderoSyncScrollFromViewport = syncFromViewport;
        updateMetrics();
        syncFromViewport();
        return;
    }

    const shimScrollTo = (x = 0, y = 0) => {
        updateMetrics();
        state.x = x || 0;
        state.y = clampScrollY(y || 0, shimDocument);
        syncCameraToScroll(state.y);
        dispatchScroll();
    };

    const shimScrollBy = (x = 0, y = 0) => {
        shimScrollTo(state.x + (x || 0), state.y + (y || 0));
    };

    Object.defineProperties(windowRef, {
        scrollX: { get: () => state.x, configurable: true },
        scrollY: { get: () => state.y, configurable: true },
        pageXOffset: { get: () => state.x, configurable: true },
        pageYOffset: { get: () => state.y, configurable: true },
    });
    windowRef.scrollTo = shimScrollTo;
    windowRef.scroll = shimScrollTo;
    windowRef.scrollBy = shimScrollBy;
    windowRef.__renderoUpdateScrollMetrics = updateMetrics;
    updateMetrics();
}

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

    return {
        getComputedStyle: shimGetComputedStyle,
        matchMedia: shimMatchMedia,
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
