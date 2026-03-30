// ═══════════════════════════════════════════════════════════════
// Layout Accuracy Test Corpus
// ═══════════════════════════════════════════════════════════════
//
// Each test case defines a tree of elements with inline styles.
// The accuracy page renders each case in:
//   1. Real browser DOM → getBoundingClientRect() per element
//   2. Rendero engine (WASM) → engine node bounds per element
// And compares. A mismatch is any dimension differing by ≥ 1px.
//
// Test cases are ordered by CSS complexity. Fix them top-to-bottom.
// Do NOT write test cases that only pass for one specific page.
// Each case tests a general CSS behavior.

export const VIEWPORT = { width: 800, height: 600 };

export const CORPUS = [

    // ─── 1. Block Flow (no flex) ───
    // Elements without display:flex should stack vertically.

    {
        label: 'block-stack-3',
        category: 'block-flow',
        tree: {
            tag: 'div', id: 'root',
            style: { width: '400px' },
            children: [
                { tag: 'div', id: 'a', style: { height: '100px', backgroundColor: '#ff0000' } },
                { tag: 'div', id: 'b', style: { height: '80px', backgroundColor: '#00ff00' } },
                { tag: 'div', id: 'c', style: { height: '60px', backgroundColor: '#0000ff' } },
            ],
        },
    },
    {
        label: 'block-width-100pct',
        category: 'block-flow',
        tree: {
            tag: 'div', id: 'root',
            style: { width: '400px' },
            children: [
                { tag: 'div', id: 'a', style: { width: '100%', height: '50px', backgroundColor: '#ff0000' } },
                { tag: 'div', id: 'b', style: { width: '50%', height: '50px', backgroundColor: '#00ff00' } },
            ],
        },
    },
    {
        label: 'block-nested',
        category: 'block-flow',
        tree: {
            tag: 'div', id: 'root',
            style: { width: '400px' },
            children: [
                {
                    tag: 'div', id: 'outer',
                    style: { padding: '20px', backgroundColor: '#eee' },
                    children: [
                        { tag: 'div', id: 'inner', style: { height: '80px', backgroundColor: '#ff0000' } },
                    ],
                },
                { tag: 'div', id: 'after', style: { height: '40px', backgroundColor: '#0000ff' } },
            ],
        },
    },

    // ─── 2. Flex Column ───

    {
        label: 'flex-column-basic',
        category: 'flex-column',
        tree: {
            tag: 'div', id: 'root',
            style: { display: 'flex', flexDirection: 'column', width: '400px' },
            children: [
                { tag: 'div', id: 'a', style: { height: '100px', backgroundColor: '#ff0000' } },
                { tag: 'div', id: 'b', style: { height: '80px', backgroundColor: '#00ff00' } },
                { tag: 'div', id: 'c', style: { height: '60px', backgroundColor: '#0000ff' } },
            ],
        },
    },
    {
        label: 'flex-column-gap',
        category: 'flex-column',
        tree: {
            tag: 'div', id: 'root',
            style: { display: 'flex', flexDirection: 'column', gap: '10px', width: '400px' },
            children: [
                { tag: 'div', id: 'a', style: { height: '50px', backgroundColor: '#ff0000' } },
                { tag: 'div', id: 'b', style: { height: '50px', backgroundColor: '#00ff00' } },
                { tag: 'div', id: 'c', style: { height: '50px', backgroundColor: '#0000ff' } },
            ],
        },
    },
    {
        label: 'flex-column-padding',
        category: 'flex-column',
        tree: {
            tag: 'div', id: 'root',
            style: { display: 'flex', flexDirection: 'column', padding: '20px 30px', width: '400px' },
            children: [
                { tag: 'div', id: 'a', style: { height: '60px', backgroundColor: '#ff0000' } },
            ],
        },
    },
    {
        label: 'flex-column-align-center',
        category: 'flex-column',
        tree: {
            tag: 'div', id: 'root',
            style: { display: 'flex', flexDirection: 'column', alignItems: 'center', width: '400px' },
            children: [
                { tag: 'div', id: 'a', style: { width: '200px', height: '60px', backgroundColor: '#ff0000' } },
                { tag: 'div', id: 'b', style: { width: '100px', height: '40px', backgroundColor: '#00ff00' } },
            ],
        },
    },

    // ─── 3. Flex Row ───

    {
        label: 'flex-row-basic',
        category: 'flex-row',
        tree: {
            tag: 'div', id: 'root',
            style: { display: 'flex', flexDirection: 'row', width: '600px' },
            children: [
                { tag: 'div', id: 'a', style: { width: '200px', height: '100px', backgroundColor: '#ff0000' } },
                { tag: 'div', id: 'b', style: { width: '150px', height: '100px', backgroundColor: '#00ff00' } },
                { tag: 'div', id: 'c', style: { width: '100px', height: '100px', backgroundColor: '#0000ff' } },
            ],
        },
    },
    {
        label: 'flex-row-gap',
        category: 'flex-row',
        tree: {
            tag: 'div', id: 'root',
            style: { display: 'flex', flexDirection: 'row', gap: '20px', width: '600px' },
            children: [
                { tag: 'div', id: 'a', style: { width: '100px', height: '80px', backgroundColor: '#ff0000' } },
                { tag: 'div', id: 'b', style: { width: '100px', height: '80px', backgroundColor: '#00ff00' } },
                { tag: 'div', id: 'c', style: { width: '100px', height: '80px', backgroundColor: '#0000ff' } },
            ],
        },
    },
    {
        label: 'flex-row-wrap',
        category: 'flex-row',
        tree: {
            tag: 'div', id: 'root',
            style: { display: 'flex', flexDirection: 'row', flexWrap: 'wrap', gap: '10px', width: '350px' },
            children: [
                { tag: 'div', id: 'a', style: { width: '150px', height: '80px', backgroundColor: '#ff0000' } },
                { tag: 'div', id: 'b', style: { width: '150px', height: '80px', backgroundColor: '#00ff00' } },
                { tag: 'div', id: 'c', style: { width: '150px', height: '80px', backgroundColor: '#0000ff' } },
                { tag: 'div', id: 'd', style: { width: '150px', height: '80px', backgroundColor: '#ff00ff' } },
            ],
        },
    },
    {
        label: 'flex-row-justify-center',
        category: 'flex-row',
        tree: {
            tag: 'div', id: 'root',
            style: { display: 'flex', flexDirection: 'row', justifyContent: 'center', width: '600px' },
            children: [
                { tag: 'div', id: 'a', style: { width: '100px', height: '60px', backgroundColor: '#ff0000' } },
                { tag: 'div', id: 'b', style: { width: '100px', height: '60px', backgroundColor: '#00ff00' } },
            ],
        },
    },

    // ─── 4. Sizing ───

    {
        label: 'size-min-height',
        category: 'sizing',
        tree: {
            tag: 'div', id: 'root',
            style: { display: 'flex', flexDirection: 'column', width: '400px', minHeight: '300px' },
            children: [
                { tag: 'div', id: 'a', style: { height: '50px', backgroundColor: '#ff0000' } },
            ],
        },
    },
    {
        label: 'size-max-width',
        category: 'sizing',
        tree: {
            tag: 'div', id: 'root',
            style: { width: '800px' },
            children: [
                { tag: 'div', id: 'a', style: { maxWidth: '300px', height: '50px', backgroundColor: '#ff0000' } },
            ],
        },
    },
    {
        label: 'size-percentage',
        category: 'sizing',
        tree: {
            tag: 'div', id: 'root',
            style: { display: 'flex', flexDirection: 'row', width: '400px', height: '200px' },
            children: [
                { tag: 'div', id: 'a', style: { width: '50%', height: '100%', backgroundColor: '#ff0000' } },
                { tag: 'div', id: 'b', style: { width: '50%', height: '100%', backgroundColor: '#00ff00' } },
            ],
        },
    },
    {
        label: 'size-flex-grow',
        category: 'sizing',
        tree: {
            tag: 'div', id: 'root',
            style: { display: 'flex', flexDirection: 'row', width: '600px', height: '100px' },
            children: [
                { tag: 'div', id: 'fixed', style: { width: '100px', backgroundColor: '#ff0000' } },
                { tag: 'div', id: 'grow', style: { flex: '1', backgroundColor: '#00ff00' } },
            ],
        },
    },
    {
        label: 'size-flex-grow-multiple',
        category: 'sizing',
        tree: {
            tag: 'div', id: 'root',
            style: { display: 'flex', flexDirection: 'row', width: '600px', height: '100px' },
            children: [
                { tag: 'div', id: 'a', style: { flex: '1', backgroundColor: '#ff0000' } },
                { tag: 'div', id: 'b', style: { flex: '2', backgroundColor: '#00ff00' } },
                { tag: 'div', id: 'c', style: { flex: '1', backgroundColor: '#0000ff' } },
            ],
        },
    },
    {
        label: 'size-padding-contributes-height',
        category: 'sizing',
        tree: {
            tag: 'div', id: 'root',
            style: { display: 'flex', flexDirection: 'column', width: '400px' },
            children: [
                {
                    tag: 'div', id: 'padded',
                    style: { padding: '40px 20px', backgroundColor: '#ff0000' },
                    children: [
                        { tag: 'div', id: 'inner', style: { height: '30px', backgroundColor: '#ffffff' } },
                    ],
                },
            ],
        },
    },

    // ─── 5. Margin ───

    {
        label: 'margin-basic',
        category: 'margin',
        tree: {
            tag: 'div', id: 'root',
            style: { display: 'flex', flexDirection: 'column', width: '400px' },
            children: [
                { tag: 'div', id: 'a', style: { height: '50px', marginBottom: '20px', backgroundColor: '#ff0000' } },
                { tag: 'div', id: 'b', style: { height: '50px', backgroundColor: '#00ff00' } },
            ],
        },
    },
    {
        label: 'margin-auto-center',
        category: 'margin',
        tree: {
            tag: 'div', id: 'root',
            style: { width: '400px', height: '200px' },
            children: [
                { tag: 'div', id: 'centered', style: { width: '200px', height: '100px', margin: '0 auto', backgroundColor: '#ff0000' } },
            ],
        },
    },

    // ─── 6. Nesting ───

    {
        label: 'nested-row-in-column',
        category: 'nesting',
        tree: {
            tag: 'div', id: 'outer',
            style: { display: 'flex', flexDirection: 'column', width: '500px', gap: '10px' },
            children: [
                {
                    tag: 'div', id: 'row',
                    style: { display: 'flex', flexDirection: 'row', gap: '10px' },
                    children: [
                        { tag: 'div', id: 'r1', style: { width: '100px', height: '60px', backgroundColor: '#ff0000' } },
                        { tag: 'div', id: 'r2', style: { width: '100px', height: '60px', backgroundColor: '#00ff00' } },
                    ],
                },
                { tag: 'div', id: 'below', style: { height: '40px', backgroundColor: '#0000ff' } },
            ],
        },
    },
    {
        label: 'nested-column-in-row',
        category: 'nesting',
        tree: {
            tag: 'div', id: 'outer',
            style: { display: 'flex', flexDirection: 'row', width: '600px', height: '300px' },
            children: [
                {
                    tag: 'div', id: 'col',
                    style: { display: 'flex', flexDirection: 'column', width: '200px', gap: '5px' },
                    children: [
                        { tag: 'div', id: 'c1', style: { height: '80px', backgroundColor: '#ff0000' } },
                        { tag: 'div', id: 'c2', style: { height: '80px', backgroundColor: '#00ff00' } },
                    ],
                },
                { tag: 'div', id: 'side', style: { flex: '1', backgroundColor: '#0000ff' } },
            ],
        },
    },
    {
        label: 'nested-3-levels',
        category: 'nesting',
        tree: {
            tag: 'div', id: 'l1',
            style: { display: 'flex', flexDirection: 'column', width: '500px', padding: '10px' },
            children: [
                {
                    tag: 'div', id: 'l2',
                    style: { display: 'flex', flexDirection: 'row', gap: '10px', padding: '5px' },
                    children: [
                        {
                            tag: 'div', id: 'l3',
                            style: { display: 'flex', flexDirection: 'column', width: '150px' },
                            children: [
                                { tag: 'div', id: 'leaf1', style: { height: '30px', backgroundColor: '#ff0000' } },
                                { tag: 'div', id: 'leaf2', style: { height: '30px', backgroundColor: '#00ff00' } },
                            ],
                        },
                        { tag: 'div', id: 'leaf3', style: { flex: '1', height: '80px', backgroundColor: '#0000ff' } },
                    ],
                },
            ],
        },
    },

    // ─── 7. Alignment Combinations ───

    {
        label: 'align-stretch',
        category: 'alignment',
        tree: {
            tag: 'div', id: 'root',
            style: { display: 'flex', flexDirection: 'row', alignItems: 'stretch', width: '400px', height: '200px' },
            children: [
                { tag: 'div', id: 'a', style: { width: '100px', backgroundColor: '#ff0000' } },
                { tag: 'div', id: 'b', style: { width: '100px', backgroundColor: '#00ff00' } },
            ],
        },
    },
    {
        label: 'justify-space-between',
        category: 'alignment',
        tree: {
            tag: 'div', id: 'root',
            style: { display: 'flex', flexDirection: 'row', justifyContent: 'space-between', width: '400px' },
            children: [
                { tag: 'div', id: 'a', style: { width: '80px', height: '50px', backgroundColor: '#ff0000' } },
                { tag: 'div', id: 'b', style: { width: '80px', height: '50px', backgroundColor: '#00ff00' } },
                { tag: 'div', id: 'c', style: { width: '80px', height: '50px', backgroundColor: '#0000ff' } },
            ],
        },
    },
    {
        label: 'align-center-justify-center',
        category: 'alignment',
        tree: {
            tag: 'div', id: 'root',
            style: { display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', width: '400px', height: '400px' },
            children: [
                { tag: 'div', id: 'a', style: { width: '200px', height: '80px', backgroundColor: '#ff0000' } },
                { tag: 'div', id: 'b', style: { width: '150px', height: '60px', backgroundColor: '#00ff00' } },
            ],
        },
    },

    // ─── 8. Absolute Position ───

    {
        label: 'position-absolute',
        category: 'position',
        tree: {
            tag: 'div', id: 'root',
            style: { position: 'relative', width: '400px', height: '300px' },
            children: [
                { tag: 'div', id: 'flow', style: { height: '50px', backgroundColor: '#ff0000' } },
                { tag: 'div', id: 'abs', style: { position: 'absolute', top: '100px', left: '50px', width: '200px', height: '100px', backgroundColor: '#00ff00' } },
            ],
        },
    },

    // ─── 9. Overflow ───

    {
        label: 'overflow-hidden',
        category: 'overflow',
        tree: {
            tag: 'div', id: 'root',
            style: { width: '200px', height: '100px', overflow: 'hidden' },
            children: [
                { tag: 'div', id: 'tall', style: { height: '300px', backgroundColor: '#ff0000' } },
            ],
        },
    },

    // ─── 10. Apple Page Sections (real-world patterns) ───

    {
        label: 'apple-hero-section',
        category: 'real-page',
        tree: {
            tag: 'section', id: 'hero',
            style: { width: '100%', backgroundColor: '#000000', display: 'flex', flexDirection: 'column', alignItems: 'center', padding: '120px 0 60px' },
            children: [
                { tag: 'h1', id: 'title', style: { fontSize: '56px', fontWeight: '700', color: '#ffffff' }, text: 'iPhone 16 Pro' },
                { tag: 'p', id: 'subtitle', style: { fontSize: '28px', color: '#86868b', marginTop: '6px' }, text: 'Hello, Apple Intelligence.' },
                {
                    tag: 'div', id: 'links',
                    style: { display: 'flex', flexDirection: 'row', gap: '24px', marginTop: '20px' },
                    children: [
                        { tag: 'span', id: 'link1', style: { fontSize: '21px', color: '#0071e3' }, text: 'Learn more >' },
                        { tag: 'span', id: 'link2', style: { fontSize: '21px', color: '#0071e3' }, text: 'Buy >' },
                    ],
                },
            ],
        },
    },
    {
        label: 'apple-product-grid',
        category: 'real-page',
        tree: {
            tag: 'div', id: 'grid',
            style: { width: '100%', maxWidth: '800px', display: 'flex', flexDirection: 'row', flexWrap: 'wrap', gap: '12px', padding: '12px' },
            children: [
                {
                    tag: 'div', id: 'card1',
                    style: { flex: '1', minWidth: '300px', height: '400px', backgroundColor: '#000000', borderRadius: '18px', display: 'flex', flexDirection: 'column', alignItems: 'center', padding: '48px 40px' },
                    children: [
                        { tag: 'h2', id: 'card1-title', style: { fontSize: '40px', fontWeight: '700', color: '#ffffff' }, text: 'MacBook Pro' },
                    ],
                },
                {
                    tag: 'div', id: 'card2',
                    style: { flex: '1', minWidth: '300px', height: '400px', backgroundColor: '#f5f5f7', borderRadius: '18px', display: 'flex', flexDirection: 'column', alignItems: 'center', padding: '48px 40px' },
                    children: [
                        { tag: 'h2', id: 'card2-title', style: { fontSize: '40px', fontWeight: '700', color: '#1d1d1f' }, text: 'MacBook Air' },
                    ],
                },
            ],
        },
    },
    {
        label: 'apple-footer',
        category: 'real-page',
        tree: {
            tag: 'footer', id: 'footer',
            style: { width: '100%', backgroundColor: '#f5f5f7', padding: '20px 0', display: 'flex', flexDirection: 'column', alignItems: 'center' },
            children: [
                {
                    tag: 'div', id: 'footer-inner',
                    style: { maxWidth: '980px', width: '100%', padding: '0 22px', display: 'flex', flexDirection: 'row', gap: '40px', justifyContent: 'center' },
                    children: [
                        {
                            tag: 'div', id: 'col1',
                            style: { display: 'flex', flexDirection: 'column', gap: '8px' },
                            children: [
                                { tag: 'span', id: 'col1-title', style: { fontSize: '12px', fontWeight: '700' }, text: 'Shop and Learn' },
                                { tag: 'span', id: 'col1-link1', style: { fontSize: '12px', color: '#6e6e73' }, text: 'Store' },
                                { tag: 'span', id: 'col1-link2', style: { fontSize: '12px', color: '#6e6e73' }, text: 'Mac' },
                            ],
                        },
                        {
                            tag: 'div', id: 'col2',
                            style: { display: 'flex', flexDirection: 'column', gap: '8px' },
                            children: [
                                { tag: 'span', id: 'col2-title', style: { fontSize: '12px', fontWeight: '700' }, text: 'Services' },
                                { tag: 'span', id: 'col2-link1', style: { fontSize: '12px', color: '#6e6e73' }, text: 'Apple Music' },
                            ],
                        },
                    ],
                },
            ],
        },
    },
];
