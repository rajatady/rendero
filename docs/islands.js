/**
 * islands.js — Build all island content in Engine B (Canvas2D)
 *
 * Each island is a cluster of content (title, description, screenshot, stats)
 * floating at a specific position in the world.
 */

// ─── Island positions (world coordinates, centered at 0,0) ───
export const ISLANDS = {
    home:       { x: 0,     y: 0,     label: 'Home' },
    earthquake: { x: -2200, y: -200,  label: 'Earthquake' },
    neural:     { x: 0,     y: -1800, label: 'Neural Net' },
    design:     { x: 2200,  y: -200,  label: 'Design Tool' },
    splat:      { x: -1400, y: 1600,  label: 'Splats' },
    genome:     { x: 1400,  y: 1600,  label: 'Genome' },
    code:       { x: 0,     y: 3200,  label: 'Code' },
};

// ─── Colors ───
const C = {
    bg:        [0.020, 0.020, 0.027, 1.0],
    surface:   [0.039, 0.039, 0.071, 0.92],
    border:    [0.102, 0.102, 0.180, 1.0],
    text:      [0.659, 0.678, 0.722, 1.0],
    textDim:   [0.357, 0.384, 0.443, 1.0],
    textBright:[0.910, 0.925, 0.957, 1.0],
    accent:    [1.000, 0.420, 0.290, 1.0],
    accent2:   [0.290, 0.878, 1.000, 1.0],
    green:     [0.290, 0.875, 0.498, 1.0],
};

function nid(arr) { return [arr[0], arr[1]]; }

/** Create a text node truly centered at (cx, y). */
function addCenteredText(engine, name, text, cx, y, size, r, g, b, a, fontFamily, fontWeight) {
    const id = nid(engine.add_text(name, text, cx, y, size, r, g, b, a));
    engine.set_node_font_family(id[0], id[1], fontFamily);
    if (fontWeight) engine.set_node_font_weight(id[0], id[1], fontWeight);
    engine.set_text_align(id[0], id[1], "center");
    // Node is placed at (cx, y) as top-left. "center" draws text at cx + width/2.
    // To truly center at cx, shift left by width/2.
    const bounds = engine.get_node_world_bounds(id[0], id[1]);
    const w = bounds[2];
    engine.set_node_position(id[0], id[1], cx - w / 2, y);
    return id;
}

async function loadImageData(url) {
    const resp = await fetch(url);
    const blob = await resp.blob();
    const bmp = await createImageBitmap(blob);
    const w = bmp.width, h = bmp.height;
    const c = document.createElement('canvas');
    c.width = w; c.height = h;
    const cx = c.getContext('2d');
    cx.drawImage(bmp, 0, 0);
    const imgData = cx.getImageData(0, 0, w, h);
    bmp.close();
    return { data: new Uint8Array(imgData.data.buffer), width: w, height: h };
}

// ─── Token colors for code syntax highlighting ───
const TC = {
    kw: [0.783, 0.573, 0.918, 1.0],
    st: [0.765, 0.910, 0.553, 1.0],
    fn: [0.510, 0.667, 1.000, 1.0],
    nm: [0.969, 0.549, 0.424, 1.0],
    op: [0.537, 0.867, 1.000, 1.0],
    cm: [0.290, 0.337, 0.416, 1.0],
    ln: [0.200, 0.200, 0.200, 1.0],
    pl: [0.659, 0.678, 0.722, 1.0],
};

/**
 * Build all islands. Returns click regions.
 */
export async function buildIslands(engine) {
    const clickRegions = [];

    // ═══════════════════════════════════════
    // CENTER ISLAND — "rendero."
    // ═══════════════════════════════════════
    const home = ISLANDS.home;

    // Particle text "rendero." is handled by particles.js
    // Add subtitle + stats below it
    addCenteredText(engine, "home-sub", "A general-purpose WebAssembly rendering engine", home.x, home.y + 140, 18, ...C.text, "Space Mono");

    // Tagline
    addCenteredText(engine, "home-tag", "Spatial culling · WebGL2 · CRDT · Scene graph · 494KB gzipped", home.x, home.y + 175, 11, ...C.textDim, "Space Mono");

    // Navigation hint
    const isMobile = window.innerWidth < 640;
    const hintText = isMobile ? "Drag to pan · Pinch to zoom · Tap nav below" : "Drag to pan · Scroll to zoom · Click nav below";
    addCenteredText(engine, "home-hint", hintText, home.x, home.y + 220, 10, ...C.textDim, "Space Mono");

    // ═══════════════════════════════════════
    // DEMO ISLANDS
    // ═══════════════════════════════════════
    const demos = [
        { key: 'earthquake', title: "Earthquake Explorer", badge: "WebGL2 + Map Tiles", stats: "14K pts · 120 FPS", shot: "shots/earthquake.jpg", url: "./demos/earthquake-explorer/", desc: "Live USGS seismic data on CartoDB Dark Matter tiles." },
        { key: 'neural', title: "Neural Net Visualizer", badge: "134M Points", stats: "134M pts · 120 FPS", shot: "shots/neural-net.jpg", url: "./demos/neural-net/", desc: "Every weight of SmolLM2-135M as a colored pixel." },
        { key: 'design', title: "Design Tool", badge: "Full Editor", stats: "Canvas2D · 120 FPS", shot: "shots/design-tool.jpg", url: "./demos/design-tool/", desc: "Figma-like vector editor with layers, pen tool, .fig import." },
        { key: 'splat', title: "Gaussian Splat Viewer", badge: "2.5M Splats", stats: "2.5M splats · 120 FPS", shot: "shots/splat-viewer.jpg", url: "./demos/splat-viewer/", desc: "3D Gaussian Splatting with real-time depth sorting." },
        { key: 'genome', title: "Genome Browser", badge: "234K Exons", stats: "28K genes · 120 FPS", shot: "shots/genome-browser.jpg", url: "./demos/genome-browser/", desc: "Human genome GRCh38 across 24 chromosomes." },
    ];

    // Load all images in parallel
    const imgLoads = demos.map(d => loadImageData(d.shot).catch(() => null));
    const images = await Promise.all(imgLoads);

    const CARD_W = 480;
    const CARD_H = 340;
    const THUMB_H = 220;

    for (let i = 0; i < demos.length; i++) {
        const d = demos[i];
        const island = ISLANDS[d.key];
        const cx = island.x;
        const cy = island.y;
        const x = cx - CARD_W / 2;
        const y = cy - CARD_H / 2;

        // Card background
        const bg = nid(engine.add_rounded_rect(`card-bg-${i}`, x, y, CARD_W, CARD_H, 0.025, 0.025, 0.045, 0.9, 12));
        engine.set_node_stroke(bg[0], bg[1], 0.102, 0.102, 0.18, 0.5, 1);

        // Screenshot
        const img = images[i];
        if (img) {
            engine.add_image(`card-img-${i}`, x + 2, y + 2, CARD_W - 4, THUMB_H, img.data, img.width, img.height);
        }

        // Title
        const tid = nid(engine.add_text(`card-title-${i}`, d.title, x + 18, y + THUMB_H + 14, 17, ...C.textBright));
        engine.set_node_font_family(tid[0], tid[1], "Syne");
        engine.set_node_font_weight(tid[0], tid[1], 700);

        // Badge
        const bidId = nid(engine.add_text(`card-badge-${i}`, d.badge, x + 18, y + THUMB_H + 40, 10, ...C.accent2));
        engine.set_node_font_family(bidId[0], bidId[1], "Space Mono");

        // Description
        const descId = nid(engine.add_text(`card-desc-${i}`, d.desc, x + 18, y + THUMB_H + 60, 12, ...C.textDim));
        engine.set_node_font_family(descId[0], descId[1], "Syne");

        // Stats (right-aligned)
        const stId = nid(engine.add_text(`card-stat-${i}`, d.stats, x + CARD_W - 18, y + THUMB_H + 40, 9, ...C.textDim));
        engine.set_node_font_family(stId[0], stId[1], "Space Mono");
        engine.set_text_align(stId[0], stId[1], "right");

        // "Open Demo →" label
        const openId = nid(engine.add_text(`card-open-${i}`, "Open Demo →", x + CARD_W - 18, y + THUMB_H + 14, 11, ...C.accent2));
        engine.set_node_font_family(openId[0], openId[1], "Space Mono");
        engine.set_text_align(openId[0], openId[1], "right");

        // Click region for the whole card
        clickRegions.push({ x, y, w: CARD_W, h: CARD_H, action: 'url', target: d.url });
    }

    // ═══════════════════════════════════════
    // CODE ISLAND
    // ═══════════════════════════════════════
    const code = ISLANDS.code;
    const codeX = code.x - 300;
    const codeY = code.y - 160;

    // Title
    addCenteredText(engine, "code-title", "Get started in 10 lines", code.x, codeY - 50, 32, ...C.textBright, "Syne", 700);

    // Code block background
    const codeBgW = 600;
    const codeBgH = 310;
    const codeBgX = code.x - codeBgW / 2;
    engine.add_rounded_rect("code-bg", codeBgX, codeY, codeBgW, codeBgH, 0.030, 0.030, 0.050, 0.95, 10);

    // "main.js" label
    const clId = nid(engine.add_text("code-label", "main.js", codeBgX + codeBgW - 70, codeY + 8, 9, ...TC.cm));
    engine.set_node_font_family(clId[0], clId[1], "Space Mono");

    // Code lines (token-based syntax highlighting)
    const LINE_H = 21;
    const FONT_SZ = 12;
    const CODE_LEFT = codeBgX + 50;

    const codeLines = [
        [[" 1", "ln"], ["import", "kw"], [" init, { CanvasEngine } ", "pl"], ["from", "kw"], [" ", "pl"], ["'./pkg/rendero.js'", "st"], [";", "op"]],
        [[" 2", "ln"], ["await", "kw"], [" ", "pl"], ["init", "fn"], ["();", "op"]],
        [[" 3", "ln"]],
        [[" 4", "ln"], ["const", "kw"], [" engine = ", "pl"], ["new", "kw"], [" ", "pl"], ["CanvasEngine", "fn"], ["(", "op"], ["\"MyApp\"", "st"], [", ", "pl"], ["1", "nm"], [")", "op"], [";", "op"]],
        [[" 5", "ln"], ["const", "kw"], [" ctx = canvas.", "pl"], ["getContext", "fn"], ["(", "op"], ["'2d'", "st"], [")", "op"], [";", "op"]],
        [[" 6", "ln"], ["engine.", "pl"], ["set_viewport", "fn"], ["(", "op"], ["1920", "nm"], [", ", "pl"], ["1080", "nm"], [")", "op"], [";", "op"]],
        [[" 7", "ln"]],
        [[" 8", "ln"], ["// Add shapes — each becomes a scene graph node", "cm"]],
        [[" 9", "ln"], ["engine.", "pl"], ["add_rectangle", "fn"], ["(", "op"], ["\"bg\"", "st"], [", ", "pl"], ["0", "nm"], [", ", "pl"], ["0", "nm"], [", ", "pl"], ["400", "nm"], [", ", "pl"], ["300", "nm"], [", ", "pl"], ["0.2", "nm"], [", ", "pl"], ["0.5", "nm"], [", ", "pl"], ["1", "nm"], [", ", "pl"], ["1", "nm"], [")", "op"], [";", "op"]],
        [["10", "ln"], ["engine.", "pl"], ["add_text", "fn"], ["(", "op"], ["\"hi\"", "st"], [", ", "pl"], ["\"Hello\"", "st"], [", ", "pl"], ["20", "nm"], [", ", "pl"], ["20", "nm"], [", ", "pl"], ["32", "nm"], [", ", "pl"], ["1", "nm"], [",", "pl"], ["1", "nm"], [",", "pl"], ["1", "nm"], [",", "pl"], ["1", "nm"], [")", "op"], [";", "op"]],
        [["11", "ln"]],
        [["12", "ln"], ["// Render — pan, zoom, select, undo/redo all built in", "cm"]],
        [["13", "ln"], ["engine.", "pl"], ["render_canvas2d", "fn"], ["(ctx, ", "op"], ["1920", "nm"], [", ", "pl"], ["1080", "nm"], [", ", "pl"], ["1", "nm"], [")", "op"], [";", "op"]],
    ];

    const charW = 7.2;
    codeLines.forEach((tokens, lineIdx) => {
        const ly = codeY + 25 + lineIdx * LINE_H;
        let xOff = 0;

        tokens.forEach((token, ti) => {
            const text = token[0];
            const colorKey = token[1];
            const color = TC[colorKey] || TC.pl;

            // Line numbers get special position
            if (ti === 0 && colorKey === 'ln') {
                const lnId = nid(engine.add_text(`cl-${lineIdx}-ln`, text, CODE_LEFT - 30, ly, FONT_SZ, ...color));
                engine.set_node_font_family(lnId[0], lnId[1], "Space Mono");
                return;
            }

            const nodeId = nid(engine.add_text(`cl-${lineIdx}-${ti}`, text, CODE_LEFT + xOff, ly, FONT_SZ, ...color));
            engine.set_node_font_family(nodeId[0], nodeId[1], "Space Mono");
            xOff += text.length * charW;
        });
    });

    // Install hint below code
    addCenteredText(engine, "install-hint", "npm install rendero", code.x, codeY + codeBgH + 30, 13, ...C.accent2, "Space Mono");

    // GitHub link
    addCenteredText(engine, "gh-link", "github.com/nickhash/rendero", code.x, codeY + codeBgH + 55, 11, ...C.textDim, "Space Mono");

    return { clickRegions };
}
