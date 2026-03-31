// ═══════════════════════════════════════════════════════════════
// Node — base class for all DOM shim nodes
// ═══════════════════════════════════════════════════════════════
//
// Implements: Node interface (parentNode, childNodes, appendChild,
// removeChild, insertBefore, replaceChild, contains, etc.)

import { allocId } from './engine-runtime.js';
import { EventTargetMixin } from './events.js';

// Node type constants
export const ELEMENT_NODE = 1;
export const TEXT_NODE = 3;
export const COMMENT_NODE = 8;
export const DOCUMENT_NODE = 9;
export const DOCUMENT_FRAGMENT_NODE = 11;

export class ShimNode {
    constructor(nodeType) {
        this.nodeType = nodeType;
        this.parentNode = null;
        this.parentElement = null;
        this.childNodes = [];
        this.ownerDocument = null;
        this._engineId = allocId();
        this._initEvents();
    }

    get firstChild() { return this.childNodes[0] || null; }
    get lastChild() { return this.childNodes[this.childNodes.length - 1] || null; }

    get nextSibling() {
        if (!this.parentNode) return null;
        const siblings = this.parentNode.childNodes;
        const idx = siblings.indexOf(this);
        return idx >= 0 && idx < siblings.length - 1 ? siblings[idx + 1] : null;
    }

    get previousSibling() {
        if (!this.parentNode) return null;
        const siblings = this.parentNode.childNodes;
        const idx = siblings.indexOf(this);
        return idx > 0 ? siblings[idx - 1] : null;
    }

    get nextElementSibling() {
        let s = this.nextSibling;
        while (s && s.nodeType !== ELEMENT_NODE) s = s.nextSibling;
        return s;
    }

    get previousElementSibling() {
        let s = this.previousSibling;
        while (s && s.nodeType !== ELEMENT_NODE) s = s.previousSibling;
        return s;
    }

    hasChildNodes() { return this.childNodes.length > 0; }

    appendChild(child) {
        if (child.nodeType === DOCUMENT_FRAGMENT_NODE) {
            const kids = [...child.childNodes];
            for (const k of kids) this.appendChild(k);
            return child;
        }
        // Remove from current parent
        if (child.parentNode) {
            try { child.parentNode.removeChild(child); } catch (e) {}
        }
        _setParent(child, this);
        this.childNodes.push(child);
        this._onChildInserted(child);
        return child;
    }

    removeChild(child) {
        const idx = this.childNodes.indexOf(child);
        if (idx === -1) throw new Error('Node not found');
        this.childNodes.splice(idx, 1);
        _setParent(child, null);
        this._onChildRemoved(child);
        return child;
    }

    insertBefore(newChild, refChild) {
        if (!refChild) return this.appendChild(newChild);
        if (newChild.nodeType === DOCUMENT_FRAGMENT_NODE) {
            const kids = [...newChild.childNodes];
            for (const k of kids) this.insertBefore(k, refChild);
            return newChild;
        }
        if (newChild.parentNode) {
            try { newChild.parentNode.removeChild(newChild); } catch (e) {}
        }
        const idx = this.childNodes.indexOf(refChild);
        if (idx === -1) throw new Error('Reference node not found');
        _setParent(newChild, this);
        this.childNodes.splice(idx, 0, newChild);
        this._onChildInserted(newChild, refChild);
        return newChild;
    }

    replaceChild(newChild, oldChild) {
        this.insertBefore(newChild, oldChild);
        this.removeChild(oldChild);
        return oldChild;
    }

    contains(other) {
        if (other === this) return true;
        for (const c of this.childNodes) {
            if (c === other || c.contains(other)) return true;
        }
        return false;
    }

    cloneNode(deep = false) {
        // Minimal clone — subclasses should override
        const clone = new ShimNode(this.nodeType);
        if (deep) {
            for (const c of this.childNodes) {
                clone.appendChild(c.cloneNode(true));
            }
        }
        return clone;
    }

    // Subclass hooks for engine sync
    _onChildInserted(child, refChild) {}
    _onChildRemoved(child) {}
}

// Mix in EventTarget methods
Object.assign(ShimNode.prototype, EventTargetMixin);

// Helper: set parentNode on any node (handles read-only native Node.parentNode)
function _setParent(child, parent) {
    try {
        child.parentNode = parent;
    } catch (e) {
        // Native browser nodes have read-only parentNode.
        // Use defineProperty to override the getter.
        Object.defineProperty(child, 'parentNode', {
            value: parent,
            writable: true,
            configurable: true,
        });
    }
    try {
        child.parentElement = parent && parent.nodeType === ELEMENT_NODE ? parent : null;
    } catch (e) {
        Object.defineProperty(child, 'parentElement', {
            value: parent && parent.nodeType === ELEMENT_NODE ? parent : null,
            writable: true,
            configurable: true,
        });
    }
}
