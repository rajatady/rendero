/**
 * particles.js — Galaxy background + hero text + star-lane connectors
 *
 * Galaxy spirals rotate slowly and are visible from the start.
 * Text convergence is triggered after the fly-in completes (startConverge flag).
 * After convergence, text particles react to cursor proximity / mobile tap.
 */
import { sampleText } from './text-sampler.js';
import { ISLANDS } from './islands.js';

const WORLD_W = 8000;
const WORLD_H = 6000;
const CX = 0;

function easeOutExpo(t) {
    return t >= 1 ? 1 : 1 - Math.pow(2, -10 * t);
}

/**
 * Create all particle systems.
 */
export function createParticles(engine, gl, cssW, cssH) {
    // ── Hero text particles ──
    const TEXT_COUNT = 120000;
    const { points: textPixels, canvasW: tCanvW, canvasH: tCanvH } = sampleText('rendero.', 300, "'Arial Black', 'Impact', sans-serif", TEXT_COUNT);
    const actualTextCount = textPixels.length;

    const targetTextW = 700;
    const scale = targetTextW / tCanvW;
    const textWorldW = tCanvW * scale;
    const textWorldH = tCanvH * scale;
    const textOffX = CX - textWorldW / 2;
    const textOffY = -textWorldH / 2 - 20;

    const textTargets = new Float32Array(actualTextCount * 2);
    const textStarts = new Float32Array(actualTextCount * 2);
    const textColors = new Float32Array(actualTextCount * 4);

    for (let i = 0; i < actualTextCount; i++) {
        const px = textPixels[i];
        const tx = textOffX + px.x * scale;
        const ty = textOffY + px.y * scale;
        textTargets[i * 2] = tx;
        textTargets[i * 2 + 1] = ty;

        const angle = Math.random() * Math.PI * 2;
        const dist = 200 + Math.random() * 1500;
        textStarts[i * 2] = CX + Math.cos(angle) * dist;
        textStarts[i * 2 + 1] = Math.sin(angle) * dist;

        const normX = px.x / tCanvW;
        let r, g, b;
        if (normX < 0.35) {
            const f = normX / 0.35;
            r = 0.31 + f * 0.45; g = 0.76 + f * 0.15; b = 0.97;
        } else if (normX < 0.65) {
            const f = (normX - 0.35) / 0.3;
            r = 0.76 + f * 0.1; g = 0.91 - f * 0.15; b = 0.97 + f * 0.03;
        } else {
            const f = (normX - 0.65) / 0.35;
            r = 0.86 - f * 0.2; g = 0.76 - f * 0.2; b = 1.0;
        }
        textColors[i * 4] = r;
        textColors[i * 4 + 1] = g;
        textColors[i * 4 + 2] = b;
        textColors[i * 4 + 3] = 0.88 + Math.random() * 0.12;
    }

    // ── Background galaxy ──
    const BG_COUNT = 380000;
    const bgBaseData = generateGalaxy(BG_COUNT, textOffX, textOffY, textWorldW, textWorldH);
    const bgAnimData = new Float32Array(bgBaseData.length);
    bgAnimData.set(bgBaseData);

    // ── Star lanes ──
    const laneData = generateStarLanes();

    // ── Buffers ──
    const textBuf = new Float32Array(actualTextCount * 8);

    // ── Physics arrays (used after convergence) ──
    const textPosX = new Float32Array(actualTextCount);
    const textPosY = new Float32Array(actualTextCount);
    const textVelX = new Float32Array(actualTextCount);
    const textVelY = new Float32Array(actualTextCount);

    return {
        textTargets, textStarts, textColors, textBuf,
        actualTextCount,
        bgBaseData, bgAnimData, laneData,
        textPosX, textPosY, textVelX, textVelY,
        // Convergence is triggered by main.js after fly-in
        startConverge: false,
        animStartTime: null, animDone: false,
        ANIM_DURATION: 2500,
        lastTime: 0,
        bgFrame: 0,
        textAtRest: true,
        // Explode state — set by main.js
        exploding: false,
        explodeX: 0, explodeY: 0,
        mobileExplode: false, mobileExplodeStart: 0,
        // Text bounds for hit testing
        textBoundsX: textOffX,
        textBoundsY: textOffY,
        textBoundsW: textWorldW,
        textBoundsH: textWorldH,
    };
}

/**
 * Update particles each frame.
 */
export function updateParticles(state, engine, gl, now) {
    // ── Galaxy rotation (always, every 2 frames) ──
    state.bgFrame++;
    if (state.bgFrame % 2 === 0) {
        const angle = now * 0.00003;
        const cos = Math.cos(angle);
        const sin = Math.sin(angle);
        const bgLen = state.bgBaseData.length;
        for (let o = 0; o < bgLen; o += 8) {
            const bx = state.bgBaseData[o];
            const by = state.bgBaseData[o + 1];
            state.bgAnimData[o] = bx * cos - by * sin;
            state.bgAnimData[o + 1] = bx * sin + by * cos;
        }
    }

    // ── Before convergence: galaxy + lanes only ──
    if (!state.startConverge) {
        engine.clear_point_clouds(gl);
        engine.add_point_cloud(gl, state.bgAnimData);
        engine.add_point_cloud(gl, state.laneData);
        return;
    }

    // ── Text convergence animation ──
    if (!state.animDone) {
        if (state.animStartTime === null) state.animStartTime = now;
        const elapsed = now - state.animStartTime;
        const progress = Math.min(elapsed / state.ANIM_DURATION, 1);
        const { textBuf, textStarts, textTargets, textColors, actualTextCount } = state;

        for (let i = 0; i < actualTextCount; i++) {
            const sx = textStarts[i * 2], sy = textStarts[i * 2 + 1];
            const tx = textTargets[i * 2], ty = textTargets[i * 2 + 1];

            const dist = Math.sqrt(sx * sx + sy * sy);
            const stagger = (dist / 2000) * 0.3;
            const lp = Math.max(0, Math.min(1, (progress - stagger) / (1 - stagger)));
            const ease = easeOutExpo(lp);

            const x = sx + (tx - sx) * ease;
            const y = sy + (ty - sy) * ease;
            const size = 2.5 + (1 - ease) * 4.0;
            const alpha = textColors[i * 4 + 3] * Math.min(1, lp * 2.5);

            const o = i * 8;
            textBuf[o] = x; textBuf[o + 1] = y;
            textBuf[o + 2] = size; textBuf[o + 3] = size;
            textBuf[o + 4] = textColors[i * 4];
            textBuf[o + 5] = textColors[i * 4 + 1];
            textBuf[o + 6] = textColors[i * 4 + 2];
            textBuf[o + 7] = alpha;
        }

        engine.clear_point_clouds(gl);
        engine.add_point_cloud(gl, state.bgAnimData);
        engine.add_point_cloud(gl, state.laneData);
        engine.add_point_cloud(gl, state.textBuf);

        if (progress >= 1) {
            state.animDone = true;
            for (let i = 0; i < actualTextCount; i++) {
                state.textPosX[i] = textTargets[i * 2];
                state.textPosY[i] = textTargets[i * 2 + 1];
            }
            state.lastTime = now;
        }
        return;
    }

    // ── Phase 2: Continuous (galaxy rotation + text physics) ──
    const dt = Math.min((now - state.lastTime) / 1000, 0.05);
    state.lastTime = now;

    const { textPosX, textPosY, textVelX, textVelY, textTargets, textColors, actualTextCount, textBuf } = state;

    // Determine if explode is active
    let isExploding = state.exploding;
    let exX = state.explodeX;
    let exY = state.explodeY;

    if (state.mobileExplode) {
        const mobileElapsed = now - state.mobileExplodeStart;
        if (mobileElapsed < 1800) {
            isExploding = true;
            exX = state.textBoundsX + state.textBoundsW / 2;
            exY = state.textBoundsY + state.textBoundsH / 2;
        } else {
            state.mobileExplode = false;
        }
    }

    if (isExploding) state.textAtRest = false;

    if (!state.textAtRest) {
        const SPRING = 18.0;
        const EXPLODE_RADIUS_SQ = 200 * 200;
        const EXPLODE_STRENGTH = 500000;
        let maxVelSq = 0;

        for (let i = 0; i < actualTextCount; i++) {
            const tx = textTargets[i * 2];
            const ty = textTargets[i * 2 + 1];
            let px = textPosX[i];
            let py = textPosY[i];
            let vx = textVelX[i];
            let vy = textVelY[i];

            // Spring toward target
            vx += (tx - px) * SPRING * dt;
            vy += (ty - py) * SPRING * dt;

            // Cursor/tap repulsion
            if (isExploding) {
                const dx = px - exX;
                const dy = py - exY;
                const distSq = dx * dx + dy * dy;
                if (distSq < EXPLODE_RADIUS_SQ) {
                    const force = EXPLODE_STRENGTH / (distSq + 500);
                    vx += dx * force * dt;
                    vy += dy * force * dt;
                }
            }

            // Damping
            vx *= 0.86;
            vy *= 0.86;

            px += vx * dt;
            py += vy * dt;

            textPosX[i] = px;
            textPosY[i] = py;
            textVelX[i] = vx;
            textVelY[i] = vy;

            const velSq = vx * vx + vy * vy;
            if (velSq > maxVelSq) maxVelSq = velSq;

            // Particles grow when scattered to reveal they're dots
            const scatter = Math.min(Math.sqrt((px - tx) * (px - tx) + (py - ty) * (py - ty)) / 60, 1);
            const size = 2.2 + scatter * 3.5;

            const o = i * 8;
            textBuf[o] = px; textBuf[o + 1] = py;
            textBuf[o + 2] = size; textBuf[o + 3] = size;
            textBuf[o + 4] = textColors[i * 4];
            textBuf[o + 5] = textColors[i * 4 + 1];
            textBuf[o + 6] = textColors[i * 4 + 2];
            textBuf[o + 7] = textColors[i * 4 + 3];
        }

        // Check if settled
        if (maxVelSq < 0.5 && !isExploding) {
            state.textAtRest = true;
            for (let i = 0; i < actualTextCount; i++) {
                const o = i * 8;
                textBuf[o] = textTargets[i * 2];
                textBuf[o + 1] = textTargets[i * 2 + 1];
                textBuf[o + 2] = 2.2; textBuf[o + 3] = 2.2;
                textPosX[i] = textTargets[i * 2];
                textPosY[i] = textTargets[i * 2 + 1];
                textVelX[i] = 0;
                textVelY[i] = 0;
            }
        }
    }

    engine.clear_point_clouds(gl);
    engine.add_point_cloud(gl, state.bgAnimData);
    engine.add_point_cloud(gl, state.laneData);
    engine.add_point_cloud(gl, state.textBuf);
}

// ─── Galaxy generator ───
function generateGalaxy(count, textOffX, textOffY, textWorldW, textWorldH) {
    const data = new Float32Array(count * 8);
    let idx = 0;

    const gapL = textOffX - 60, gapR = textOffX + textWorldW + 60;
    const gapT = textOffY - 40, gapB = textOffY + textWorldH + 40;

    function isInTextGap(x, y) {
        return x > gapL && x < gapR && y > gapT && y < gapB;
    }

    const L1 = Math.floor(count * 0.65);
    for (let i = 0; i < L1; i++) {
        const arm = Math.floor(Math.random() * 4);
        const t = Math.random() * 16;
        const spiralAngle = (arm / 4) * Math.PI * 2 + t * 0.55;
        const r = t * 200 + (Math.random() - 0.5) * (80 + t * 40);
        let x = CX + Math.cos(spiralAngle) * r + (Math.random() - 0.5) * 80;
        let y = Math.sin(spiralAngle) * r * 0.55 + (Math.random() - 0.5) * 60;

        x = Math.max(-WORLD_W / 2, Math.min(WORLD_W / 2, x));
        y = Math.max(-WORLD_H / 2, Math.min(WORLD_H / 2, y));

        let textFade = 1;
        if (isInTextGap(x, y)) {
            if (Math.random() < 0.75) continue;
            textFade = 0.2;
        }

        const size = 1.0 + Math.random() * 1.5;
        const dist = Math.sqrt(x * x + (y / 0.55) ** 2);
        const nd = Math.min(dist / 3000, 1);
        let cr, cg, cb;
        if (nd < 0.3) { cr=0.12+nd/0.3*0.12; cg=0.45-nd/0.3*0.15; cb=0.7-nd/0.3*0.05; }
        else if (nd < 0.6) { const f=(nd-0.3)/0.3; cr=0.24+f*0.18; cg=0.3-f*0.10; cb=0.65-f*0.10; }
        else { const f=(nd-0.6)/0.4; cr=0.42-f*0.25; cg=0.2-f*0.08; cb=0.55-f*0.2; }
        const alpha = (0.06 + Math.random() * 0.12) * (1 - nd * 0.5) * textFade;

        if (idx < count) {
            const o = idx * 8;
            data[o]=x; data[o+1]=y; data[o+2]=size; data[o+3]=size;
            data[o+4]=cr; data[o+5]=cg; data[o+6]=cb; data[o+7]=alpha;
        }
        idx++;
    }

    const L2 = Math.floor(count * 0.28);
    for (let i = 0; i < L2 && idx < count; i++) {
        const x = (Math.random() - 0.5) * WORLD_W;
        const y = (Math.random() - 0.5) * WORLD_H;

        let textFade = 1;
        if (isInTextGap(x, y)) {
            if (Math.random() < 0.7) continue;
            textFade = 0.3;
        }

        const size = 1.0 + Math.random() * 2.0;
        const h = Math.random();
        let cr, cg, cb;
        if (h < 0.5) { cr=0.30+Math.random()*0.2; cg=0.40+Math.random()*0.2; cb=0.60+Math.random()*0.2; }
        else { cr=0.50+Math.random()*0.2; cg=0.50+Math.random()*0.12; cb=0.40+Math.random()*0.2; }
        const alpha = (0.03 + Math.random() * 0.08) * textFade;

        const o = idx * 8;
        data[o]=x; data[o+1]=y; data[o+2]=size; data[o+3]=size;
        data[o+4]=cr; data[o+5]=cg; data[o+6]=cb; data[o+7]=alpha;
        idx++;
    }

    const L3 = Math.floor(count * 0.07);
    for (let i = 0; i < L3 && idx < count; i++) {
        const arm = Math.floor(Math.random() * 4);
        const t = Math.random() * 12;
        const spiralAngle = (arm / 4) * Math.PI * 2 + t * 0.55;
        const r = t * 200 + (Math.random() - 0.5) * 300;
        let x = CX + Math.cos(spiralAngle) * r;
        let y = Math.sin(spiralAngle) * r * 0.55;

        x = Math.max(-WORLD_W / 2, Math.min(WORLD_W / 2, x));
        y = Math.max(-WORLD_H / 2, Math.min(WORLD_H / 2, y));

        if (isInTextGap(x, y) && Math.random() < 0.6) continue;

        const size = 2.0 + Math.random() * 3.5;
        const pick = Math.random();
        let cr, cg, cb;
        if (pick < 0.4) { cr=0.25; cg=0.65; cb=0.88; }
        else if (pick < 0.7) { cr=0.55; cg=0.45; cb=0.88; }
        else { cr=0.85; cg=0.38; cb=0.55; }
        const alpha = 0.08 + Math.random() * 0.2;

        const o = idx * 8;
        data[o]=x; data[o+1]=y; data[o+2]=size; data[o+3]=size;
        data[o+4]=cr; data[o+5]=cg; data[o+6]=cb; data[o+7]=alpha;
        idx++;
    }

    return data.subarray(0, idx * 8);
}

// ─── Star-lane connectors ───
function generateStarLanes() {
    const connections = [
        ['home', 'earthquake'], ['home', 'neural'], ['home', 'design'],
        ['home', 'splat'], ['home', 'genome'],
        ['splat', 'code'], ['genome', 'code'],
    ];

    const DOTS_PER_LANE = 300;
    const total = connections.length * DOTS_PER_LANE;
    const data = new Float32Array(total * 8);
    let idx = 0;

    for (const [fromKey, toKey] of connections) {
        const from = ISLANDS[fromKey];
        const to = ISLANDS[toKey];

        for (let i = 0; i < DOTS_PER_LANE; i++) {
            const t = Math.random();
            const x = from.x + (to.x - from.x) * t + (Math.random() - 0.5) * 30;
            const y = from.y + (to.y - from.y) * t + (Math.random() - 0.5) * 30;
            const edgeFade = Math.min(t * 5, (1 - t) * 5, 1);
            const size = 1.0 + Math.random() * 1.2;
            const alpha = (0.04 + Math.random() * 0.08) * edgeFade;

            const o = idx * 8;
            data[o] = x; data[o + 1] = y;
            data[o + 2] = size; data[o + 3] = size;
            data[o + 4] = 0.29; data[o + 5] = 0.75; data[o + 6] = 0.95;
            data[o + 7] = alpha;
            idx++;
        }
    }

    return data.subarray(0, idx * 8);
}
