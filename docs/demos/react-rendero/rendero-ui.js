// ═══════════════════════════════════════════════════════════════════
// STEP 1 & 2: The "Native Side" — React Native's UIManager + Yoga
// ═══════════════════════════════════════════════════════════════════
//
// In React Native, this layer is written in Objective-C (iOS) or
// Java (Android). It creates real platform views (UIView, android.View).
//
// Here, we create Rendero engine nodes instead. Same concept:
// imperative CRUD operations on a tree of visual elements.
//
// React never touches this directly — the Reconciler (renderer.js)
// translates React's declarative output into these imperative calls.

import init, { CanvasEngine } from '../../pkg/rendero.js';

export class RenderoUI {
    constructor() {
        this.engine = null;
        this.canvas = null;
        this.ctx = null;
        this.nodes = new Map();     // nodeKey → { counter, clientId, type, props, parent }
        this.inputs = new Map();    // nodeKey → { el: HTMLInputElement, frameKey, textKey }
        this.nextKey = 1;
        this.opsLog = [];           // For debug display
        this._raf = null;
        this._dirty = true;
        this._scrollY = 0;      // Current scroll offset (world units)
        this._scrollMax = 0;    // Max scroll (set by content height)
        this._touchStartY = 0;
        this._touchStartScroll = 0;
        this._touchMoved = false;
    }

    async init(canvas) {
        await init();
        this.canvas = canvas;
        this.ctx = canvas.getContext('2d');
        this.engine = new CanvasEngine("ReactRendero", 1);
        this._resize();
        window.addEventListener('resize', () => this._resize());
        this._setupScroll();
        this._startRenderLoop();
        return this;
    }

    _resize() {
        const dpr = window.devicePixelRatio || 1;
        this.canvas.width = window.innerWidth * dpr;
        this.canvas.height = window.innerHeight * dpr;
        this.engine.set_viewport(this.canvas.width, this.canvas.height);
        // Position camera so (0,0) is top-left of screen
        this.engine.set_camera(0, 0, dpr);
        this._dirty = true;
    }

    // ─── STEP 1: View Creation (UIManager.createView) ───

    createView(type, props = {}) {
        const key = this.nextKey++;
        const { x = 0, y = 0, width = 0, height = 0 } = props;
        const [r, g, b, a] = this._parseColor(props.backgroundColor || 'transparent');

        // Normalize: lowercase 'frame' and uppercase 'Frame' both work
        const t = type.charAt(0).toUpperCase() + type.slice(1);

        let counter, clientId;

        if (t === 'Frame') {
            const ids = this.engine.add_frame(`frame_${key}`, x, y, width || 100, height || 100, r, g, b, a);
            [counter, clientId] = ids;
        } else if (t === 'Text') {
            const [tr, tg, tb, ta] = this._parseColor(props.color || '#000000');
            const ids = this.engine.add_text(`text_${key}`, props.text || '', x, y, props.fontSize || 16, tr, tg, tb, ta);
            [counter, clientId] = ids;
        } else if (t === 'Textinput' || t === 'TextInput') {
            // ─── TextInput: Frame (border) + Text (value) + hidden HTML <input> ───
            // This is EXACTLY how React Native's TextInput works:
            // - iOS: invisible UITextField captures keyboard input
            // - Android: invisible EditText captures keyboard input
            // - Here: invisible HTML <input> captures keyboard input
            // The Rendero engine draws the visual frame + text on canvas.
            // The platform input handles IME, cursor, selection, autocomplete.
            const ids = this.engine.add_frame(`input_frame_${key}`, x, y, width || 260, height || 44, r || 1, g || 1, b || 1, a || 1);
            [counter, clientId] = ids;

            // Add text child inside the frame
            this.engine.set_insert_parent(counter, clientId);
            const [tc, tci] = this._parseColor(props.color || '#000000');
            const textIds = this.engine.add_text(
                `input_text_${key}`,
                props.value || props.placeholder || '',
                0, 0,
                props.fontSize || 15,
                ...this._parseColor(
                    (props.value ? props.color : props.placeholderColor) || '#000000'
                )
            );
            this.engine.clear_insert_parent();

            // Create hidden HTML <input> positioned over the canvas location
            const inputEl = document.createElement('input');
            inputEl.type = props.inputType || 'text';
            inputEl.value = props.value || '';
            inputEl.placeholder = props.placeholder || '';
            if (props.inputType === 'number') inputEl.inputMode = 'numeric';
            if (props.inputType === 'email') inputEl.inputMode = 'email';
            if (props.inputType === 'tel') inputEl.inputMode = 'tel';
            if (props.secureTextEntry) inputEl.type = 'password';
            if (props.maxLength) inputEl.maxLength = props.maxLength;

            // Position the real HTML input OVER the canvas-rendered input.
            // This is exactly how React Native works on iOS:
            // A real UITextField sits on top of the Skia canvas at the exact
            // position of the visual TextInput. The OS handles focus, cursor,
            // keyboard, IME, autocomplete — we just sync the value via onChange.
            Object.assign(inputEl.style, {
                position: 'fixed',
                background: 'transparent',
                border: 'none',
                outline: 'none',
                color: 'transparent',       // Text invisible (canvas draws it)
                caretColor: '#007AFF',      // But caret IS visible
                fontSize: '16px',           // Prevents iOS zoom on focus
                fontFamily: 'monospace',
                padding: '12px',
                zIndex: '100',
                // Will be positioned by _syncInputPositions()
                left: '-9999px', top: '-9999px',
                width: '1px', height: '1px',
            });
            document.body.appendChild(inputEl);

            this.inputs.set(key, {
                el: inputEl,
                textNodeId: textIds,
                onChange: props.onChange || null,
                onSubmit: props.onSubmit || null,
                placeholder: props.placeholder || '',
                placeholderColor: props.placeholderColor || '#999999',
                valueColor: props.color || '#000000',
            });

            // Wire up input events — this is the bridge between platform and React
            inputEl.addEventListener('input', () => {
                const info = this.inputs.get(key);
                if (info && info.onChange) {
                    info.onChange(inputEl.value);
                }
            });
            inputEl.addEventListener('keydown', (e) => {
                if (e.key === 'Enter') {
                    const info = this.inputs.get(key);
                    if (info && info.onSubmit) info.onSubmit(inputEl.value);
                }
            });
            inputEl.addEventListener('focus', () => { this._dirty = true; });
            inputEl.addEventListener('blur', () => { this._dirty = true; });

        } else if (t === 'Rect') {
            const ids = this.engine.add_rectangle(`rect_${key}`, x, y, width || 100, height || 100, r, g, b, a);
            [counter, clientId] = ids;
        } else {
            // Default to Frame
            const ids = this.engine.add_frame(`view_${key}`, x, y, width || 100, height || 100, r, g, b, a);
            [counter, clientId] = ids;
        }

        const node = { key, counter, clientId, type: t, props: { ...props }, parent: null };
        this.nodes.set(key, node);

        // Apply non-creation props
        this._applyProps(node, props);

        this._logOp('CREATE', `${type}#${key}`, props);
        this._dirty = true;
        return key;
    }

    // ─── STEP 1: Tree Manipulation (UIManager.setChildren) ───

    appendChild(parentKey, childKey) {
        const parent = this.nodes.get(parentKey);
        const child = this.nodes.get(childKey);
        if (!parent || !child) return;

        // In Rendero, set_insert_parent must be called BEFORE creating the child.
        // Since the child already exists, we need to use the engine's internal
        // move_node. But the WASM API doesn't expose reparent directly.
        // Workaround: we track parent in JS and recreate if needed.
        // For the POC, we set insert_parent before creation in createViewAsChild().
        child.parent = parentKey;
        this._dirty = true;
    }

    // Create a view directly as a child of parent (the correct way)
    createViewAsChild(parentKey, type, props = {}) {
        const parent = this.nodes.get(parentKey);
        if (!parent) return this.createView(type, props);

        // Tell engine: next created node goes inside this parent
        this.engine.set_insert_parent(parent.counter, parent.clientId);
        const key = this.createView(type, props);
        this.engine.clear_insert_parent();

        const child = this.nodes.get(key);
        child.parent = parentKey;
        return key;
    }

    // ─── STEP 1: Property Updates (UIManager.updateView) ───

    updateProps(key, newProps) {
        const node = this.nodes.get(key);
        if (!node) return;

        const changes = {};
        for (const [k, v] of Object.entries(newProps)) {
            if (node.props[k] !== v) {
                changes[k] = v;
                node.props[k] = v;
            }
        }

        if (Object.keys(changes).length === 0) return;

        this._applyProps(node, changes);
        this._logOp('UPDATE', `${node.type}#${key}`, changes);
        this._dirty = true;
    }

    // ─── STEP 1: Node Removal ───

    removeView(key) {
        const node = this.nodes.get(key);
        if (!node) return;
        // Clean up HTML input if this is a TextInput
        if (this.inputs.has(key)) {
            const info = this.inputs.get(key);
            info.el.remove();
            this.inputs.delete(key);
        }
        this.engine.select_node(node.counter, node.clientId);
        this.engine.delete_selected();
        this.nodes.delete(key);
        this._logOp('REMOVE', `${node.type}#${key}`);
        this._dirty = true;
    }

    // Check if a hit-tested node key belongs to any TextInput
    // Returns the TextInput's key, or null
    getInputAtKey(hitKey) {
        // Direct match: the frame itself was hit
        if (this.inputs.has(hitKey)) return hitKey;
        // Check if hitKey is the internal text child of any TextInput
        for (const [inputKey, info] of this.inputs) {
            const [tc, tci] = info.textNodeId;
            // Find the text child's key by matching counter/clientId
            for (const [nodeKey, node] of this.nodes) {
                if (node.counter === tc && node.clientId === tci && nodeKey === hitKey) {
                    return inputKey;
                }
            }
        }
        return null;
    }

    // Focus a TextInput's hidden HTML element (triggers keyboard on mobile)
    focusInput(key) {
        const info = this.inputs.get(key);
        if (!info) return;
        info.el.focus();
    }

    // Position all HTML <input> elements over their canvas counterparts.
    // Called every frame in the render loop.
    // This is how React Native positions native views over the canvas.
    _syncInputPositions() {
        const cam = this.engine.get_camera();
        const [camX, camY, zoom] = cam;

        for (const [key, info] of this.inputs) {
            const node = this.nodes.get(key);
            if (!node) continue;

            const bounds = this.engine.get_node_world_bounds(node.counter, node.clientId);
            if (!bounds || bounds.length < 4) continue;

            const [wx, wy, ww, wh] = bounds;
            // World → screen coordinates
            const sx = (wx - camX) * zoom;
            const sy = (wy - camY) * zoom;
            const sw = ww * zoom;
            const sh = wh * zoom;

            Object.assign(info.el.style, {
                left: sx + 'px',
                top: sy + 'px',
                width: sw + 'px',
                height: sh + 'px',
            });
        }
    }

    // ─── STEP 2: Layout Props (Yoga) ───

    _applyProps(node, props) {
        const { counter, clientId } = node;

        // Position
        if ('x' in props || 'y' in props) {
            this.engine.set_node_position(counter, clientId,
                props.x ?? node.props.x ?? 0,
                props.y ?? node.props.y ?? 0);
        }

        // Size
        if ('width' in props || 'height' in props) {
            this.engine.set_node_size(counter, clientId,
                props.width ?? node.props.width ?? 100,
                props.height ?? node.props.height ?? 100);
        }

        // Background color
        if ('backgroundColor' in props) {
            const [r, g, b, a] = this._parseColor(props.backgroundColor);
            this.engine.set_node_fill(counter, clientId, r, g, b, a);
        }

        // Corner radius
        if ('borderRadius' in props) {
            const r = props.borderRadius;
            this.engine.set_node_corner_radius(counter, clientId, r, r, r, r);
        }

        // Opacity
        if ('opacity' in props) {
            this.engine.set_node_opacity(counter, clientId, props.opacity);
        }

        // ─── TextInput value sync ───
        if ((node.type === 'TextInput' || node.type === 'Textinput') && this.inputs.has(node.key)) {
            const info = this.inputs.get(node.key);
            const [tc, tci] = info.textNodeId;

            if ('value' in props) {
                // Update the hidden HTML input
                info.el.value = props.value;
                // Update the display text on canvas
                const displayText = props.value || info.placeholder;
                this.engine.set_node_text(tc, tci, displayText);
                // Set color based on whether it's placeholder or real text
                const color = props.value ? info.valueColor : info.placeholderColor;
                const [cr, cg, cb, ca] = this._parseColor(color);
                this.engine.set_node_fill(tc, tci, cr, cg, cb, ca);
            }
            if ('onChange' in props) info.onChange = props.onChange;
            if ('onSubmit' in props) info.onSubmit = props.onSubmit;
        }

        // ─── Text props ───
        if ('text' in props) {
            this.engine.set_node_text(counter, clientId, props.text);
        }
        if ('fontSize' in props) {
            this.engine.set_node_font_size(counter, clientId, props.fontSize);
        }
        if ('fontWeight' in props) {
            this.engine.set_node_font_weight(counter, clientId, props.fontWeight);
        }
        if ('color' in props && node.type.toLowerCase() === 'text') {
            const [r, g, b, a] = this._parseColor(props.color);
            this.engine.set_node_fill(counter, clientId, r, g, b, a);
        }
        if ('textAlign' in props) {
            this.engine.set_text_align(counter, clientId, props.textAlign);
        }

        // ─── STEP 2: Auto-layout (Yoga equivalent) ───
        if ('flexDirection' in props || 'gap' in props || 'padding' in props ||
            'paddingHorizontal' in props || 'paddingVertical' in props) {
            const dir = (props.flexDirection ?? node.props.flexDirection) === 'row' ? 0 : 1;
            const gap = props.gap ?? node.props.gap ?? 0;
            const padH = props.paddingHorizontal ?? props.padding ?? node.props.paddingHorizontal ?? node.props.padding ?? 0;
            const padV = props.paddingVertical ?? props.padding ?? node.props.paddingVertical ?? node.props.padding ?? 0;
            const padT = props.paddingTop ?? padV;
            const padR = props.paddingRight ?? padH;
            const padB = props.paddingBottom ?? padV;
            const padL = props.paddingLeft ?? padH;

            this.engine.set_auto_layout(counter, clientId, dir, gap, padT, padR, padB, padL);
        }

        // Store onClick flag for cursor detection
        if ('onClick' in props) {
            node.props.onClick = props.onClick;
        }
    }

    // ─── Scrolling (ScrollView equivalent) ───
    // React Native's ScrollView wraps content and handles touch/wheel scrolling.
    // Here we scroll by adjusting the engine camera's Y offset.

    _setupScroll() {
        const canvas = this.canvas;

        // Cursor: pointer over clickable elements
        canvas.addEventListener('mousemove', (e) => {
            const hitKey = this.getNodeAtPoint(e.clientX, e.clientY);
            if (hitKey && this.inputs.has(hitKey)) {
                canvas.style.cursor = 'text';
            } else if (hitKey) {
                // Check if any node in the hit chain has an onClick
                const node = this.nodes.get(hitKey);
                canvas.style.cursor = (node && node.props.onClick) ? 'pointer' : 'default';
            } else {
                canvas.style.cursor = 'default';
            }
        });

        // Mouse wheel → scroll
        canvas.addEventListener('wheel', (e) => {
            e.preventDefault();
            const dpr = window.devicePixelRatio || 1;
            this._scrollY += e.deltaY / dpr;
            this._clampScroll();
            this._applyScroll();
        }, { passive: false });

        // Touch drag → scroll (distinguishes tap vs drag)
        canvas.addEventListener('touchstart', (e) => {
            if (e.touches.length === 1) {
                this._touchStartY = e.touches[0].clientY;
                this._touchStartScroll = this._scrollY;
                this._touchMoved = false;
            }
        }, { passive: true });

        canvas.addEventListener('touchmove', (e) => {
            if (e.touches.length === 1) {
                const dy = this._touchStartY - e.touches[0].clientY;
                const dpr = window.devicePixelRatio || 1;
                if (Math.abs(dy) > 5) {
                    this._touchMoved = true;
                    this._scrollY = this._touchStartScroll + dy / dpr;
                    this._clampScroll();
                    this._applyScroll();
                    e.preventDefault();
                }
            }
        }, { passive: false });
    }

    _clampScroll() {
        this._scrollY = Math.max(0, Math.min(this._scrollMax, this._scrollY));
    }

    _applyScroll() {
        const dpr = window.devicePixelRatio || 1;
        this.engine.set_camera(0, this._scrollY, dpr);
        this._dirty = true;
    }

    setScrollableHeight(contentHeight) {
        const dpr = window.devicePixelRatio || 1;
        const viewportHeight = window.innerHeight / dpr;
        this._scrollMax = Math.max(0, contentHeight - viewportHeight + 40);
    }

    // Was the last touch a drag (scroll) rather than a tap?
    wasTouchScroll() {
        return this._touchMoved;
    }

    // ─── Render Loop ───

    _startRenderLoop() {
        const loop = () => {
            if (this._dirty) {
                this._dirty = false;
                const dpr = window.devicePixelRatio || 1;
                this.engine.render_canvas2d(this.ctx, this.canvas.width, this.canvas.height, dpr);
            }
            // Keep HTML inputs positioned over their canvas counterparts
            this._syncInputPositions();
            this._raf = requestAnimationFrame(loop);
        };
        loop();
    }

    // ─── Helpers ───

    _parseColor(color) {
        if (color === 'transparent') return [0, 0, 0, 0];
        if (color.startsWith('#')) {
            const hex = color.slice(1);
            if (hex.length === 3) {
                const r = parseInt(hex[0] + hex[0], 16) / 255;
                const g = parseInt(hex[1] + hex[1], 16) / 255;
                const b = parseInt(hex[2] + hex[2], 16) / 255;
                return [r, g, b, 1];
            }
            const r = parseInt(hex.slice(0, 2), 16) / 255;
            const g = parseInt(hex.slice(2, 4), 16) / 255;
            const b = parseInt(hex.slice(4, 6), 16) / 255;
            const a = hex.length === 8 ? parseInt(hex.slice(6, 8), 16) / 255 : 1;
            return [r, g, b, a];
        }
        return [0, 0, 0, 1];
    }

    _logOp(type, target, data) {
        this.opsLog.push({ type, target, data, time: performance.now() });
        // Keep last 20
        if (this.opsLog.length > 20) this.opsLog.shift();
    }

    getOpsLog() {
        return this.opsLog;
    }

    // ─── Hit Testing (for event system) ───

    getNodeAtPoint(screenX, screenY) {
        // Convert screen to world coordinates
        const cam = this.engine.get_camera();
        const [camX, camY, zoom] = cam;
        const worldX = screenX / zoom + camX;
        const worldY = screenY / zoom + camY;

        // Check all nodes in reverse order (topmost first)
        let hit = null;
        for (const [key, node] of this.nodes) {
            const bounds = this.engine.get_node_world_bounds(node.counter, node.clientId);
            if (bounds && bounds.length === 4) {
                const [nx, ny, nw, nh] = bounds;
                if (worldX >= nx && worldX <= nx + nw && worldY >= ny && worldY <= ny + nh) {
                    hit = key; // Last match = topmost (nodes are in creation order)
                }
            }
        }
        return hit;
    }
}
