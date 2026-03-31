// ═══════════════════════════════════════════════════════════════
// Apple Website Clone — Vue Version
// ═══════════════════════════════════════════════════════════════
//
// Same polished Apple landing page, written with Vue 3 Composition API.
// Uses h() render functions (no .vue SFC needed, no compiler).
// On web → real browser DOM. On native → DOM shim → Rendero.

import { createApp, h, ref, reactive } from 'vue';

const colors = {
    black: '#1d1d1f',
    darkBg: '#000000',
    lightBg: '#f5f5f7',
    white: '#ffffff',
    blue: '#0071e3',
    gray: '#86868b',
    grayLight: '#d2d2d7',
    text: '#1d1d1f',
    textSecondary: '#6e6e73',
    navBg: 'rgba(22,22,23,0.8)',
};

// ─── Components ───

const NavBar = {
    setup() {
        const links = ['Store', 'Mac', 'iPad', 'iPhone', 'Watch', 'Vision', 'AirPods', 'TV & Home', 'Entertainment', 'Accessories', 'Support'];
        return () => h('nav', {
            style: {
                position: 'fixed', top: '0', left: '0', width: '100%', height: '44px',
                backgroundColor: colors.navBg, display: 'flex', flexDirection: 'row',
                alignItems: 'center', justifyContent: 'center', zIndex: '100',
            },
        }, [
            h('div', {
                style: {
                    display: 'flex', flexDirection: 'row', alignItems: 'center',
                    gap: '32px', maxWidth: '980px', width: '100%', padding: '0 22px',
                },
            }, [
                h('span', { style: { fontSize: '18px', fontWeight: '600', color: colors.white } }, '\uF8FF'),
                ...links.map(link =>
                    h('span', {
                        key: link,
                        style: { fontSize: '12px', color: '#d2d2d7', fontWeight: '400', cursor: 'pointer' },
                    }, link)
                ),
            ]),
        ]);
    },
};

const HeroSection = {
    setup() {
        return () => h('section', {
            style: {
                width: '100%', backgroundColor: colors.darkBg, display: 'flex',
                flexDirection: 'column', alignItems: 'center', padding: '120px 0 60px',
            },
        }, [
            h('div', {
                style: {
                    display: 'flex', flexDirection: 'row', alignItems: 'center', gap: '8px',
                    marginTop: '0', backgroundColor: 'rgba(255,255,255,0.08)',
                    borderRadius: '20px', padding: '8px 20px',
                },
            }, [
                h('span', { style: { fontSize: '14px', color: '#ff6b35' } }, 'New'),
                h('span', { style: { fontSize: '14px', color: colors.grayLight } }, 'Apple Intelligence everywhere.'),
            ]),
            h('h1', {
                style: { fontSize: '56px', fontWeight: '700', color: colors.white, textAlign: 'center', lineHeight: '1.05' },
            }, 'iPhone 16 Pro'),
            h('p', {
                style: { fontSize: '28px', fontWeight: '400', color: colors.gray, textAlign: 'center', marginTop: '6px' },
            }, 'Hello, Apple Intelligence.'),
            h('div', {
                style: { display: 'flex', flexDirection: 'row', gap: '24px', marginTop: '20px' },
            }, [
                h('span', { style: { fontSize: '21px', color: colors.blue, cursor: 'pointer' } }, 'Learn more >'),
                h('span', { style: { fontSize: '21px', color: colors.blue, cursor: 'pointer' } }, 'Buy >'),
            ]),
            h('div', {
                style: {
                    width: '700px', height: '400px', marginTop: '40px', borderRadius: '20px',
                    background: 'linear-gradient(135deg, #1a1a2e 0%, #16213e 50%, #0f3460 100%)',
                },
            }),
        ]);
    },
};

const ProductCard = {
    props: ['title', 'subtitle', 'dark', 'gradient'],
    setup(props) {
        return () => h('div', {
            style: {
                flex: '1', minWidth: '400px', height: '500px', borderRadius: '18px',
                display: 'flex', flexDirection: 'column', alignItems: 'center',
                padding: '48px 40px', overflow: 'hidden', position: 'relative',
                backgroundColor: props.dark ? colors.darkBg : colors.lightBg,
            },
        }, [
            h('h2', {
                style: {
                    fontSize: '40px', fontWeight: '700', textAlign: 'center', lineHeight: '1.1',
                    color: props.dark ? colors.white : colors.text,
                },
            }, props.title),
            h('p', {
                style: {
                    fontSize: '21px', fontWeight: '400', textAlign: 'center', marginTop: '6px',
                    color: props.dark ? colors.gray : colors.textSecondary,
                },
            }, props.subtitle),
            h('div', {
                style: { display: 'flex', flexDirection: 'row', gap: '20px', marginTop: '14px' },
            }, [
                h('span', { style: { fontSize: '17px', color: colors.blue, cursor: 'pointer' } }, 'Learn more >'),
                h('span', { style: { fontSize: '17px', color: colors.blue, cursor: 'pointer' } }, 'Buy >'),
            ]),
            h('div', {
                style: {
                    width: '100%', height: '280px', marginTop: '30px', borderRadius: '16px',
                    background: props.gradient || 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)',
                },
            }),
        ]);
    },
};

const ProductGrid = {
    setup() {
        const products = [
            { title: 'MacBook Pro', subtitle: 'Mind-blowing. Head-turning.', dark: true, gradient: 'linear-gradient(135deg, #0c0c0c 0%, #2d2d2d 50%, #1a1a1a 100%)' },
            { title: 'MacBook Air', subtitle: 'Lean. Mean. M3 machine.', dark: false, gradient: 'linear-gradient(135deg, #e0c3fc 0%, #8ec5fc 100%)' },
            { title: 'iPad Pro', subtitle: 'Thinpossible.', dark: true, gradient: 'linear-gradient(135deg, #0f0c29 0%, #302b63 50%, #24243e 100%)' },
            { title: 'iPad Air', subtitle: 'Fresh Air.', dark: false, gradient: 'linear-gradient(135deg, #a8edea 0%, #fed6e3 100%)' },
        ];
        return () => h('div', {
            style: {
                width: '100%', maxWidth: '1200px', margin: '0 auto', padding: '12px',
                display: 'flex', flexDirection: 'row', flexWrap: 'wrap', gap: '12px',
            },
        }, products.map(p =>
            h(ProductCard, { key: p.title, ...p })
        ));
    },
};

const FeatureSection = {
    setup() {
        const features = [
            { icon: '🧠', color: 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)', title: 'Apple Intelligence', desc: 'Personal, private, powerful AI that understands you.' },
            { icon: '🔒', color: 'linear-gradient(135deg, #11998e 0%, #38ef7d 100%)', title: 'Privacy', desc: 'Your data stays on your device. Period.' },
            { icon: '♻️', color: 'linear-gradient(135deg, #f093fb 0%, #f5576c 100%)', title: 'Environment', desc: 'Carbon neutral by 2030. We mean it.' },
        ];
        return () => h('section', {
            style: {
                width: '100%', backgroundColor: colors.lightBg, padding: '80px 0',
                display: 'flex', flexDirection: 'column', alignItems: 'center',
            },
        }, [
            h('h2', {
                style: { fontSize: '48px', fontWeight: '700', color: colors.text, textAlign: 'center' },
            }, 'Why Apple.'),
            h('div', {
                style: {
                    display: 'flex', flexDirection: 'row', gap: '30px',
                    maxWidth: '980px', width: '100%', padding: '0 22px', marginTop: '50px',
                },
            }, features.map(f =>
                h('div', {
                    key: f.title,
                    style: { flex: '1', display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '16px', padding: '30px' },
                }, [
                    h('div', {
                        style: { width: '56px', height: '56px', borderRadius: '14px', display: 'flex', alignItems: 'center', justifyContent: 'center', background: f.color },
                    }, h('span', { style: { fontSize: '28px' } }, f.icon)),
                    h('h3', { style: { fontSize: '24px', fontWeight: '700', color: colors.text, textAlign: 'center' } }, f.title),
                    h('p', { style: { fontSize: '14px', color: colors.textSecondary, textAlign: 'center', lineHeight: '1.5' } }, f.desc),
                ])
            )),
        ]);
    },
};

const CTASection = {
    setup() {
        const clicked = ref(false);
        return () => h('section', {
            style: {
                width: '100%', backgroundColor: colors.darkBg, padding: '100px 0',
                display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '20px',
            },
        }, [
            h('h2', {
                style: { fontSize: '48px', fontWeight: '700', color: colors.white, textAlign: 'center' },
            }, clicked.value ? 'Coming right up.' : 'Get the new iPhone.'),
            h('p', {
                style: { fontSize: '21px', color: colors.gray, textAlign: 'center', maxWidth: '600px' },
            }, 'From $999 or $41.62/mo. for 24 mo. before trade-in.'),
            h('div', {
                style: {
                    backgroundColor: clicked.value ? '#34c759' : colors.blue,
                    color: colors.white, fontSize: '17px', fontWeight: '600',
                    padding: '12px 28px', borderRadius: '980px', cursor: 'pointer',
                    border: 'none', marginTop: '10px',
                },
                onClick: () => { clicked.value = true; },
            }, clicked.value ? 'Ordered ✓' : 'Buy now'),
        ]);
    },
};

const FooterComponent = {
    setup() {
        const columns = [
            { title: 'Shop and Learn', links: ['Store', 'Mac', 'iPad', 'iPhone', 'Watch', 'AirPods'] },
            { title: 'Services', links: ['Apple Music', 'Apple TV+', 'Apple Arcade', 'iCloud', 'Apple One'] },
            { title: 'Account', links: ['Manage Your Apple ID', 'Apple Store Account', 'iCloud.com'] },
            { title: 'Apple Values', links: ['Accessibility', 'Environment', 'Privacy', 'Supplier Responsibility'] },
        ];
        return () => h('footer', {
            style: { width: '100%', backgroundColor: colors.lightBg, padding: '20px 0', display: 'flex', flexDirection: 'column', alignItems: 'center' },
        }, [
            h('div', {
                style: { maxWidth: '980px', width: '100%', padding: '0 22px', display: 'flex', flexDirection: 'column', gap: '20px' },
            }, [
                h('div', {
                    style: { display: 'flex', flexDirection: 'row', gap: '40px', justifyContent: 'center' },
                }, columns.map(col =>
                    h('div', { key: col.title, style: { display: 'flex', flexDirection: 'column', gap: '8px' } }, [
                        h('span', { style: { fontSize: '12px', fontWeight: '700', color: colors.text } }, col.title),
                        ...col.links.map(link =>
                            h('span', { key: link, style: { fontSize: '12px', color: colors.textSecondary } }, link)
                        ),
                    ])
                )),
                h('p', {
                    style: { fontSize: '12px', color: colors.textSecondary, textAlign: 'center', paddingTop: '20px', borderTop: '1px solid #d2d2d7' },
                }, 'Copyright 2024 Apple Inc. All rights reserved.'),
            ]),
        ]);
    },
};

// ─── Root App ───

const AppleApp = {
    setup() {
        return () => h('div', { style: { width: '100%', minHeight: '100vh' } }, [
            h(NavBar),
            h(HeroSection),
            h(ProductGrid),
            h(FeatureSection),
            h(CTASection),
            h(FooterComponent),
        ]);
    },
};

export default AppleApp;
export { createApp };
