// ═══════════════════════════════════════════════════════════════
// Apple Website Clone — React Version
// ═══════════════════════════════════════════════════════════════
//
// This is a polished Apple-style landing page using real React
// and real react-dom. On web it renders to the normal browser DOM.
// On native, the DOM shim intercepts everything and routes to Rendero.
//
// ZERO framework-specific code. Just standard React.

import React, { useState, useEffect, useRef } from 'react';

// ─── Styles (inline objects, like React Native but CSS property names) ───

const colors = {
    black: '#1d1d1f',
    darkBg: '#000000',
    lightBg: '#f5f5f7',
    white: '#ffffff',
    blue: '#0071e3',
    blueHover: '#0077ed',
    gray: '#86868b',
    grayLight: '#d2d2d7',
    text: '#1d1d1f',
    textSecondary: '#6e6e73',
    navBg: 'rgba(22,22,23,0.8)',
};

const s = {
    // ─── Nav ───
    nav: {
        position: 'fixed',
        top: '0',
        left: '0',
        width: '100%',
        height: '44px',
        backgroundColor: colors.navBg,
        display: 'flex',
        flexDirection: 'row',
        alignItems: 'center',
        justifyContent: 'center',
        zIndex: '100',
    },
    navInner: {
        display: 'flex',
        flexDirection: 'row',
        alignItems: 'center',
        gap: '32px',
        maxWidth: '980px',
        width: '100%',
        padding: '0 22px',
    },
    navLogo: {
        fontSize: '18px',
        fontWeight: '600',
        color: colors.white,
    },
    navLink: {
        fontSize: '12px',
        color: '#d2d2d7',
        fontWeight: '400',
        cursor: 'pointer',
    },

    // ─── Hero Section ───
    heroSection: {
        width: '100%',
        backgroundColor: colors.darkBg,
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        padding: '120px 0 60px',
    },
    heroTitle: {
        fontSize: '56px',
        fontWeight: '700',
        color: colors.white,
        textAlign: 'center',
        lineHeight: '1.05',
    },
    heroSubtitle: {
        fontSize: '28px',
        fontWeight: '400',
        color: colors.gray,
        textAlign: 'center',
        marginTop: '6px',
    },
    heroLinks: {
        display: 'flex',
        flexDirection: 'row',
        gap: '24px',
        marginTop: '20px',
    },
    heroLink: {
        fontSize: '21px',
        color: colors.blue,
        fontWeight: '400',
        cursor: 'pointer',
    },
    heroBadge: {
        display: 'flex',
        flexDirection: 'row',
        alignItems: 'center',
        gap: '8px',
        marginTop: '40px',
        backgroundColor: 'rgba(255,255,255,0.08)',
        borderRadius: '20px',
        padding: '8px 20px',
    },
    heroBadgeText: {
        fontSize: '14px',
        color: colors.grayLight,
    },
    heroImage: {
        width: '700px',
        height: '400px',
        marginTop: '40px',
        borderRadius: '20px',
        background: 'linear-gradient(135deg, #1a1a2e 0%, #16213e 50%, #0f3460 100%)',
    },

    // ─── Product Grid ───
    gridSection: {
        width: '100%',
        maxWidth: '1200px',
        margin: '0 auto',
        padding: '12px',
        display: 'flex',
        flexDirection: 'row',
        flexWrap: 'wrap',
        gap: '12px',
    },
    card: {
        flex: '1',
        minWidth: '400px',
        height: '500px',
        borderRadius: '18px',
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        padding: '48px 40px',
        overflow: 'hidden',
        position: 'relative',
    },
    cardDark: {
        backgroundColor: colors.darkBg,
    },
    cardLight: {
        backgroundColor: colors.lightBg,
    },
    cardTitle: {
        fontSize: '40px',
        fontWeight: '700',
        textAlign: 'center',
        lineHeight: '1.1',
    },
    cardSubtitle: {
        fontSize: '21px',
        fontWeight: '400',
        textAlign: 'center',
        marginTop: '6px',
    },
    cardLinks: {
        display: 'flex',
        flexDirection: 'row',
        gap: '20px',
        marginTop: '14px',
    },
    cardLink: {
        fontSize: '17px',
        color: colors.blue,
        cursor: 'pointer',
    },
    cardImage: {
        width: '100%',
        height: '280px',
        marginTop: '30px',
        borderRadius: '16px',
    },

    // ─── Feature Section ───
    featureSection: {
        width: '100%',
        backgroundColor: colors.lightBg,
        padding: '80px 0',
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
    },
    featureGrid: {
        display: 'flex',
        flexDirection: 'row',
        gap: '30px',
        maxWidth: '980px',
        width: '100%',
        padding: '0 22px',
        marginTop: '50px',
    },
    featureCard: {
        flex: '1',
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        gap: '16px',
        padding: '30px',
    },
    featureIcon: {
        width: '56px',
        height: '56px',
        borderRadius: '14px',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
    },
    featureTitle: {
        fontSize: '24px',
        fontWeight: '700',
        color: colors.text,
        textAlign: 'center',
    },
    featureDesc: {
        fontSize: '14px',
        color: colors.textSecondary,
        textAlign: 'center',
        lineHeight: '1.5',
    },

    // ─── CTA ───
    ctaSection: {
        width: '100%',
        backgroundColor: colors.darkBg,
        padding: '100px 0',
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        gap: '20px',
    },
    ctaTitle: {
        fontSize: '48px',
        fontWeight: '700',
        color: colors.white,
        textAlign: 'center',
    },
    ctaSubtitle: {
        fontSize: '21px',
        color: colors.gray,
        textAlign: 'center',
        maxWidth: '600px',
    },
    ctaButton: {
        backgroundColor: colors.blue,
        color: colors.white,
        fontSize: '17px',
        fontWeight: '600',
        padding: '12px 28px',
        borderRadius: '980px',
        cursor: 'pointer',
        border: 'none',
        marginTop: '10px',
    },

    // ─── Footer ───
    footer: {
        width: '100%',
        backgroundColor: colors.lightBg,
        padding: '20px 0',
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
    },
    footerInner: {
        maxWidth: '980px',
        width: '100%',
        padding: '0 22px',
        display: 'flex',
        flexDirection: 'column',
        gap: '20px',
    },
    footerRow: {
        display: 'flex',
        flexDirection: 'row',
        gap: '40px',
        justifyContent: 'center',
    },
    footerCol: {
        display: 'flex',
        flexDirection: 'column',
        gap: '8px',
    },
    footerColTitle: {
        fontSize: '12px',
        fontWeight: '700',
        color: colors.text,
    },
    footerLink: {
        fontSize: '12px',
        color: colors.textSecondary,
    },
    footerCopy: {
        fontSize: '12px',
        color: colors.textSecondary,
        textAlign: 'center',
        paddingTop: '20px',
        borderTop: '1px solid #d2d2d7',
    },
};

// ─── Components ───

function NavBar() {
    const links = ['Store', 'Mac', 'iPad', 'iPhone', 'Watch', 'Vision', 'AirPods', 'TV & Home', 'Entertainment', 'Accessories', 'Support'];

    return (
        <nav style={s.nav}>
            <div style={s.navInner}>
                <span style={s.navLogo}>&#63743;</span>
                {links.map(link => (
                    <span key={link} style={s.navLink}>{link}</span>
                ))}
            </div>
        </nav>
    );
}

function HeroSection() {
    return (
        <section style={s.heroSection}>
            <div style={s.heroBadge}>
                <span style={{ ...s.heroBadgeText, color: '#ff6b35' }}>New</span>
                <span style={s.heroBadgeText}>Apple Intelligence everywhere.</span>
            </div>
            <h1 style={s.heroTitle}>iPhone 16 Pro</h1>
            <p style={s.heroSubtitle}>Hello, Apple Intelligence.</p>
            <div style={s.heroLinks}>
                <span style={s.heroLink}>Learn more &gt;</span>
                <span style={s.heroLink}>Buy &gt;</span>
            </div>
            <div style={s.heroImage} />
        </section>
    );
}

function ProductCard({ title, subtitle, dark, gradient, children }) {
    const cardStyle = {
        ...s.card,
        ...(dark ? s.cardDark : s.cardLight),
    };
    const titleColor = dark ? colors.white : colors.text;
    const subtitleColor = dark ? colors.gray : colors.textSecondary;

    return (
        <div style={cardStyle}>
            <h2 style={{ ...s.cardTitle, color: titleColor }}>{title}</h2>
            <p style={{ ...s.cardSubtitle, color: subtitleColor }}>{subtitle}</p>
            <div style={s.cardLinks}>
                <span style={s.cardLink}>Learn more &gt;</span>
                <span style={s.cardLink}>Buy &gt;</span>
            </div>
            <div style={{
                ...s.cardImage,
                background: gradient || 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)',
            }} />
        </div>
    );
}

function ProductGrid() {
    return (
        <div style={s.gridSection}>
            <ProductCard
                title="MacBook Pro"
                subtitle="Mind-blowing. Head-turning."
                dark
                gradient="linear-gradient(135deg, #0c0c0c 0%, #2d2d2d 50%, #1a1a1a 100%)"
            />
            <ProductCard
                title="MacBook Air"
                subtitle="Lean. Mean. M3 machine."
                dark={false}
                gradient="linear-gradient(135deg, #e0c3fc 0%, #8ec5fc 100%)"
            />
            <ProductCard
                title="iPad Pro"
                subtitle="Thinpossible."
                dark
                gradient="linear-gradient(135deg, #0f0c29 0%, #302b63 50%, #24243e 100%)"
            />
            <ProductCard
                title="iPad Air"
                subtitle="Fresh Air."
                dark={false}
                gradient="linear-gradient(135deg, #a8edea 0%, #fed6e3 100%)"
            />
        </div>
    );
}

function FeatureSection() {
    const features = [
        {
            icon: '🧠',
            color: 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)',
            title: 'Apple Intelligence',
            desc: 'Personal, private, powerful AI that understands you.',
        },
        {
            icon: '🔒',
            color: 'linear-gradient(135deg, #11998e 0%, #38ef7d 100%)',
            title: 'Privacy',
            desc: 'Your data stays on your device. Period.',
        },
        {
            icon: '♻️',
            color: 'linear-gradient(135deg, #f093fb 0%, #f5576c 100%)',
            title: 'Environment',
            desc: 'Carbon neutral by 2030. We mean it.',
        },
    ];

    return (
        <section style={s.featureSection}>
            <h2 style={{ fontSize: '48px', fontWeight: '700', color: colors.text, textAlign: 'center' }}>
                Why Apple.
            </h2>
            <div style={s.featureGrid}>
                {features.map(f => (
                    <div key={f.title} style={s.featureCard}>
                        <div style={{ ...s.featureIcon, background: f.color }}>
                            <span style={{ fontSize: '28px' }}>{f.icon}</span>
                        </div>
                        <h3 style={s.featureTitle}>{f.title}</h3>
                        <p style={s.featureDesc}>{f.desc}</p>
                    </div>
                ))}
            </div>
        </section>
    );
}

function CTASection() {
    const [clicked, setClicked] = useState(false);

    return (
        <section style={s.ctaSection}>
            <h2 style={s.ctaTitle}>
                {clicked ? 'Coming right up.' : 'Get the new iPhone.'}
            </h2>
            <p style={s.ctaSubtitle}>
                From $999 or $41.62/mo. for 24 mo. before trade-in.
            </p>
            <div
                style={{
                    ...s.ctaButton,
                    backgroundColor: clicked ? '#34c759' : colors.blue,
                }}
                onClick={() => setClicked(true)}
            >
                {clicked ? 'Ordered ✓' : 'Buy now'}
            </div>
        </section>
    );
}

function Footer() {
    const columns = [
        { title: 'Shop and Learn', links: ['Store', 'Mac', 'iPad', 'iPhone', 'Watch', 'AirPods'] },
        { title: 'Services', links: ['Apple Music', 'Apple TV+', 'Apple Arcade', 'iCloud', 'Apple One'] },
        { title: 'Account', links: ['Manage Your Apple ID', 'Apple Store Account', 'iCloud.com'] },
        { title: 'Apple Values', links: ['Accessibility', 'Environment', 'Privacy', 'Supplier Responsibility'] },
    ];

    return (
        <footer style={s.footer}>
            <div style={s.footerInner}>
                <div style={s.footerRow}>
                    {columns.map(col => (
                        <div key={col.title} style={s.footerCol}>
                            <span style={s.footerColTitle}>{col.title}</span>
                            {col.links.map(link => (
                                <span key={link} style={s.footerLink}>{link}</span>
                            ))}
                        </div>
                    ))}
                </div>
                <p style={s.footerCopy}>
                    Copyright 2024 Apple Inc. All rights reserved.
                </p>
            </div>
        </footer>
    );
}

// ─── Test: Simple colored rectangles ───

function TestApp() {
    return (
        <div style={{ width: '100%', height: '100vh', backgroundColor: '#ffffff', display: 'flex', flexDirection: 'column' }}>
            <div style={{ width: '100%', height: '100px', backgroundColor: '#ff0000' }} />
            <div style={{ width: '100%', height: '100px', backgroundColor: '#00ff00' }} />
            <div style={{ width: '100%', height: '100px', backgroundColor: '#0000ff' }} />
            <div style={{ width: '50%', height: '200px', backgroundColor: '#ff00ff' }} />
        </div>
    );
}

// ─── App Root: swap between TestApp and full Apple page ───
// Comment/uncomment to switch:

function AppleApp() {
    return (
        <div style={{ width: '100%', minHeight: '100vh' }}>
            <NavBar />
            <HeroSection />
            <ProductGrid />
            <FeatureSection />
            <CTASection />
            <Footer />
        </div>
    );
}
// Switch between AppleApp and TestApp in one line now.
export default function App() { return <AppleApp />; }
