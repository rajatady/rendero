// ═══════════════════════════════════════════════════════════════
// Element — the core DOM Element shim
// ═══════════════════════════════════════════════════════════════
//
// Maps HTML elements to Rendero engine nodes:
//   div, section, header, footer, nav, main, article, aside → Frame
//   span, p, h1-h6, a, label, strong, em → Text (or Frame with text child)
//   img → Frame with image fill
//   button → Frame (clickable)
//   input, textarea → TextInput pattern
//
// react-dom calls createElement, then sets properties, then appendChild.
// Each of those operations routes through here to the engine.

import { ShimNode, ELEMENT_NODE } from './node.js';
import { ShimTextNode } from './text-node.js';
import { createStyleProxy } from './style.js';
import {
    engineCreateFrame, engineCreateText, engineDeleteNode,
    engineSetProp, engineGetBounds, setInsertParent, clearInsertParent,
    markDirty, getNodeIds,
} from './engine-runtime.js';
import { parseColor, parseLength, parseLineHeight } from './css-values.js';

// Elements that are inherently text containers
const TEXT_TAGS = new Set(['span', 'p', 'h1', 'h2', 'h3', 'h4', 'h5', 'h6', 'a', 'label', 'strong', 'em', 'b', 'i', 'small', 'code', 'pre', 'li']);
const BLOCK_TEXT_TAGS = new Set(['p', 'h1', 'h2', 'h3', 'h4', 'h5', 'h6', 'pre', 'li']);
// Elements that are inherently layout containers (frames)
const FRAME_TAGS = new Set(['div', 'section', 'header', 'footer', 'nav', 'main', 'article', 'aside', 'ul', 'ol', 'form', 'fieldset', 'figure', 'figcaption', 'details', 'summary', 'dialog', 'table', 'tr', 'td', 'th', 'thead', 'tbody', 'tfoot']);

export class ShimElement extends ShimNode {
    constructor(tagName) {
        super(ELEMENT_NODE);
        this.tagName = tagName.toUpperCase();
        this.localName = tagName.toLowerCase();
        this.nodeName = this.tagName;
        this._attributes = {};
        this._classList = new ShimClassList(this);
        this._dataset = {};
        this.style = createStyleProxy(this);
        this._engineCreated = false;
        this._isTextElement = TEXT_TAGS.has(this.localName);
        this._textContent = '';
        this.namespaceURI = 'http://www.w3.org/1999/xhtml';

        // Properties that react-dom sets directly
        this._innerHTML = '';
    }

    // ─── Attributes ───

    setAttribute(name, value) {
        this._attributes[name] = String(value);
        this._syncAttribute(name, value);
    }

    getAttribute(name) {
        return this._attributes[name] !== undefined ? this._attributes[name] : null;
    }

    removeAttribute(name) {
        delete this._attributes[name];
    }

    hasAttribute(name) {
        return name in this._attributes;
    }

    get id() { return this._attributes.id || ''; }
    set id(val) { this._attributes.id = val; }

    get className() { return this._attributes.class || ''; }
    set className(val) {
        this._attributes.class = val;
        this._classList._parse(val);
    }

    get classList() { return this._classList; }
    get dataset() { return this._dataset; }

    get children() {
        return this.childNodes.filter(c => c.nodeType === ELEMENT_NODE);
    }
    get childElementCount() { return this.children.length; }
    get firstElementChild() { return this.children[0] || null; }
    get lastElementChild() { return this.children[this.children.length - 1] || null; }

    // ─── textContent ───

    get textContent() {
        if (this.childNodes.length === 0) return this._textContent;
        let text = '';
        for (const c of this.childNodes) {
            text += c.textContent || '';
        }
        return text;
    }

    set textContent(val) {
        // Remove all children
        while (this.childNodes.length > 0) {
            this.removeChild(this.childNodes[0]);
        }
        this._textContent = val || '';
        if (val) {
            const tn = new ShimTextNode(val);
            this.appendChild(tn);
        }
    }

    get innerText() { return this.textContent; }
    set innerText(val) { this.textContent = val; }

    get innerHTML() { return this._innerHTML; }
    set innerHTML(val) {
        // Clear children
        while (this.childNodes.length > 0) {
            this.removeChild(this.childNodes[0]);
        }
        this._innerHTML = val;
        // Minimal: if it's just text, create a text node
        if (val && !val.includes('<')) {
            this.appendChild(new ShimTextNode(val));
        }
        // Full HTML parsing would use html5ever — skipping for POC
    }

    // ─── Geometry ───

    getBoundingClientRect() {
        const b = engineGetBounds(this._engineId);
        return {
            x: b.x, y: b.y, width: b.width, height: b.height,
            top: b.y, left: b.x, right: b.x + b.width, bottom: b.y + b.height,
            toJSON() { return this; },
        };
    }

    get offsetWidth() { return this.getBoundingClientRect().width; }
    get offsetHeight() { return this.getBoundingClientRect().height; }
    get offsetTop() { return this.getBoundingClientRect().top; }
    get offsetLeft() { return this.getBoundingClientRect().left; }
    get clientWidth() { return this.offsetWidth; }
    get clientHeight() { return this.offsetHeight; }
    get scrollWidth() { return this.offsetWidth; }
    get scrollHeight() { return this.offsetHeight; }
    get scrollTop() { return 0; }
    set scrollTop(v) {}
    get scrollLeft() { return 0; }
    set scrollLeft(v) {}

    // ─── Query ───

    querySelector(selector) {
        return queryOne(this, selector);
    }
    querySelectorAll(selector) {
        const results = [];
        queryAll(this, selector, results);
        return results;
    }
    getElementsByTagName(tag) {
        const results = [];
        const lower = tag.toLowerCase();
        queryAll(this, null, results, (el) =>
            lower === '*' || el.localName === lower);
        return results;
    }
    getElementsByClassName(cls) {
        const results = [];
        queryAll(this, null, results, (el) =>
            el._classList.contains(cls));
        return results;
    }
    getElementById(id) {
        return queryOne(this, null, (el) => el.id === id);
    }

    matches(selector) {
        return matchesSelector(this, selector);
    }
    closest(selector) {
        let el = this;
        while (el) {
            if (el.nodeType === ELEMENT_NODE && matchesSelector(el, selector)) return el;
            el = el.parentNode;
        }
        return null;
    }

    // ─── Focus ───

    focus() { /* TODO: focus management */ }
    blur() {}
    click() {
        this.dispatchEvent({ type: 'click', bubbles: true, cancelable: true, target: this, currentTarget: this, preventDefault() {}, stopPropagation() {} });
    }

    // ─── Misc DOM properties react-dom needs ───

    get nodeValue() { return null; }
    get isConnected() { return !!this.parentNode; }
    get baseURI() { return ''; }

    // react-dom checks these
    get ELEMENT_NODE() { return 1; }
    get TEXT_NODE() { return 3; }
    get DOCUMENT_NODE() { return 9; }

    // ─── Engine sync ───

    _createInEngine(parentEngineId) {
        if (this._engineCreated) return;

        if (parentEngineId) {
            setInsertParent(parentEngineId);
        }

        // Decide: Frame or Text engine node
        let result;
        const textBacked = this._isRenderedAsTextNode();
        if (textBacked) {
            result = engineCreateText(this._engineId, `${this.localName}_${this._engineId}`, this._engineTextContent());
        } else {
            result = engineCreateFrame(this._engineId, `${this.localName}_${this._engineId}`);
        }

        if (parentEngineId) {
            clearInsertParent();
        }

        if (!result) {
            if (typeof __rendero_log === 'function') __rendero_log('ENGINE CREATE FAILED: ' + this.localName + '#' + this._engineId);
            return;
        }
        this._engineCreated = true;

        if (typeof __rendero_log === 'function') {
            const sv = this.style._values;
            __rendero_log('CREATED: <' + this.localName + '> id=' + this._engineId +
                ' w=' + (sv.width||'?') + ' h=' + (sv.height||'?') +
                ' bg=' + (sv.backgroundColor||'none') +
                ' display=' + (sv.display||'?') +
                ' children=' + this.childNodes.length);
        }

        // Apply initial styles
        this.style._syncNow();

        // Create children in engine
        if (!textBacked) {
            for (const child of this.childNodes) {
                if (child._createInEngine) {
                    child._createInEngine(this._engineId);
                }
            }
        }
    }

    _destroyInEngine() {
        if (!this._engineCreated) return;
        // Destroy children first
        for (const child of this.childNodes) {
            if (child._destroyInEngine) child._destroyInEngine();
        }
        engineDeleteNode(this._engineId);
        this._engineCreated = false;
    }

    _onChildInserted(child, refChild) {
        if (typeof __rendero_log === 'function') {
            __rendero_log('CHILD_INSERT parent=<' + this.localName + '>#' + this._engineId +
                ' engineCreated=' + this._engineCreated +
                ' child=' + (child.localName || '#text') + '#' + child._engineId +
                ' childType=' + child.nodeType);
        }
        if (!this._engineCreated) return;
        if (this._isRenderedAsTextNode()) {
            this._syncTextChildrenToEngine();
            return;
        }
        if (child._createInEngine) {
            child._createInEngine(this._engineId);
        }
    }

    _onChildRemoved(child) {
        if (this._engineCreated && this._isRenderedAsTextNode()) {
            this._syncTextChildrenToEngine();
            return;
        }
        if (child._destroyInEngine) {
            child._destroyInEngine();
        }
    }

    _hasElementChildren() {
        return this.childNodes.some(c => c.nodeType === ELEMENT_NODE);
    }

    _isRenderedAsTextNode() {
        return this._isTextElement && !BLOCK_TEXT_TAGS.has(this.localName) && !this._hasElementChildren();
    }

    _engineTextContent() {
        const text = this.textContent || '';
        return text.trim() ? text : ' ';
    }

    _syncTextChildrenToEngine() {
        if (!this._engineCreated || !this._isRenderedAsTextNode()) return;
        const text = this._engineTextContent();
        const styles = this.style?._values || {};
        engineSetProp(this._engineId, 'text', text);
        if (typeof this.style?._syncNow === 'function') {
            this.style._syncNow();
        }
        const fontSize = parseLength(styles.fontSize) || 16;
        if (styles.lineHeight) {
            engineSetProp(this._engineId, 'lineHeight', parseLineHeight(styles.lineHeight, fontSize));
        }
        if (styles.letterSpacing) {
            engineSetProp(this._engineId, 'letterSpacing', parseLength(styles.letterSpacing, fontSize));
        }
        markDirty();
    }

    _syncFrameBackedTextChildrenToEngine() {
        if (!this._engineCreated || this._isRenderedAsTextNode()) return;
        if (!this._isTextElement) return;
        for (const child of this.childNodes) {
            if (child.nodeType === 3 && typeof child._syncInheritedTextStyleFromParent === 'function') {
                child._syncInheritedTextStyleFromParent(this);
            }
        }
    }

    _syncAttribute(name, value) {
        if (!this._engineCreated) return;

        switch (name) {
            case 'style':
                this.style.cssText = value;
                break;
            case 'class':
                // Could resolve class-based styles here
                break;
            case 'src':
                // Image source — would load and set image fill
                break;
        }
    }

    cloneNode(deep = false) {
        const clone = new ShimElement(this.localName);
        clone._attributes = { ...this._attributes };
        clone.style.cssText = this.style.cssText;
        if (deep) {
            for (const c of this.childNodes) {
                clone.appendChild(c.cloneNode(true));
            }
        }
        return clone;
    }
}

// ─── ClassList ───

class ShimClassList {
    constructor(element) {
        this._el = element;
        this._set = new Set();
    }
    _parse(str) {
        this._set.clear();
        if (str) str.split(/\s+/).forEach(c => c && this._set.add(c));
    }
    add(...classes) { classes.forEach(c => this._set.add(c)); this._sync(); }
    remove(...classes) { classes.forEach(c => this._set.delete(c)); this._sync(); }
    toggle(cls, force) {
        if (force !== undefined) {
            force ? this._set.add(cls) : this._set.delete(cls);
        } else {
            this._set.has(cls) ? this._set.delete(cls) : this._set.add(cls);
        }
        this._sync();
    }
    contains(cls) { return this._set.has(cls); }
    get length() { return this._set.size; }
    item(idx) { return [...this._set][idx] || null; }
    get value() { return [...this._set].join(' '); }
    toString() { return this.value; }
    forEach(fn) { this._set.forEach(fn); }
    [Symbol.iterator]() { return this._set.values(); }
    _sync() { this._el._attributes.class = this.value; }
}

// ─── Minimal query engine ───

function matchesSelector(el, selector) {
    if (!selector) return true;
    if (selector.startsWith('#')) return el.id === selector.slice(1);
    if (selector.startsWith('.')) return el._classList.contains(selector.slice(1));
    if (selector === '*') return true;
    return el.localName === selector.toLowerCase();
}

function queryOne(root, selector, predicate) {
    for (const c of root.childNodes) {
        if (c.nodeType !== ELEMENT_NODE) continue;
        if (predicate ? predicate(c) : matchesSelector(c, selector)) return c;
        const found = queryOne(c, selector, predicate);
        if (found) return found;
    }
    return null;
}

function queryAll(root, selector, results, predicate) {
    for (const c of root.childNodes) {
        if (c.nodeType !== ELEMENT_NODE) continue;
        if (predicate ? predicate(c) : matchesSelector(c, selector)) results.push(c);
        queryAll(c, selector, results, predicate);
    }
}
