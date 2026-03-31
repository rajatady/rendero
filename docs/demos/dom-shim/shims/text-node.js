// ═══════════════════════════════════════════════════════════════
// Text Node — textContent / nodeValue → engine text updates
// ═══════════════════════════════════════════════════════════════

import { ShimNode, TEXT_NODE } from './node.js';
import { engineCreateText, engineDeleteNode, engineSetProp, setInsertParent, clearInsertParent } from './engine-runtime.js';
import { parseColor, parseLength, parseLineHeight, parseFontWeight } from './css-values.js';

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
        if (result) {
            this._engineCreated = true;
            if (this.parentNode && typeof this._syncInheritedTextStyleFromParent === 'function') {
                this._syncInheritedTextStyleFromParent(this.parentNode);
            }
        }
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

    _syncInheritedTextStyleFromParent(parent) {
        if (!this._engineCreated || !parent?.style?._values) return;
        const styles = parent.style._values;
        const fontSize = parseLength(styles.fontSize) || 16;
        if (styles.fontSize) engineSetProp(this._engineId, 'fontSize', fontSize);
        if (styles.fontWeight) engineSetProp(this._engineId, 'fontWeight', parseFontWeight(styles.fontWeight));
        if (styles.fontFamily) engineSetProp(this._engineId, 'fontFamily', styles.fontFamily.replace(/['"]/g, ''));
        if (styles.letterSpacing) engineSetProp(this._engineId, 'letterSpacing', parseLength(styles.letterSpacing, fontSize));
        if (styles.lineHeight) engineSetProp(this._engineId, 'lineHeight', parseLineHeight(styles.lineHeight, fontSize));
        if (styles.textAlign) engineSetProp(this._engineId, 'textAlign', styles.textAlign);
        if (styles.color) {
            const c = parseColor(styles.color);
            if (c) engineSetProp(this._engineId, 'fill', { r: c[0], g: c[1], b: c[2], a: c[3] });
        }
    }
}
