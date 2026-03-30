// ═══════════════════════════════════════════════════════════════
// Document Shim — document.createElement, createTextNode, etc.
// ═══════════════════════════════════════════════════════════════
//
// This replaces the real `document` when running in native mode.
// On web, none of this is used — react-dom talks to the real browser DOM.
//
// The shim document is the root of the virtual DOM tree.
// When react-dom calls document.createElement('div'), it gets a ShimElement
// backed by a Rendero Frame node. When it appends that to the DOM tree,
// the engine node is created and displayed.

import { ShimNode, ELEMENT_NODE, DOCUMENT_NODE, DOCUMENT_FRAGMENT_NODE } from './node.js';
import { ShimElement } from './element.js';
import { ShimTextNode } from './text-node.js';
import { EventTargetMixin } from './events.js';

class ShimDocumentFragment extends ShimNode {
    constructor() {
        super(DOCUMENT_FRAGMENT_NODE);
        this.nodeName = '#document-fragment';
    }
}

export class ShimDocument extends ShimNode {
    constructor() {
        super(DOCUMENT_NODE);
        this.nodeName = '#document';
        this.nodeType = DOCUMENT_NODE;

        // Create html > body structure
        this.documentElement = new ShimElement('html');
        this.documentElement.ownerDocument = this;
        this.head = new ShimElement('head');
        this.head.ownerDocument = this;
        this.body = new ShimElement('body');
        this.body.ownerDocument = this;
        this.documentElement.appendChild(this.head);
        this.documentElement.appendChild(this.body);
        this.childNodes = [this.documentElement];

        this.defaultView = null; // Set by window shim
        this.readyState = 'complete';
        this.visibilityState = 'visible';
        this.hidden = false;
    }

    createElement(tagName) {
        const el = new ShimElement(tagName);
        el.ownerDocument = this;
        return el;
    }

    createElementNS(ns, tagName) {
        // SVG namespace creates frames for now
        const el = new ShimElement(tagName);
        el.ownerDocument = this;
        el.namespaceURI = ns;
        return el;
    }

    createTextNode(text) {
        const tn = new ShimTextNode(text);
        tn.ownerDocument = this;
        return tn;
    }

    createComment(text) {
        // Comments are no-ops in the engine
        const n = new ShimNode(8); // COMMENT_NODE
        n.nodeName = '#comment';
        n.textContent = text;
        n.ownerDocument = this;
        return n;
    }

    createDocumentFragment() {
        const frag = new ShimDocumentFragment();
        frag.ownerDocument = this;
        return frag;
    }

    createEvent(type) {
        return { type: '', initEvent(t) { this.type = t; }, bubbles: true, cancelable: true };
    }

    createTreeWalker(root, whatToShow, filter) {
        // Minimal TreeWalker for react-dom
        let current = root;
        return {
            currentNode: root,
            get firstChild() {
                const c = this.currentNode.firstChild;
                if (c) this.currentNode = c;
                return c;
            },
            nextSibling() {
                const s = this.currentNode.nextSibling;
                if (s) this.currentNode = s;
                return s;
            },
            parentNode() {
                const p = this.currentNode.parentNode;
                if (p) this.currentNode = p;
                return p;
            },
        };
    }

    // ─── Query methods ───

    getElementById(id) {
        return this.body.getElementById(id) || this.head.getElementById(id);
    }

    querySelector(selector) {
        return this.body.querySelector(selector) || this.head.querySelector(selector);
    }

    querySelectorAll(selector) {
        return [...this.body.querySelectorAll(selector), ...this.head.querySelectorAll(selector)];
    }

    getElementsByTagName(tag) {
        return this.body.getElementsByTagName(tag);
    }

    getElementsByClassName(cls) {
        return this.body.getElementsByClassName(cls);
    }

    // ─── Properties react-dom checks ───

    get activeElement() { return this.body; }
    get compatMode() { return 'CSS1Compat'; }
    get contentType() { return 'text/html'; }
    get doctype() { return { name: 'html' }; }
    get implementation() {
        return {
            createHTMLDocument: (title) => {
                const doc = new ShimDocument();
                return doc;
            },
            hasFeature: () => true,
        };
    }

    // Noop methods react-dom may call
    addEventListener(type, handler, opts) {
        // Delegate to body for document-level events
        this.body.addEventListener(type, handler, opts);
    }
    removeEventListener(type, handler, opts) {
        this.body.removeEventListener(type, handler, opts);
    }
    dispatchEvent(event) {
        return this.body.dispatchEvent(event);
    }

    // ─── Init: connect body to engine ───

    _connectToEngine() {
        // Size the shim root to the viewport before creating the engine root.
        // The renderer culls from the root down, so a 0x0 body frame would
        // discard the entire scene even if descendants have visible content.
        this.documentElement.style.width = '100vw';
        this.documentElement.style.height = '100vh';
        this.body.style.width = '100vw';
        this.body.style.height = '100vh';

        // Create the body element in the engine as the root frame
        this.body._createInEngine(null);
    }
}
