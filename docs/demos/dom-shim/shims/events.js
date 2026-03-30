// ═══════════════════════════════════════════════════════════════
// Event System — DOM Events shimmed over Rendero hit testing
// ═══════════════════════════════════════════════════════════════
//
// Implements: Event, MouseEvent, addEventListener, removeEventListener,
// dispatchEvent, event bubbling up the shim node tree.

export class ShimEvent {
    constructor(type, opts = {}) {
        this.type = type;
        this.bubbles = opts.bubbles !== undefined ? opts.bubbles : true;
        this.cancelable = opts.cancelable !== undefined ? opts.cancelable : true;
        this.target = null;
        this.currentTarget = null;
        this.defaultPrevented = false;
        this._stopped = false;
        this._immediateStopped = false;
        this.eventPhase = 0;
        this.timeStamp = performance.now();
    }
    preventDefault() { this.defaultPrevented = true; }
    stopPropagation() { this._stopped = true; }
    stopImmediatePropagation() { this._stopped = true; this._immediateStopped = true; }
}

export class ShimMouseEvent extends ShimEvent {
    constructor(type, opts = {}) {
        super(type, opts);
        this.clientX = opts.clientX || 0;
        this.clientY = opts.clientY || 0;
        this.pageX = opts.pageX || this.clientX;
        this.pageY = opts.pageY || this.clientY;
        this.screenX = opts.screenX || this.clientX;
        this.screenY = opts.screenY || this.clientY;
        this.button = opts.button || 0;
        this.buttons = opts.buttons || 0;
        this.ctrlKey = opts.ctrlKey || false;
        this.shiftKey = opts.shiftKey || false;
        this.altKey = opts.altKey || false;
        this.metaKey = opts.metaKey || false;
    }
}

export class ShimKeyboardEvent extends ShimEvent {
    constructor(type, opts = {}) {
        super(type, opts);
        this.key = opts.key || '';
        this.code = opts.code || '';
        this.ctrlKey = opts.ctrlKey || false;
        this.shiftKey = opts.shiftKey || false;
        this.altKey = opts.altKey || false;
        this.metaKey = opts.metaKey || false;
        this.repeat = opts.repeat || false;
    }
}

export class ShimFocusEvent extends ShimEvent {
    constructor(type, opts = {}) {
        super(type, opts);
        this.relatedTarget = opts.relatedTarget || null;
    }
}

export class ShimInputEvent extends ShimEvent {
    constructor(type, opts = {}) {
        super(type, opts);
        this.data = opts.data || '';
        this.inputType = opts.inputType || '';
    }
}

// EventTarget mixin — added to every ShimNode
export const EventTargetMixin = {
    _initEvents() {
        this._listeners = new Map();
    },

    addEventListener(type, handler, options) {
        if (!handler) return;
        if (!this._listeners.has(type)) {
            this._listeners.set(type, []);
        }
        const list = this._listeners.get(type);
        // Avoid duplicates
        const capture = typeof options === 'object' ? !!options.capture : !!options;
        if (!list.some(l => l.handler === handler && l.capture === capture)) {
            list.push({
                handler,
                capture,
                once: typeof options === 'object' ? !!options.once : false,
                passive: typeof options === 'object' ? !!options.passive : false,
            });
        }
    },

    removeEventListener(type, handler, options) {
        const list = this._listeners.get(type);
        if (!list) return;
        const capture = typeof options === 'object' ? !!options.capture : !!options;
        const idx = list.findIndex(l => l.handler === handler && l.capture === capture);
        if (idx !== -1) list.splice(idx, 1);
    },

    dispatchEvent(event) {
        event.target = this;

        // Build ancestor path for bubbling
        const path = [];
        let node = this;
        while (node) {
            path.push(node);
            node = node.parentNode;
        }

        // Capture phase (root → target)
        event.eventPhase = 1;
        for (let i = path.length - 1; i > 0; i--) {
            if (event._stopped) break;
            _fireListeners(path[i], event, true);
        }

        // Target phase
        if (!event._stopped) {
            event.eventPhase = 2;
            _fireListeners(this, event, false);
            _fireListeners(this, event, true);
        }

        // Bubble phase (target → root)
        if (event.bubbles && !event._stopped) {
            event.eventPhase = 3;
            for (let i = 1; i < path.length; i++) {
                if (event._stopped) break;
                _fireListeners(path[i], event, false);
            }
        }

        event.eventPhase = 0;
        return !event.defaultPrevented;
    },
};

function _fireListeners(node, event, capturePhase) {
    const list = node._listeners ? node._listeners.get(event.type) : null;
    if (!list) return;

    event.currentTarget = node;
    for (let i = 0; i < list.length; i++) {
        const entry = list[i];
        if (event._immediateStopped) break;

        // In target phase, fire both capture and bubble listeners
        if (event.eventPhase !== 2 && entry.capture !== capturePhase) continue;

        try {
            if (typeof entry.handler === 'function') {
                entry.handler.call(node, event);
            } else if (entry.handler && typeof entry.handler.handleEvent === 'function') {
                entry.handler.handleEvent(event);
            }
        } catch (e) {
            console.error('Event handler error:', e);
        }

        if (entry.once) {
            list.splice(i, 1);
            i--;
        }
    }
}
