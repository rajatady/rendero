// ─── Gaussian Splat Viewer ───
// WebGL2 renderer for .splat files
// Based on the projection math from antimatter15/splat

const canvas = document.getElementById('canvas');
const loading = document.getElementById('loading');
const barFill = document.getElementById('bar-fill');
const barText = document.getElementById('bar-text');

const gl = canvas.getContext('webgl2', { antialias: false, premultipliedAlpha: false });
if (!gl) { barText.textContent = 'WebGL2 required.'; throw new Error('No WebGL2'); }

// ─── Scene URLs ───
const SCENES = {
    plush:  'https://huggingface.co/datasets/dylanebert/3dgs/resolve/main/bonsai/point_cloud.splat',
    nike:   'https://media.reshot.ai/models/nike_next/model.splat',
    stump:  'https://huggingface.co/cakewalk/splat-data/resolve/main/stump.splat',
    truck:  'https://huggingface.co/cakewalk/splat-data/resolve/main/truck.splat',
    garden: 'https://huggingface.co/cakewalk/splat-data/resolve/main/garden.splat',
};

// ─── Shaders (following antimatter15 proven math) ───
const VERT_SRC = `#version 300 es
precision highp float;
precision highp int;

uniform highp usampler2D u_texture;
uniform mat4 u_proj;
uniform mat4 u_view;
uniform vec2 u_focal;
uniform vec2 u_viewport;

in vec2 a_quad;
in int a_index;

out vec4 v_color;
out vec2 v_position;

void main() {
    // Fetch center position from texture
    uvec4 cen = texelFetch(u_texture, ivec2((uint(a_index) & 0x3ffu) << 1, uint(a_index) >> 10), 0);
    vec4 cam = u_view * vec4(uintBitsToFloat(cen.xyz), 1);
    vec4 pos2d = u_proj * cam;

    // Frustum cull
    float clip = 1.2 * pos2d.w;
    if (pos2d.z < -clip || pos2d.x < -clip || pos2d.x > clip || pos2d.y < -clip || pos2d.y > clip) {
        gl_Position = vec4(0.0, 0.0, 2.0, 1.0);
        return;
    }

    // Fetch covariance + color from texture
    uvec4 cov = texelFetch(u_texture, ivec2(((uint(a_index) & 0x3ffu) << 1) | 1u, uint(a_index) >> 10), 0);
    vec2 u1 = unpackHalf2x16(cov.x), u2 = unpackHalf2x16(cov.y), u3 = unpackHalf2x16(cov.z);
    mat3 Vrk = mat3(u1.x, u1.y, u2.x, u1.y, u2.y, u3.x, u2.x, u3.x, u3.y);

    // Jacobian of projection
    mat3 J = mat3(
        u_focal.x / cam.z, 0., -(u_focal.x * cam.x) / (cam.z * cam.z),
        0., -u_focal.y / cam.z, (u_focal.y * cam.y) / (cam.z * cam.z),
        0., 0., 0.
    );

    mat3 T = transpose(mat3(u_view)) * J;
    mat3 cov2d = transpose(T) * Vrk * T;

    // Low-pass filter: prevent degenerate / sub-pixel splats (matches 3DGS paper + antimatter15)
    cov2d[0][0] += 0.3;
    cov2d[1][1] += 0.3;

    float mid = (cov2d[0][0] + cov2d[1][1]) / 2.0;
    float radius = length(vec2((cov2d[0][0] - cov2d[1][1]) / 2.0, cov2d[0][1]));
    float lambda1 = mid + radius, lambda2 = mid - radius;

    if (lambda2 < 0.0) return;

    // Eigenvector for major axis — handle near-isotropic case where both components → 0
    vec2 v1 = vec2(cov2d[0][1], lambda1 - cov2d[0][0]);
    float v1len = length(v1);
    vec2 diag = v1len > 1.0e-6 ? v1 / v1len : vec2(1.0, 0.0);
    vec2 majorAxis = min(sqrt(2.0 * lambda1), 1024.0) * diag;
    vec2 minorAxis = min(sqrt(2.0 * lambda2), 1024.0) * vec2(diag.y, -diag.x);

    // Color from packed uint
    v_color = clamp(pos2d.z / pos2d.w + 1.0, 0.0, 1.0) *
        vec4((cov.w) & 0xffu, (cov.w >> 8) & 0xffu, (cov.w >> 16) & 0xffu, (cov.w >> 24) & 0xffu) / 255.0;
    v_position = a_quad;

    vec2 vCenter = vec2(pos2d) / pos2d.w;
    gl_Position = vec4(
        vCenter + a_quad.x * majorAxis / u_viewport + a_quad.y * minorAxis / u_viewport,
        0.0, 1.0
    );
}
`;

const FRAG_SRC = `#version 300 es
precision highp float;

in vec4 v_color;
in vec2 v_position;
out vec4 fragColor;

void main() {
    float A = -dot(v_position, v_position);
    if (A < -4.0) discard;
    float B = exp(A) * v_color.a;
    fragColor = vec4(B * v_color.rgb, B);
}
`;

// ─── Compile ───
function compileShader(src, type) {
    const s = gl.createShader(type);
    gl.shaderSource(s, src);
    gl.compileShader(s);
    if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) {
        console.error(gl.getShaderInfoLog(s));
        throw new Error('Shader fail');
    }
    return s;
}

const program = gl.createProgram();
gl.attachShader(program, compileShader(VERT_SRC, gl.VERTEX_SHADER));
gl.attachShader(program, compileShader(FRAG_SRC, gl.FRAGMENT_SHADER));
gl.linkProgram(program);
if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
    console.error(gl.getProgramInfoLog(program));
    throw new Error('Link fail');
}

const u_proj = gl.getUniformLocation(program, 'u_proj');
const u_view = gl.getUniformLocation(program, 'u_view');
const u_focal = gl.getUniformLocation(program, 'u_focal');
const u_viewport = gl.getUniformLocation(program, 'u_viewport');
const u_texture = gl.getUniformLocation(program, 'u_texture');
const a_quad = gl.getAttribLocation(program, 'a_quad');
const a_index = gl.getAttribLocation(program, 'a_index');

// ─── Sort worker ───
const sortWorker = new Worker('sort-worker.js');
let sortPending = false;
let lastSortTime = 0;

// ─── State ───
let splatCount = 0;
let splatTexture = null;
let vao = null;
let indexBuf = null;
let rawPositions = null; // Float32Array of [x,y,z] for sorting

// Texture dimensions (2 texels per splat, width up to 2048)
const TEX_W = 2048;

sortWorker.onmessage = (e) => {
    const { sortedIndices, timeMs } = e.data;
    lastSortTime = timeMs;
    sortPending = false;
    // Re-upload index buffer with sorted order
    if (indexBuf) {
        // Build sorted index attribute
        const idx = new Int32Array(sortedIndices.length * 4);
        for (let i = 0; i < sortedIndices.length; i++) {
            idx[i * 4] = sortedIndices[i];
            idx[i * 4 + 1] = sortedIndices[i];
            idx[i * 4 + 2] = sortedIndices[i];
            idx[i * 4 + 3] = sortedIndices[i];
        }
        gl.bindBuffer(gl.ARRAY_BUFFER, indexBuf);
        gl.bufferSubData(gl.ARRAY_BUFFER, 0, idx);
    }
};

// ─── Parse .splat and build GPU texture ───
function processSplat(buffer) {
    const count = buffer.byteLength / 32;
    splatCount = count;
    barText.textContent = `Processing ${count.toLocaleString()} gaussians...`;

    const f32 = new Float32Array(buffer);
    const u8 = new Uint8Array(buffer);

    // Store positions for sorting
    rawPositions = new Float32Array(count * 3);
    for (let i = 0; i < count; i++) {
        rawPositions[i * 3] = f32[i * 8];
        rawPositions[i * 3 + 1] = f32[i * 8 + 1];
        rawPositions[i * 3 + 2] = f32[i * 8 + 2];
    }

    // Build texture data: 2 texels per splat
    // Texel 0: position (xyz as float bits in uint) + w unused
    // Texel 1: covariance (6 floats packed as 3 half-float pairs) + packed RGBA color
    const texH = Math.ceil(count / (TEX_W / 2));  // /2 because 2 texels per splat
    const texData = new Uint32Array(TEX_W * texH * 4); // RGBA32UI

    for (let i = 0; i < count; i++) {
        const bOff = i * 32;
        const fOff = i * 8;

        // Position
        const px = f32[fOff], py = f32[fOff + 1], pz = f32[fOff + 2];

        // Scale (stored as floats in the .splat)
        const sx = f32[fOff + 3], sy = f32[fOff + 4], sz = f32[fOff + 5];

        // Rotation quaternion from uint8: byte order is (w, x, y, z)
        let rw = (u8[bOff + 28] - 128) / 128;
        let rx = (u8[bOff + 29] - 128) / 128;
        let ry = (u8[bOff + 30] - 128) / 128;
        let rz = (u8[bOff + 31] - 128) / 128;
        // Normalize quaternion (quantization denormalizes it)
        const qlen = Math.sqrt(rw * rw + rx * rx + ry * ry + rz * rz) || 1;
        const rot = [rw / qlen, rx / qlen, ry / qlen, rz / qlen];

        // Rotation-scale matrix M (antimatter15 convention: scale per row)
        const M = [
            1.0 - 2.0 * (rot[2] * rot[2] + rot[3] * rot[3]),
            2.0 * (rot[1] * rot[2] + rot[0] * rot[3]),
            2.0 * (rot[1] * rot[3] - rot[0] * rot[2]),

            2.0 * (rot[1] * rot[2] - rot[0] * rot[3]),
            1.0 - 2.0 * (rot[1] * rot[1] + rot[3] * rot[3]),
            2.0 * (rot[2] * rot[3] + rot[0] * rot[1]),

            2.0 * (rot[1] * rot[3] + rot[0] * rot[2]),
            2.0 * (rot[2] * rot[3] - rot[0] * rot[1]),
            1.0 - 2.0 * (rot[1] * rot[1] + rot[2] * rot[2]),
        ].map((k, i) => k * [sx, sy, sz][Math.floor(i / 3)]);

        // Covariance Sigma = M * M^T (symmetric 3x3, 6 unique values)
        const sigma = [
            M[0] * M[0] + M[3] * M[3] + M[6] * M[6],
            M[0] * M[1] + M[3] * M[4] + M[6] * M[7],
            M[0] * M[2] + M[3] * M[5] + M[6] * M[8],
            M[1] * M[1] + M[4] * M[4] + M[7] * M[7],
            M[1] * M[2] + M[4] * M[5] + M[7] * M[8],
            M[2] * M[2] + M[5] * M[5] + M[8] * M[8],
        ];

        // Color (RGBA packed into one uint32)
        const r = u8[bOff + 24], g = u8[bOff + 25], b = u8[bOff + 26], a = u8[bOff + 27];
        const packedColor = r | (g << 8) | (b << 16) | (a << 24);

        // Pack sigma as half-floats
        const sigHalf = sigma.map(floatToHalf);

        // Texel row = i >> 10, col = (i & 0x3ff) << 1
        const row = i >> 10;
        const col = (i & 0x3FF) << 1;

        // Texel 0: position (as uint bits)
        const t0 = (row * TEX_W + col) * 4;
        const pxU = new Float32Array([px]);
        const pyU = new Float32Array([py]);
        const pzU = new Float32Array([pz]);
        texData[t0] = new Uint32Array(pxU.buffer)[0];
        texData[t0 + 1] = new Uint32Array(pyU.buffer)[0];
        texData[t0 + 2] = new Uint32Array(pzU.buffer)[0];
        texData[t0 + 3] = 0;

        // Texel 1: covariance (half-float pairs) + packed color
        const t1 = (row * TEX_W + col + 1) * 4;
        texData[t1] = sigHalf[0] | (sigHalf[1] << 16);
        texData[t1 + 1] = sigHalf[2] | (sigHalf[3] << 16);
        texData[t1 + 2] = sigHalf[4] | (sigHalf[5] << 16);
        texData[t1 + 3] = packedColor;
    }

    // Upload texture
    if (splatTexture) gl.deleteTexture(splatTexture);
    splatTexture = gl.createTexture();
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, splatTexture);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA32UI, TEX_W, texH, 0, gl.RGBA_INTEGER, gl.UNSIGNED_INT, texData);

    // Build VAO with quad + per-vertex index
    if (vao) gl.deleteVertexArray(vao);
    vao = gl.createVertexArray();
    gl.bindVertexArray(vao);

    // Quad positions (4 verts per splat, triangle strip)
    // But we need indexed rendering for sorted order
    // Actually: 6 verts per splat (2 triangles), each carrying the splat index
    const quadData = new Float32Array(count * 4 * 2); // 4 verts * 2 floats per splat
    const indexData = new Int32Array(count * 4);
    for (let i = 0; i < count; i++) {
        const off = i * 8;
        quadData[off] = -2; quadData[off + 1] = -2;
        quadData[off + 2] = 2; quadData[off + 3] = -2;
        quadData[off + 4] = -2; quadData[off + 5] = 2;
        quadData[off + 6] = 2; quadData[off + 7] = 2;
        indexData[i * 4] = i;
        indexData[i * 4 + 1] = i;
        indexData[i * 4 + 2] = i;
        indexData[i * 4 + 3] = i;
    }

    const quadBuf = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, quadBuf);
    gl.bufferData(gl.ARRAY_BUFFER, quadData, gl.STATIC_DRAW);
    gl.enableVertexAttribArray(a_quad);
    gl.vertexAttribPointer(a_quad, 2, gl.FLOAT, false, 0, 0);

    indexBuf = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, indexBuf);
    gl.bufferData(gl.ARRAY_BUFFER, indexData, gl.DYNAMIC_DRAW);
    gl.enableVertexAttribArray(a_index);
    gl.vertexAttribIPointer(a_index, 1, gl.INT, 0, 0);

    // Element buffer for triangle strip → triangles
    const eleBuf = gl.createBuffer();
    const elements = new Uint32Array(count * 6);
    for (let i = 0; i < count; i++) {
        const v = i * 4;
        const e = i * 6;
        elements[e] = v; elements[e + 1] = v + 1; elements[e + 2] = v + 2;
        elements[e + 3] = v + 1; elements[e + 4] = v + 3; elements[e + 5] = v + 2;
    }
    gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, eleBuf);
    gl.bufferData(gl.ELEMENT_ARRAY_BUFFER, elements, gl.STATIC_DRAW);

    gl.bindVertexArray(null);

    return count;
}

// ─── Float to half-float ───
const floatView = new Float32Array(1);
const int32View = new Int32Array(floatView.buffer);
function floatToHalf(val) {
    floatView[0] = val;
    const x = int32View[0];
    let bits = (x >> 16) & 0x8000;
    let m = (x >> 12) & 0x07ff;
    let e = (x >> 23) & 0xff;
    if (e < 103) return bits;
    if (e > 142) {
        bits |= 0x7c00;
        bits |= ((e === 255) ? 0 : 1) && (x & 0x007fffff);
        return bits;
    }
    if (e < 113) {
        m |= 0x0800;
        bits |= (m >> (114 - e)) + ((m >> (113 - e)) & 1);
        return bits;
    }
    bits |= ((e - 112) << 10) | (m >> 1);
    bits += m & 1;
    return bits;
}

// ─── 3D Camera ───
let camTheta = 0.5;
let camPhi = 0.4;
let camRadius = 4.0;
let camTarget = [0, 0, 0];
let camFovY = 50 * Math.PI / 180;

function getCameraPos() {
    return [
        camTarget[0] + camRadius * Math.cos(camPhi) * Math.sin(camTheta),
        camTarget[1] + camRadius * Math.sin(camPhi),
        camTarget[2] + camRadius * Math.cos(camPhi) * Math.cos(camTheta),
    ];
}

// ─── Matrix math ───
function mat4Perspective(fovY, aspect, near, far) {
    const f = 1 / Math.tan(fovY / 2);
    const nf = 1 / (near - far);
    return new Float32Array([
        f / aspect, 0, 0, 0,
        0, f, 0, 0,
        0, 0, (far + near) * nf, -1,
        0, 0, 2 * far * near * nf, 0,
    ]);
}

function mat4LookAt(eye, center, up) {
    const zx = eye[0] - center[0], zy = eye[1] - center[1], zz = eye[2] - center[2];
    let len = 1 / (Math.sqrt(zx * zx + zy * zy + zz * zz) || 1);
    const fz = [zx * len, zy * len, zz * len];
    const sx = up[1] * fz[2] - up[2] * fz[1];
    const sy = up[2] * fz[0] - up[0] * fz[2];
    const sz = up[0] * fz[1] - up[1] * fz[0];
    len = 1 / (Math.sqrt(sx * sx + sy * sy + sz * sz) || 1);
    const fs = [sx * len, sy * len, sz * len];
    const ux = fz[1] * fs[2] - fz[2] * fs[1];
    const uy = fz[2] * fs[0] - fz[0] * fs[2];
    const uz = fz[0] * fs[1] - fz[1] * fs[0];
    return new Float32Array([
        fs[0], ux, fz[0], 0,
        fs[1], uy, fz[1], 0,
        fs[2], uz, fz[2], 0,
        -(fs[0] * eye[0] + fs[1] * eye[1] + fs[2] * eye[2]),
        -(ux * eye[0] + uy * eye[1] + uz * eye[2]),
        -(fz[0] * eye[0] + fz[1] * eye[1] + fz[2] * eye[2]),
        1,
    ]);
}

function mat4Multiply(a, b) {
    const out = new Float32Array(16);
    for (let i = 0; i < 4; i++) {
        for (let j = 0; j < 4; j++) {
            out[j * 4 + i] = a[i] * b[j * 4] + a[4 + i] * b[j * 4 + 1] +
                              a[8 + i] * b[j * 4 + 2] + a[12 + i] * b[j * 4 + 3];
        }
    }
    return out;
}

// ─── Resize ───
function resize() {
    canvas.width = window.innerWidth;
    canvas.height = window.innerHeight;
    gl.viewport(0, 0, canvas.width, canvas.height);
}
resize();
window.addEventListener('resize', resize);

// ─── Mouse controls ───
let isDragging = false, isShiftDrag = false;
let lastMX = 0, lastMY = 0;

canvas.addEventListener('mousedown', (e) => {
    isDragging = true;
    isShiftDrag = e.shiftKey;
    lastMX = e.clientX; lastMY = e.clientY;
});
window.addEventListener('mousemove', (e) => {
    if (!isDragging) return;
    const dx = e.clientX - lastMX, dy = e.clientY - lastMY;
    lastMX = e.clientX; lastMY = e.clientY;
    if (isShiftDrag) {
        const panSpeed = camRadius * 0.002;
        const right = [Math.cos(camTheta), 0, -Math.sin(camTheta)];
        camTarget[0] -= dx * panSpeed * right[0];
        camTarget[2] -= dx * panSpeed * right[2];
        camTarget[1] += dy * panSpeed;
    } else {
        camTheta -= dx * 0.005;
        camPhi += dy * 0.005;
        camPhi = Math.max(-1.5, Math.min(1.5, camPhi));
    }
});
window.addEventListener('mouseup', () => { isDragging = false; });
canvas.addEventListener('wheel', (e) => {
    e.preventDefault();
    camRadius *= e.deltaY > 0 ? 1.1 : 0.9;
    camRadius = Math.max(0.1, Math.min(100, camRadius));
}, { passive: false });

// ─── Keyboard ───
document.addEventListener('keydown', (e) => {
    if (e.key === 'r' || e.key === 'R') {
        camTheta = 0.5; camPhi = 0.4; camRadius = 4.0; camTarget = [0, 0, 0];
    }
    if (e.key >= '1' && e.key <= '5') {
        const scenes = Object.keys(SCENES);
        const idx = parseInt(e.key) - 1;
        if (idx < scenes.length) {
            document.getElementById('scene-select').value = scenes[idx];
            loadScene(scenes[idx]);
        }
    }
});

document.getElementById('scene-select').addEventListener('change', (e) => {
    loadScene(e.target.value);
});

// ─── Request sort ───
function requestSort() {
    if (sortPending || !rawPositions) return;
    sortPending = true;
    const eye = getCameraPos();
    const view = mat4LookAt(eye, camTarget, [0, 1, 0]);
    const aspect = canvas.width / canvas.height;
    const proj = mat4Perspective(camFovY, aspect, 0.1, 200);
    const viewProj = mat4Multiply(proj, view);
    sortWorker.postMessage({
        positions: rawPositions,
        viewProj,
        count: splatCount,
    });
}

// ─── Load scene ───
async function loadScene(name) {
    const url = SCENES[name];
    if (!url) return;

    loading.style.display = 'flex';
    barFill.style.width = '0%';
    barText.textContent = `Fetching ${name}...`;
    splatCount = 0;

    try {
        const resp = await fetch(url);
        if (!resp.ok) throw new Error(`HTTP ${resp.status}`);

        const contentLength = parseInt(resp.headers.get('Content-Length') || '0');
        const reader = resp.body.getReader();
        const chunks = [];
        let received = 0;

        while (true) {
            const { done, value } = await reader.read();
            if (done) break;
            chunks.push(value);
            received += value.length;
            if (contentLength > 0) {
                const pct = (received / contentLength * 100).toFixed(1);
                barFill.style.width = pct + '%';
                barText.textContent = `${(received / 1e6).toFixed(1)} / ${(contentLength / 1e6).toFixed(1)} MB`;
            } else {
                barText.textContent = `${(received / 1e6).toFixed(1)} MB...`;
            }
        }

        const buffer = new ArrayBuffer(received);
        const u8 = new Uint8Array(buffer);
        let offset = 0;
        for (const chunk of chunks) { u8.set(chunk, offset); offset += chunk.length; }

        barText.textContent = 'Processing splats...';
        await new Promise(r => setTimeout(r, 0));

        const count = processSplat(buffer);

        document.getElementById('s-count').textContent = count.toLocaleString();
        document.getElementById('i-file').textContent = name + '.splat';
        document.getElementById('i-size').textContent = (received / 1e6).toFixed(1) + ' MB';
        document.getElementById('i-gaussians').textContent = count.toLocaleString();

        // Auto-center
        let cx = 0, cy = 0, cz = 0;
        const n = Math.min(count, 50000);
        for (let i = 0; i < n; i++) {
            cx += rawPositions[i * 3];
            cy += rawPositions[i * 3 + 1];
            cz += rawPositions[i * 3 + 2];
        }
        camTarget = [cx / n, cy / n, cz / n];
        camRadius = 4.0; camTheta = 0.5; camPhi = 0.4;

        requestSort();
        loading.style.display = 'none';
    } catch (err) {
        barText.textContent = `Error: ${err.message}`;
        console.error(err);
    }
}

// ─── Render ───
let lastTime = performance.now();
let frames = 0, fps = 0, lastSortReq = 0;

function render() {
    const now = performance.now();
    if (now - lastSortReq > 50) { requestSort(); lastSortReq = now; }

    const w = canvas.width, h = canvas.height;
    gl.viewport(0, 0, w, h);
    gl.clearColor(0.02, 0.02, 0.028, 1.0);
    gl.clear(gl.COLOR_BUFFER_BIT);

    if (splatCount > 0 && vao && splatTexture) {
        const eye = getCameraPos();
        const view = mat4LookAt(eye, camTarget, [0, 1, 0]);
        const aspect = w / h;
        const proj = mat4Perspective(camFovY, aspect, 0.1, 200);
        const focalY = (h / 2) / Math.tan(camFovY / 2);
        const focalX = focalY;

        gl.useProgram(program);
        gl.uniformMatrix4fv(u_proj, false, proj);
        gl.uniformMatrix4fv(u_view, false, view);
        gl.uniform2f(u_focal, focalX, focalY);
        gl.uniform2f(u_viewport, w, h);

        gl.activeTexture(gl.TEXTURE0);
        gl.bindTexture(gl.TEXTURE_2D, splatTexture);
        gl.uniform1i(u_texture, 0);

        gl.enable(gl.BLEND);
        gl.blendFuncSeparate(gl.ONE, gl.ONE_MINUS_SRC_ALPHA, gl.ONE, gl.ONE_MINUS_SRC_ALPHA);
        gl.disable(gl.DEPTH_TEST);
        gl.depthMask(false);

        gl.bindVertexArray(vao);
        gl.drawElements(gl.TRIANGLES, splatCount * 6, gl.UNSIGNED_INT, 0);
        gl.bindVertexArray(null);

        gl.disable(gl.BLEND);

        document.getElementById('i-camera').textContent =
            `θ=${(camTheta * 180 / Math.PI).toFixed(0)}° φ=${(camPhi * 180 / Math.PI).toFixed(0)}° r=${camRadius.toFixed(1)}`;
    }

    frames++;
    if (now - lastTime > 500) {
        fps = Math.round(frames / (now - lastTime) * 1000);
        frames = 0; lastTime = now;
        document.getElementById('s-fps').textContent = fps;
        document.getElementById('s-sort').textContent = lastSortTime.toFixed(1);
    }

    requestAnimationFrame(render);
}

requestAnimationFrame(render);
loadScene('truck');
