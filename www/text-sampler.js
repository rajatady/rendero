/**
 * text-sampler.js — Sample text glyphs into particle positions
 *
 * Renders text on an offscreen canvas and extracts pixel positions
 * where the glyph is drawn. Used for the hero "rendero." particle effect.
 */

/**
 * @param {string} text
 * @param {number} fontSize
 * @param {string} fontFamily
 * @param {number} maxPoints
 * @returns {{ points: {x:number,y:number}[], canvasW: number, canvasH: number }}
 */
export function sampleText(text, fontSize = 300, fontFamily = "'Arial Black', 'Impact', sans-serif", maxPoints = 120000) {
    const offscreen = document.createElement('canvas');
    const ctx = offscreen.getContext('2d');

    const fontStr = `900 ${fontSize}px ${fontFamily}`;
    ctx.font = fontStr;
    const metrics = ctx.measureText(text);
    const textW = Math.ceil(metrics.width);
    const textH = Math.ceil(fontSize * 1.2);

    offscreen.width = textW + 60;
    offscreen.height = textH + 60;

    ctx.fillStyle = '#000';
    ctx.fillRect(0, 0, offscreen.width, offscreen.height);
    ctx.font = fontStr;
    ctx.fillStyle = '#fff';
    ctx.textBaseline = 'top';
    ctx.fillText(text, 30, 30);

    const imgData = ctx.getImageData(0, 0, offscreen.width, offscreen.height);
    const pixels = imgData.data;

    const candidates = [];
    for (let y = 0; y < offscreen.height; y++) {
        for (let x = 0; x < offscreen.width; x++) {
            const i = (y * offscreen.width + x) * 4;
            if (pixels[i] > 100) {
                candidates.push({ x, y });
            }
        }
    }

    // Randomly subsample to maxPoints
    const points = [];
    const count = Math.min(maxPoints, candidates.length);

    if (candidates.length <= maxPoints) {
        for (const c of candidates) points.push(c);
    } else {
        const arr = candidates.slice();
        for (let i = 0; i < count; i++) {
            const j = i + Math.floor(Math.random() * (arr.length - i));
            [arr[i], arr[j]] = [arr[j], arr[i]];
            points.push(arr[i]);
        }
    }

    return {
        points,
        canvasW: offscreen.width,
        canvasH: offscreen.height,
    };
}
