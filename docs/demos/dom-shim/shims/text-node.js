// ═══════════════════════════════════════════════════════════════
// Text Node — textContent / nodeValue → engine text updates
// ═══════════════════════════════════════════════════════════════

import { ShimNode, TEXT_NODE } from './node.js';
import { engineCreateText, engineDeleteNode, engineSetProp, setInsertParent, clearInsertParent } from './engine-runtime.js';

export class ShimTextNode extends ShimNode {
    constructor(text) {
        super(TEXT_NODE);
        this._text = text || '';
        this._engineCreated = false;
        this.nodeName = '#text';
    }

    get textContent() { return this._text; }
    set textContent(val) {
        this._text = val || '';
        if (this.parentNode && this.parentNode._isRenderedAsTextNode && this.parentNode._isRenderedAsTextNode()) {
            this.parentNode._syncTextChildrenToEngine();
            return;
        }
        if (this._engineCreated) {
            engineSetProp(this._engineId, 'text', this._text);
        }
    }

    get nodeValue() { return this._text; }
    set nodeValue(val) { this.textContent = val; }

    get data() { return this._text; }
    set data(val) { this.textContent = val; }

    get wholeText() { return this._text; }
    get length() { return this._text.length; }

    // Called when this text node is inserted into an element
    _createInEngine(parentEngineId) {
        if (this._engineCreated) return;
        if (!this._text || !this._text.trim()) return; // skip whitespace-only
        if (this.parentNode && this.parentNode._isRenderedAsTextNode && this.parentNode._isRenderedAsTextNode()) {
            this.parentNode._syncTextChildrenToEngine();
            return;
        }
        setInsertParent(parentEngineId);
        const result = engineCreateText(this._engineId, `text_${this._engineId}`, this._text);
        clearInsertParent();
        if (result) this._engineCreated = true;
    }

    // Called when this text node is removed from the tree
    _destroyInEngine() {
        if (this.parentNode && this.parentNode._isRenderedAsTextNode && this.parentNode._isRenderedAsTextNode()) {
            this.parentNode._syncTextChildrenToEngine();
            return;
        }
        if (!this._engineCreated) return;
        engineDeleteNode(this._engineId);
        this._engineCreated = false;
    }

    cloneNode() {
        return new ShimTextNode(this._text);
    }
}
