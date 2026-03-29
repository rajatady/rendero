// ═══════════════════════════════════════════════════════════════════
// STEPS 3-6: The "React Side" — Components, Reconciler, Events
// ═══════════════════════════════════════════════════════════════════
//
// In React Native, this is the JavaScript thread running React.
// It produces a virtual tree, diffs it, and sends operations to
// the native side (rendero-ui.js).
//
// This file implements:
//   h()         — createElement: returns virtual node descriptions
//   useState()  — State hook: triggers re-render on change
//   reconcile() — Diffing: compares old vs new tree, emits minimal ops
//   mount()     — Entry point: renders a component tree into the engine

// ─── STEP 3: Virtual DOM — h() ───
// React.createElement('View', { style: ... }, children)
// becomes: h('Frame', { backgroundColor: '#fff' }, [children])

export function h(type, props = {}, ...children) {
    // Flatten nested arrays
    const flat = children.flat(Infinity).filter(c => c != null && c !== false);
    return { type, props: props || {}, children: flat };
}

// ─── STEP 4: State — useState() ───
// Stores state in the component's fiber (internal tree node).
// When setState is called, schedules a re-render.

let currentFiber = null;    // The fiber currently being rendered
let hookIndex = 0;          // Which hook we're on in this render
let pendingRender = null;   // Scheduled re-render

export function useState(initial) {
    const fiber = currentFiber;
    const idx = hookIndex++;

    // Initialize hook state on first render
    if (!fiber.hooks[idx]) {
        fiber.hooks[idx] = { value: initial };
    }

    const hook = fiber.hooks[idx];

    const setState = (newValue) => {
        const next = typeof newValue === 'function' ? newValue(hook.value) : newValue;
        if (next === hook.value) return;
        hook.value = next;
        // Schedule re-render (batched via microtask)
        if (!pendingRender) {
            pendingRender = Promise.resolve().then(() => {
                pendingRender = null;
                rerender();
            });
        }
    };

    return [hook.value, setState];
}

// ─── STEP 4: Reconciler — The Heart of React ───
//
// Compares the old virtual tree to the new virtual tree.
// Produces a minimal set of operations:
//   CREATE — new node that didn't exist before
//   UPDATE — node exists but props changed
//   REMOVE — node was removed
//
// This is WHY React is fast. Without diffing, every state change
// would destroy and recreate the entire UI.

function reconcile(ui, parentKey, oldFiber, vnode, index) {
    // Determine if we can reuse the existing fiber
    const sameType = oldFiber && vnode && oldFiber.vtype === getVType(vnode);

    let fiber;

    if (sameType) {
        // ─── UPDATE: Same type, just patch props ───
        fiber = oldFiber;
        fiber.vnode = vnode;
        fiber.props = vnode.props;

        if (typeof vnode.type === 'string') {
            // Host component (Frame, Text, etc.) — update native props
            ui.updateProps(fiber.nativeKey, vnode.props);
        }
        // If it's a function component, we'll re-render it below
    } else {
        // ─── CREATE or REPLACE ───
        if (oldFiber) {
            // Remove old tree
            removeFiber(ui, oldFiber);
        }

        if (!vnode) return null;

        fiber = {
            vnode,
            vtype: getVType(vnode),
            props: vnode.props,
            hooks: [],
            children: [],
            nativeKey: null,
            parent: null,
        };

        if (typeof vnode.type === 'string') {
            // Host component — create native node
            if (parentKey) {
                fiber.nativeKey = ui.createViewAsChild(parentKey, vnode.type, vnode.props);
            } else {
                fiber.nativeKey = ui.createView(vnode.type, vnode.props);
            }
        }
    }

    // ─── Render children ───
    let childVNodes;

    if (typeof vnode.type === 'function') {
        // Function component — call it to get its virtual children
        currentFiber = fiber;
        hookIndex = 0;
        const output = vnode.type({ ...vnode.props, children: vnode.children });
        currentFiber = null;

        // Function components return a single vnode (their render output)
        childVNodes = output ? [output] : [];
        // Function components don't create native nodes — they inherit parent's
        fiber.nativeKey = fiber.nativeKey || parentKey;
    } else {
        childVNodes = vnode.children || [];
    }

    // The native key for children to attach to
    const nativeParent = (typeof vnode.type === 'function') ? parentKey : fiber.nativeKey;

    // Reconcile each child
    const oldChildren = fiber.children || [];
    const newChildren = [];
    const maxLen = Math.max(oldChildren.length, childVNodes.length);

    for (let i = 0; i < maxLen; i++) {
        const oldChild = oldChildren[i] || null;
        const newChild = childVNodes[i] || null;
        const result = reconcile(ui, nativeParent, oldChild, newChild, i);
        if (result) newChildren.push(result);
    }

    fiber.children = newChildren;
    return fiber;
}

function getVType(vnode) {
    if (!vnode) return null;
    return typeof vnode.type === 'function' ? vnode.type.name || vnode.type : vnode.type;
}

function removeFiber(ui, fiber) {
    if (!fiber) return;
    // Remove children first (bottom-up)
    for (const child of fiber.children || []) {
        removeFiber(ui, child);
    }
    // Remove native node (only host components have one)
    if (fiber.nativeKey && typeof fiber.vnode?.type === 'string') {
        ui.removeView(fiber.nativeKey);
    }
}

// ─── STEP 5: Event System ───
//
// In React Native, the "Responder System" handles touch events.
// A tap on screen → hit test → find the deepest component with
// an onPress handler → call it.
//
// We do the same: canvas click → getNodeAtPoint() → walk up
// the fiber tree to find onClick handler → call it.

function setupEvents(ui, getRootFiber) {
    const canvas = ui.canvas;

    const handleClick = (screenX, screenY) => {
        const hitKey = ui.getNodeAtPoint(screenX, screenY);
        if (!hitKey) return;

        // Check if we hit a TextInput — the native side knows which keys are inputs
        const inputKey = ui.getInputAtKey(hitKey);
        if (inputKey !== null) {
            ui.focusInput(inputKey);
            return;
        }

        // Normal click handler bubbling
        const rootFiber = getRootFiber();
        const handler = findHandlerBubble(rootFiber, hitKey, 'onClick');
        if (handler) handler();
    };

    canvas.addEventListener('mousedown', (e) => {
        handleClick(e.clientX, e.clientY);
    });

    // Touch: fire click only on touchend if it wasn't a scroll drag
    canvas.addEventListener('touchend', (e) => {
        if (ui.wasTouchScroll()) return; // Was a scroll, not a tap
        if (e.changedTouches.length === 1) {
            handleClick(e.changedTouches[0].clientX, e.changedTouches[0].clientY);
        }
    }, { passive: true });
}

function findHandler(fiber, targetKey, eventName) {
    if (!fiber) return null;

    // Search children first (depth-first to find the target)
    for (const child of fiber.children || []) {
        const found = findHandler(child, targetKey, eventName);
        if (found) return found;
    }

    // Check if this fiber's native node matches OR contains the target
    // Events bubble UP: if a child was hit, check if THIS node has the handler
    if (fiber.nativeKey === targetKey && fiber.props[eventName]) {
        return fiber.props[eventName];
    }

    return null;
}

function findFiberByNativeKey(fiber, key) {
    if (!fiber) return null;
    if (fiber.nativeKey === key) return fiber;
    for (const child of fiber.children || []) {
        const found = findFiberByNativeKey(child, key);
        if (found) return found;
    }
    return null;
}

function findAncestorOfType(rootFiber, targetKey, typeName) {
    const path = [];
    function walk(fiber) {
        if (!fiber) return false;
        path.push(fiber);
        if (fiber.nativeKey === targetKey) return true;
        for (const child of fiber.children || []) {
            if (walk(child)) return true;
        }
        path.pop();
        return false;
    }
    walk(rootFiber);
    for (let i = path.length - 1; i >= 0; i--) {
        if (path[i].vnode && path[i].vnode.type === typeName) return path[i];
    }
    return null;
}

// Find the handler by bubbling up: find the target fiber, then walk up ancestors
function findHandlerBubble(rootFiber, targetKey, eventName) {
    // First find the path from root to the target
    const path = [];
    function findPath(fiber) {
        if (!fiber) return false;
        path.push(fiber);
        if (fiber.nativeKey === targetKey) return true;
        for (const child of fiber.children || []) {
            if (findPath(child)) return true;
        }
        path.pop();
        return false;
    }
    findPath(rootFiber);

    // Walk the path from target (deepest) to root, looking for handler
    for (let i = path.length - 1; i >= 0; i--) {
        if (path[i].props[eventName]) {
            return path[i].props[eventName];
        }
    }
    return null;
}

// ─── MOUNT: Entry Point ───
//
// React Native's AppRegistry.registerComponent() equivalent.
// Takes a root component function and a canvas, wires everything up.

let rootFiber = null;
let rootComponent = null;
let ui = null;

function rerender() {
    if (!rootComponent || !ui) return;
    const vnode = h(rootComponent, {});
    rootFiber = reconcile(ui, null, rootFiber, vnode, 0);
    ui._dirty = true;
    updateOpsDisplay(ui);
    // Recalculate scrollable height after tree changes
    setTimeout(() => {
        let maxY = 0;
        for (const [, node] of ui.nodes) {
            const bounds = ui.engine.get_node_world_bounds(node.counter, node.clientId);
            if (bounds && bounds.length === 4) {
                const bottom = bounds[1] + bounds[3];
                if (bottom > maxY) maxY = bottom;
            }
        }
        ui.setScrollableHeight(maxY);
    }, 50);
}

export async function mount(Component, canvas) {
    const { RenderoUI } = await import('./rendero-ui.js');
    ui = new RenderoUI();
    await ui.init(canvas);

    rootComponent = Component;

    // Initial render
    const vnode = h(Component, {});
    rootFiber = reconcile(ui, null, null, vnode, 0);
    ui._dirty = true;

    // Wire up events
    setupEvents(ui, () => rootFiber);

    // Auto-compute scrollable height from content bounds
    setTimeout(() => {
        let maxY = 0;
        for (const [key, node] of ui.nodes) {
            const bounds = ui.engine.get_node_world_bounds(node.counter, node.clientId);
            if (bounds && bounds.length === 4) {
                const bottom = bounds[1] + bounds[3];
                if (bottom > maxY) maxY = bottom;
            }
        }
        ui.setScrollableHeight(maxY);
    }, 100);

    updateOpsDisplay(ui);
    window.__ui = ui; // Debug access
    return ui;
}

// ─── Debug: Show reconciler ops on screen ───

function updateOpsDisplay(ui) {
    const el = document.getElementById('ops');
    if (!el) return;
    const ops = ui.getOpsLog();
    el.innerHTML = ops.map(op => {
        const cls = op.type === 'CREATE' ? 'op-create' : op.type === 'UPDATE' ? 'op-update' : 'op-remove';
        const data = op.data ? ' ' + JSON.stringify(op.data).slice(0, 60) : '';
        return `<div class="${cls}">${op.type} ${op.target}${data}</div>`;
    }).join('');
    el.parentElement.scrollTop = el.parentElement.scrollHeight;
}
