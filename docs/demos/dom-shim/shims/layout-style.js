import { parseLength } from './css-values.js';

export function parseAlignMode(value) {
    switch (value) {
        case 'center': return 1;
        case 'flex-end':
        case 'end': return 2;
        case 'stretch': return 3;
        default: return 0;
    }
}

export function parseJustifyMode(value) {
    switch (value) {
        case 'center': return 1;
        case 'flex-end':
        case 'end': return 2;
        case 'space-between': return 3;
        case 'space-around': return 4;
        case 'space-evenly': return 5;
        default: return 0;
    }
}

export function resolveMargins(values) {
    const parseMarginEdge = (value) => {
        if (typeof value === 'string' && value.trim() === 'auto') {
            return { value: 0, auto: true };
        }
        return { value: parseLength(value) || 0, auto: false };
    };
    const top = parseMarginEdge(values.marginTop);
    const right = parseMarginEdge(values.marginRight);
    const bottom = parseMarginEdge(values.marginBottom);
    const left = parseMarginEdge(values.marginLeft);
    return {
        top: top.value,
        right: right.value,
        bottom: bottom.value,
        left: left.value,
        autoTop: top.auto,
        autoRight: right.auto,
        autoBottom: bottom.auto,
        autoLeft: left.auto,
    };
}

export function buildAutoLayout(values, { isText, hasChildren }) {
    const isFlexLayout = values.display === 'flex' || values.display === 'inline-flex';
    const isBlockFlowContainer =
        !isText &&
        !!hasChildren &&
        !isFlexLayout &&
        values.display !== 'inline' &&
        values.display !== 'inline-block' &&
        values.display !== 'contents';

    if (!(isFlexLayout || isBlockFlowContainer ||
        values.flexDirection || values.gap || values.padding ||
        values.paddingTop || values.paddingRight || values.paddingBottom || values.paddingLeft)) {
        return null;
    }

    const dir = values.flexDirection === 'row' || values.flexDirection === 'row-reverse' ? 0 : 1;
    const gap = parseLength(values.gap) || parseLength(values.rowGap) || parseLength(values.columnGap) || 0;

    return {
        direction: dir,
        spacing: gap,
        padTop: parseLength(values.paddingTop) || parseLength(values.padding) || 0,
        padRight: parseLength(values.paddingRight) || parseLength(values.padding) || 0,
        padBottom: parseLength(values.paddingBottom) || parseLength(values.padding) || 0,
        padLeft: parseLength(values.paddingLeft) || parseLength(values.padding) || 0,
        align: parseAlignMode(values.alignItems),
        justify: parseJustifyMode(values.justifyContent),
        wrap: values.flexWrap === 'wrap' ? 1 : 0,
    };
}
