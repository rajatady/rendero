// ═══════════════════════════════════════════════════════════════
// CSS Value Parsing — colors, units, shorthand expansion
// ═══════════════════════════════════════════════════════════════

const NAMED_COLORS = {
    transparent: [0, 0, 0, 0],
    black: [0, 0, 0, 1],
    white: [1, 1, 1, 1],
    red: [1, 0, 0, 1],
    green: [0, 0.502, 0, 1],
    blue: [0, 0, 1, 1],
    gray: [0.502, 0.502, 0.502, 1],
    grey: [0.502, 0.502, 0.502, 1],
    silver: [0.753, 0.753, 0.753, 1],
    orange: [1, 0.647, 0, 1],
    yellow: [1, 1, 0, 1],
    purple: [0.502, 0, 0.502, 1],
    pink: [1, 0.753, 0.796, 1],
    cyan: [0, 1, 1, 1],
    magenta: [1, 0, 1, 1],
    navy: [0, 0, 0.502, 1],
    teal: [0, 0.502, 0.502, 1],
    maroon: [0.502, 0, 0, 1],
    lime: [0, 1, 0, 1],
    olive: [0.502, 0.502, 0, 1],
    aqua: [0, 1, 1, 1],
    coral: [1, 0.498, 0.314, 1],
    tomato: [1, 0.388, 0.278, 1],
    salmon: [0.980, 0.502, 0.447, 1],
    gold: [1, 0.843, 0, 1],
    wheat: [0.961, 0.871, 0.702, 1],
    ivory: [1, 1, 0.941, 1],
    snow: [1, 0.980, 0.980, 1],
    linen: [0.980, 0.941, 0.902, 1],
    mintcream: [0.961, 1, 0.980, 1],
    aliceblue: [0.941, 0.973, 1, 1],
    ghostwhite: [0.973, 0.973, 1, 1],
    whitesmoke: [0.961, 0.961, 0.961, 1],
    lavender: [0.902, 0.902, 0.980, 1],
    honeydew: [0.941, 1, 0.941, 1],
    seashell: [1, 0.961, 0.933, 1],
    cornsilk: [1, 0.973, 0.863, 1],
    beige: [0.961, 0.961, 0.863, 1],
    oldlace: [0.992, 0.961, 0.902, 1],
    floralwhite: [1, 0.980, 0.941, 1],
    darkgray: [0.663, 0.663, 0.663, 1],
    darkgrey: [0.663, 0.663, 0.663, 1],
    lightgray: [0.827, 0.827, 0.827, 1],
    lightgrey: [0.827, 0.827, 0.827, 1],
    dimgray: [0.412, 0.412, 0.412, 1],
    dimgrey: [0.412, 0.412, 0.412, 1],
    slategray: [0.439, 0.502, 0.565, 1],
    darkslategray: [0.184, 0.310, 0.310, 1],
    gainsboro: [0.863, 0.863, 0.863, 1],
    steelblue: [0.275, 0.510, 0.706, 1],
    royalblue: [0.255, 0.412, 0.882, 1],
    dodgerblue: [0.118, 0.565, 1, 1],
    cornflowerblue: [0.392, 0.584, 0.929, 1],
    deepskyblue: [0, 0.749, 1, 1],
    skyblue: [0.529, 0.808, 0.922, 1],
    lightskyblue: [0.529, 0.808, 0.980, 1],
    lightblue: [0.678, 0.847, 0.902, 1],
    midnightblue: [0.098, 0.098, 0.439, 1],
    darkblue: [0, 0, 0.545, 1],
    mediumblue: [0, 0, 0.804, 1],
    indianred: [0.804, 0.361, 0.361, 1],
    crimson: [0.863, 0.078, 0.235, 1],
    firebrick: [0.698, 0.133, 0.133, 1],
    darkred: [0.545, 0, 0, 1],
    darkgreen: [0, 0.392, 0, 1],
    forestgreen: [0.133, 0.545, 0.133, 1],
    seagreen: [0.180, 0.545, 0.341, 1],
    limegreen: [0.196, 0.804, 0.196, 1],
    mediumseagreen: [0.235, 0.702, 0.443, 1],
    springgreen: [0, 1, 0.498, 1],
    darkviolet: [0.580, 0, 0.827, 1],
    darkorchid: [0.600, 0.196, 0.800, 1],
    indigo: [0.294, 0, 0.510, 1],
    mediumpurple: [0.576, 0.439, 0.859, 1],
    orchid: [0.855, 0.439, 0.839, 1],
    plum: [0.867, 0.627, 0.867, 1],
    violet: [0.933, 0.510, 0.933, 1],
    hotpink: [1, 0.412, 0.706, 1],
    deeppink: [1, 0.078, 0.576, 1],
    chocolate: [0.824, 0.412, 0.118, 1],
    sienna: [0.627, 0.322, 0.176, 1],
    saddlebrown: [0.545, 0.271, 0.075, 1],
    peru: [0.804, 0.522, 0.247, 1],
    sandybrown: [0.957, 0.643, 0.376, 1],
    burlywood: [0.871, 0.722, 0.529, 1],
    tan: [0.824, 0.706, 0.549, 1],
    rosybrown: [0.737, 0.561, 0.561, 1],
};

// Parse any CSS color to [r, g, b, a] (0-1 range)
export function parseColor(str) {
    if (!str || str === 'none' || str === 'inherit' || str === 'initial' || str === 'unset') {
        return null;
    }
    str = str.trim().toLowerCase();

    if (NAMED_COLORS[str]) return [...NAMED_COLORS[str]];

    // #hex
    if (str.startsWith('#')) {
        const hex = str.slice(1);
        if (hex.length === 3) {
            return [
                parseInt(hex[0] + hex[0], 16) / 255,
                parseInt(hex[1] + hex[1], 16) / 255,
                parseInt(hex[2] + hex[2], 16) / 255,
                1
            ];
        }
        if (hex.length === 6) {
            return [
                parseInt(hex.slice(0, 2), 16) / 255,
                parseInt(hex.slice(2, 4), 16) / 255,
                parseInt(hex.slice(4, 6), 16) / 255,
                1
            ];
        }
        if (hex.length === 8) {
            return [
                parseInt(hex.slice(0, 2), 16) / 255,
                parseInt(hex.slice(2, 4), 16) / 255,
                parseInt(hex.slice(4, 6), 16) / 255,
                parseInt(hex.slice(6, 8), 16) / 255,
            ];
        }
    }

    // rgb(r, g, b) or rgba(r, g, b, a)
    const rgbMatch = str.match(/rgba?\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*(?:,\s*([\d.]+))?\s*\)/);
    if (rgbMatch) {
        return [
            parseInt(rgbMatch[1]) / 255,
            parseInt(rgbMatch[2]) / 255,
            parseInt(rgbMatch[3]) / 255,
            rgbMatch[4] !== undefined ? parseFloat(rgbMatch[4]) : 1,
        ];
    }

    // rgb(r g b / a) modern syntax
    const rgbModern = str.match(/rgba?\(\s*(\d+)\s+(\d+)\s+(\d+)\s*(?:\/\s*([\d.]+%?))?\s*\)/);
    if (rgbModern) {
        let a = 1;
        if (rgbModern[4]) {
            a = rgbModern[4].endsWith('%') ? parseFloat(rgbModern[4]) / 100 : parseFloat(rgbModern[4]);
        }
        return [
            parseInt(rgbModern[1]) / 255,
            parseInt(rgbModern[2]) / 255,
            parseInt(rgbModern[3]) / 255,
            a,
        ];
    }

    return [0, 0, 0, 1]; // fallback black
}

// Viewport dimensions — set by the platform (browser or native)
let _viewportW = 1280;
let _viewportH = 800;

export function setViewport(w, h) {
    _viewportW = w;
    _viewportH = h;
}

// Parse a CSS length value to pixels
export function parseLength(value, base) {
    if (typeof value === 'number') return value;
    if (!value || value === 'auto' || value === 'none') return 0;
    const str = String(value).trim();
    if (str.endsWith('px')) return parseFloat(str);
    if (str.endsWith('rem')) return parseFloat(str) * 16;
    if (str.endsWith('em')) return parseFloat(str) * (base || 16);
    if (str.endsWith('vh')) return parseFloat(str) * _viewportH / 100;
    if (str.endsWith('vw')) return parseFloat(str) * _viewportW / 100;
    if (str.endsWith('%')) {
        // % resolves against parent. If no base given, use viewport width as default.
        const pct = parseFloat(str) / 100;
        return pct * (base || _viewportW);
    }
    const n = parseFloat(str);
    return isNaN(n) ? 0 : n;
}

export function parsePercentage(value) {
    if (typeof value !== 'string') return 0;
    const str = value.trim();
    if (!str.endsWith('%')) return 0;
    const pct = parseFloat(str);
    return Number.isFinite(pct) ? pct / 100 : 0;
}

export function parseLineHeight(value, fontSize) {
    if (value == null || value === '' || value === 'normal') return 0;
    if (typeof value === 'number') {
        return value <= 4 ? value * (fontSize || 16) : value;
    }
    const str = String(value).trim();
    if (/^[\d.]+$/.test(str)) {
        const unitless = parseFloat(str);
        return isNaN(unitless) ? 0 : unitless * (fontSize || 16);
    }
    return parseLength(str, fontSize);
}

// Parse font-weight string to numeric
export function parseFontWeight(value) {
    if (typeof value === 'number') return value;
    const map = {
        thin: 100, hairline: 100,
        extralight: 200, ultralight: 200,
        light: 300,
        normal: 400, regular: 400,
        medium: 500,
        semibold: 600, demibold: 600,
        bold: 700,
        extrabold: 800, ultrabold: 800,
        black: 900, heavy: 900,
    };
    const s = String(value).toLowerCase().replace(/[- ]/g, '');
    return map[s] || parseInt(value) || 400;
}

// Parse box-shadow: "offsetX offsetY blur spread color"
export function parseBoxShadow(value) {
    if (!value || value === 'none') return null;
    // Simplified: "2px 4px 8px rgba(0,0,0,0.2)"
    const parts = value.match(/([-\d.]+)px\s+([-\d.]+)px\s+([-\d.]+)px\s*(?:([-\d.]+)px\s*)?(.*)/);
    if (!parts) return null;
    const color = parseColor(parts[5] || 'rgba(0,0,0,0.3)');
    return {
        ox: parseFloat(parts[1]),
        oy: parseFloat(parts[2]),
        blur: parseFloat(parts[3]),
        spread: parts[4] ? parseFloat(parts[4]) : 0,
        r: color[0], g: color[1], b: color[2], a: color[3],
    };
}

// Parse linear-gradient CSS to engine format
export function parseLinearGradient(value) {
    if (!value || !value.startsWith('linear-gradient')) return null;
    // Simple: linear-gradient(to right, #000, #fff)
    // or: linear-gradient(180deg, #000 0%, #fff 100%)
    const inner = value.match(/linear-gradient\((.+)\)/);
    if (!inner) return null;

    const parts = inner[1].split(',').map(s => s.trim());
    let angle = 180; // default: top to bottom
    let colorStart = 0;

    // Check if first part is direction/angle
    const first = parts[0];
    if (first.endsWith('deg')) {
        angle = parseFloat(first);
        colorStart = 1;
    } else if (first.startsWith('to ')) {
        const dir = first.slice(3);
        const angles = { bottom: 180, top: 0, right: 90, left: 270 };
        angle = angles[dir] || 180;
        colorStart = 1;
    }

    const stops = [];
    for (let i = colorStart; i < parts.length; i++) {
        const match = parts[i].match(/(.+?)(?:\s+([\d.]+%?))?$/);
        if (match) {
            const color = parseColor(match[1].trim());
            const pos = match[2]
                ? (match[2].endsWith('%') ? parseFloat(match[2]) / 100 : parseFloat(match[2]))
                : i / (parts.length - 1);
            stops.push({ color, pos });
        }
    }

    // Convert angle to start/end coordinates (0-1 range)
    const rad = (angle - 90) * Math.PI / 180;
    const startX = 0.5 - Math.cos(rad) * 0.5;
    const startY = 0.5 - Math.sin(rad) * 0.5;
    const endX = 0.5 + Math.cos(rad) * 0.5;
    const endY = 0.5 + Math.sin(rad) * 0.5;

    const positions = new Float32Array(stops.map(s => s.pos));
    const colors = new Float32Array(stops.flatMap(s => s.color));

    return { startX, startY, endX, endY, positions, colors };
}

// Expand shorthand CSS properties into individual properties
export function expandShorthand(prop, value) {
    const str = String(value).trim();

    switch (prop) {
        case 'margin': return expandBox('margin', str);
        case 'padding': return expandBox('padding', str);
        case 'borderRadius': return expandCorners(str);
        case 'border-radius': return expandCorners(str);
        case 'gap': {
            const parts = str.split(/\s+/);
            return {
                rowGap: parts[0],
                columnGap: parts[1] || parts[0],
            };
        }
        case 'flex': {
            // CSS spec: flex: <number> → grow=N, shrink=1, basis=0%
            // flex: none → 0 0 auto. flex: auto → 1 1 auto.
            if (str === 'none') return { flexGrow: '0', flexShrink: '0', flexBasis: 'auto' };
            if (str === 'auto') return { flexGrow: '1', flexShrink: '1', flexBasis: 'auto' };
            const parts = str.split(/\s+/);
            if (parts.length === 1) {
                return { flexGrow: parts[0], flexShrink: '1', flexBasis: '0%' };
            }
            if (parts.length === 2) {
                return { flexGrow: parts[0], flexShrink: parts[1], flexBasis: '0%' };
            }
            return { flexGrow: parts[0], flexShrink: parts[1], flexBasis: parts[2] };
        }
        case 'background': {
            // Simple: treat as backgroundColor unless it's a gradient
            if (str.startsWith('linear-gradient') || str.startsWith('radial-gradient')) {
                return { backgroundImage: str };
            }
            return { backgroundColor: str };
        }
    }
    return null;
}

function expandBox(prefix, str) {
    const parts = str.split(/\s+/).map(s => s.trim());
    if (parts.length === 1) {
        return {
            [`${prefix}Top`]: parts[0],
            [`${prefix}Right`]: parts[0],
            [`${prefix}Bottom`]: parts[0],
            [`${prefix}Left`]: parts[0],
        };
    }
    if (parts.length === 2) {
        return {
            [`${prefix}Top`]: parts[0],
            [`${prefix}Right`]: parts[1],
            [`${prefix}Bottom`]: parts[0],
            [`${prefix}Left`]: parts[1],
        };
    }
    if (parts.length === 3) {
        return {
            [`${prefix}Top`]: parts[0],
            [`${prefix}Right`]: parts[1],
            [`${prefix}Bottom`]: parts[2],
            [`${prefix}Left`]: parts[1],
        };
    }
    return {
        [`${prefix}Top`]: parts[0],
        [`${prefix}Right`]: parts[1],
        [`${prefix}Bottom`]: parts[2],
        [`${prefix}Left`]: parts[3],
    };
}

function expandCorners(str) {
    const parts = str.split(/\s+/).map(s => parseLength(s));
    if (parts.length === 1) return { _tl: parts[0], _tr: parts[0], _br: parts[0], _bl: parts[0] };
    if (parts.length === 2) return { _tl: parts[0], _tr: parts[1], _br: parts[0], _bl: parts[1] };
    if (parts.length === 3) return { _tl: parts[0], _tr: parts[1], _br: parts[2], _bl: parts[1] };
    return { _tl: parts[0], _tr: parts[1], _br: parts[2], _bl: parts[3] };
}
