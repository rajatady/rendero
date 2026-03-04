// ─── Gaussian Splat Viewer ───
// WebGL2 renderer for .splat files
// Rendering pipeline from antimatter15/splat (instanced, front-to-back)

// Register service worker to cache .splat files
if ('serviceWorker' in navigator) {
    navigator.serviceWorker.register('sw.js').catch(() => {});
}

const canvas = document.getElementById('canvas');
const loading = document.getElementById('loading');
const barFill = document.getElementById('bar-fill');
const barText = document.getElementById('bar-text');

const gl = canvas.getContext('webgl2', { antialias: false });
if (!gl) { barText.textContent = 'WebGL2 required.'; throw new Error('No WebGL2'); }

// ─── Scene URLs ───
const SCENES = {
    plush:  'https://huggingface.co/cakewalk/splat-data/resolve/main/plush.splat',
    nike:   'https://huggingface.co/cakewalk/splat-data/resolve/main/nike.splat',
    stump:  'https://huggingface.co/cakewalk/splat-data/resolve/main/stump.splat',
    truck:  'https://huggingface.co/cakewalk/splat-data/resolve/main/truck.splat',
    garden: 'https://huggingface.co/cakewalk/splat-data/resolve/main/garden.splat',
};

// ─── Shaders (identical to antimatter15/splat) ───
const VERT_SRC = `#version 300 es
precision highp float;
precision highp int;

uniform highp usampler2D u_texture;
uniform mat4 projection, view;
uniform vec2 focal;
uniform vec2 viewport;

in vec2 position;
in int index;

out vec4 vColor;
out vec2 vPosition;

void main () {
    uvec4 cen = texelFetch(u_texture, ivec2((uint(index) & 0x3ffu) << 1, uint(index) >> 10), 0);
    vec4 cam = view * vec4(uintBitsToFloat(cen.xyz), 1);
    vec4 pos2d = projection * cam;

    float clip = 1.2 * pos2d.w;
    if (pos2d.z < -clip || pos2d.x < -clip || pos2d.x > clip || pos2d.y < -clip || pos2d.y > clip) {
        gl_Position = vec4(0.0, 0.0, 2.0, 1.0);
        return;
    }

    uvec4 cov = texelFetch(u_texture, ivec2(((uint(index) & 0x3ffu) << 1) | 1u, uint(index) >> 10), 0);
    vec2 u1 = unpackHalf2x16(cov.x), u2 = unpackHalf2x16(cov.y), u3 = unpackHalf2x16(cov.z);
    mat3 Vrk = mat3(u1.x, u1.y, u2.x, u1.y, u2.y, u3.x, u2.x, u3.x, u3.y);

    mat3 J = mat3(
        focal.x / cam.z, 0., -(focal.x * cam.x) / (cam.z * cam.z),
        0., -focal.y / cam.z, (focal.y * cam.y) / (cam.z * cam.z),
        0., 0., 0.
    );

    mat3 T = transpose(mat3(view)) * J;
    mat3 cov2d = transpose(T) * Vrk * T;

    float mid = (cov2d[0][0] + cov2d[1][1]) / 2.0;
    float radius = length(vec2((cov2d[0][0] - cov2d[1][1]) / 2.0, cov2d[0][1]));
    float lambda1 = mid + radius, lambda2 = mid - radius;

    if(lambda2 < 0.0) return;
    vec2 diagonalVector = normalize(vec2(cov2d[0][1], lambda1 - cov2d[0][0]));
    vec2 majorAxis = min(sqrt(2.0 * lambda1), 1024.0) * diagonalVector;
    vec2 minorAxis = min(sqrt(2.0 * lambda2), 1024.0) * vec2(diagonalVector.y, -diagonalVector.x);

    vColor = clamp(pos2d.z/pos2d.w+1.0, 0.0, 1.0) * vec4((cov.w) & 0xffu, (cov.w >> 8) & 0xffu, (cov.w >> 16) & 0xffu, (cov.w >> 24) & 0xffu) / 255.0;
    vPosition = position;

    vec2 vCenter = vec2(pos2d) / pos2d.w;
    gl_Position = vec4(
        vCenter
        + position.x * majorAxis / viewport
        + position.y * minorAxis / viewport, 0.0, 1.0);
}
`;

const FRAG_SRC = `#version 300 es
precision highp float;

in vec4 vColor;
in vec2 vPosition;
out vec4 fragColor;

void main () {
    float A = -dot(vPosition, vPosition);
    if (A < -4.0) discard;
    float B = exp(A) * vColor.a;
    fragColor = vec4(B * vColor.rgb, B);
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
gl.useProgram(program);

const u_proj = gl.getUniformLocation(program, 'projection');
const u_view = gl.getUniformLocation(program, 'view');
const u_focal = gl.getUniformLocation(program, 'focal');
const u_viewport = gl.getUniformLocation(program, 'viewport');
const u_texture = gl.getUniformLocation(program, 'u_texture');

// ─── GL state (set once, like antimatter15) ───
gl.disable(gl.DEPTH_TEST);
gl.enable(gl.BLEND);
gl.blendFuncSeparate(gl.ONE_MINUS_DST_ALPHA, gl.ONE, gl.ONE_MINUS_DST_ALPHA, gl.ONE);
gl.blendEquationSeparate(gl.FUNC_ADD, gl.FUNC_ADD);

// ─── Shared quad (4 vertices, instanced) ───
const quadVerts = new Float32Array([-2, -2, 2, -2, 2, 2, -2, 2]);
const quadBuf = gl.createBuffer();
gl.bindBuffer(gl.ARRAY_BUFFER, quadBuf);
gl.bufferData(gl.ARRAY_BUFFER, quadVerts, gl.STATIC_DRAW);
const a_position = gl.getAttribLocation(program, 'position');
gl.enableVertexAttribArray(a_position);
gl.bindBuffer(gl.ARRAY_BUFFER, quadBuf);
gl.vertexAttribPointer(a_position, 2, gl.FLOAT, false, 0, 0);

// ─── Per-instance index buffer ───
const indexBuffer = gl.createBuffer();
const a_index = gl.getAttribLocation(program, 'index');
gl.enableVertexAttribArray(a_index);
gl.bindBuffer(gl.ARRAY_BUFFER, indexBuffer);
gl.vertexAttribIPointer(a_index, 1, gl.INT, false, 0, 0);
gl.vertexAttribDivisor(a_index, 1);

// ─── Texture ───
const splatTexture = gl.createTexture();
gl.bindTexture(gl.TEXTURE_2D, splatTexture);
gl.uniform1i(u_texture, 0);

// ─── Inline sort worker (matches antimatter15 approach) ───
function createSortWorker(self) {
    let buffer;
    let vertexCount = 0;
    let viewProj;
    const rowLength = 3 * 4 + 3 * 4 + 4 + 4;
    let lastProj = [];
    let lastVertexCount = 0;
    let sortRunning = false;

    var _floatView = new Float32Array(1);
    var _int32View = new Int32Array(_floatView.buffer);

    function floatToHalf(float) {
        _floatView[0] = float;
        var f = _int32View[0];
        var sign = (f >> 31) & 0x0001;
        var exp = (f >> 23) & 0x00ff;
        var frac = f & 0x007fffff;
        var newExp;
        if (exp == 0) { newExp = 0; }
        else if (exp < 113) {
            newExp = 0;
            frac |= 0x00800000;
            frac = frac >> (113 - exp);
            if (frac & 0x01000000) { newExp = 1; frac = 0; }
        } else if (exp < 142) { newExp = exp - 112; }
        else { newExp = 31; frac = 0; }
        return (sign << 15) | (newExp << 10) | (frac >> 13);
    }

    function packHalf2x16(x, y) {
        return (floatToHalf(x) | (floatToHalf(y) << 16)) >>> 0;
    }

    function generateTexture() {
        if (!buffer) return;
        const f_buffer = new Float32Array(buffer);
        const u_buffer = new Uint8Array(buffer);

        var texwidth = 1024 * 2;
        var texheight = Math.ceil((2 * vertexCount) / texwidth);
        var texdata = new Uint32Array(texwidth * texheight * 4);
        var texdata_c = new Uint8Array(texdata.buffer);
        var texdata_f = new Float32Array(texdata.buffer);

        for (let i = 0; i < vertexCount; i++) {
            // Position (reinterpret float bits directly)
            texdata_f[8 * i + 0] = f_buffer[8 * i + 0];
            texdata_f[8 * i + 1] = f_buffer[8 * i + 1];
            texdata_f[8 * i + 2] = f_buffer[8 * i + 2];

            // Color
            texdata_c[4 * (8 * i + 7) + 0] = u_buffer[32 * i + 24 + 0];
            texdata_c[4 * (8 * i + 7) + 1] = u_buffer[32 * i + 24 + 1];
            texdata_c[4 * (8 * i + 7) + 2] = u_buffer[32 * i + 24 + 2];
            texdata_c[4 * (8 * i + 7) + 3] = u_buffer[32 * i + 24 + 3];

            // Quaternion + Scale
            let scale = [
                f_buffer[8 * i + 3 + 0],
                f_buffer[8 * i + 3 + 1],
                f_buffer[8 * i + 3 + 2],
            ];
            let rot = [
                (u_buffer[32 * i + 28 + 0] - 128) / 128,
                (u_buffer[32 * i + 28 + 1] - 128) / 128,
                (u_buffer[32 * i + 28 + 2] - 128) / 128,
                (u_buffer[32 * i + 28 + 3] - 128) / 128,
            ];

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
            ].map((k, i) => k * scale[Math.floor(i / 3)]);

            const sigma = [
                M[0] * M[0] + M[3] * M[3] + M[6] * M[6],
                M[0] * M[1] + M[3] * M[4] + M[6] * M[7],
                M[0] * M[2] + M[3] * M[5] + M[6] * M[8],
                M[1] * M[1] + M[4] * M[4] + M[7] * M[7],
                M[1] * M[2] + M[4] * M[5] + M[7] * M[8],
                M[2] * M[2] + M[5] * M[5] + M[8] * M[8],
            ];

            texdata[8 * i + 4] = packHalf2x16(4 * sigma[0], 4 * sigma[1]);
            texdata[8 * i + 5] = packHalf2x16(4 * sigma[2], 4 * sigma[3]);
            texdata[8 * i + 6] = packHalf2x16(4 * sigma[4], 4 * sigma[5]);
        }

        self.postMessage({ texdata, texwidth, texheight }, [texdata.buffer]);
    }

    function runSort(viewProj) {
        if (!buffer) return;
        const f_buffer = new Float32Array(buffer);
        if (lastVertexCount == vertexCount) {
            let dot = lastProj[2] * viewProj[2] + lastProj[6] * viewProj[6] + lastProj[10] * viewProj[10];
            if (Math.abs(dot - 1) < 0.01) return;
        } else {
            generateTexture();
            lastVertexCount = vertexCount;
        }

        let maxDepth = -Infinity;
        let minDepth = Infinity;
        let sizeList = new Int32Array(vertexCount);
        for (let i = 0; i < vertexCount; i++) {
            let depth = ((viewProj[2] * f_buffer[8 * i + 0] +
                viewProj[6] * f_buffer[8 * i + 1] +
                viewProj[10] * f_buffer[8 * i + 2]) * 4096) | 0;
            sizeList[i] = depth;
            if (depth > maxDepth) maxDepth = depth;
            if (depth < minDepth) minDepth = depth;
        }

        let depthInv = (256 * 256 - 1) / (maxDepth - minDepth);
        let counts0 = new Uint32Array(256 * 256);
        for (let i = 0; i < vertexCount; i++) {
            sizeList[i] = ((sizeList[i] - minDepth) * depthInv) | 0;
            counts0[sizeList[i]]++;
        }
        let starts0 = new Uint32Array(256 * 256);
        for (let i = 1; i < 256 * 256; i++)
            starts0[i] = starts0[i - 1] + counts0[i - 1];
        let depthIndex = new Uint32Array(vertexCount);
        for (let i = 0; i < vertexCount; i++)
            depthIndex[starts0[sizeList[i]]++] = i;

        lastProj = viewProj;
        self.postMessage({ depthIndex, viewProj, vertexCount }, [depthIndex.buffer]);
    }

    const throttledSort = () => {
        if (!sortRunning) {
            sortRunning = true;
            let lastView = viewProj;
            runSort(lastView);
            setTimeout(() => {
                sortRunning = false;
                if (lastView !== viewProj) throttledSort();
            }, 0);
        }
    };

    self.onmessage = (e) => {
        if (e.data.buffer) {
            buffer = e.data.buffer;
            vertexCount = e.data.vertexCount;
        } else if (e.data.vertexCount) {
            vertexCount = e.data.vertexCount;
        } else if (e.data.view) {
            viewProj = e.data.view;
            throttledSort();
        }
    };
}

const worker = new Worker(
    URL.createObjectURL(
        new Blob(['(', createSortWorker.toString(), ')(self)'], {
            type: 'application/javascript',
        }),
    ),
);

// ─── State ───
let splatCount = 0;
let vertexCount = 0;

// ─── Worker message handler ───
worker.onmessage = (e) => {
    if (e.data.texdata) {
        const { texdata, texwidth, texheight } = e.data;
        gl.bindTexture(gl.TEXTURE_2D, splatTexture);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
        gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA32UI, texwidth, texheight, 0,
            gl.RGBA_INTEGER, gl.UNSIGNED_INT, texdata);
        gl.activeTexture(gl.TEXTURE0);
        gl.bindTexture(gl.TEXTURE_2D, splatTexture);
    } else if (e.data.depthIndex) {
        const { depthIndex, viewProj } = e.data;
        gl.bindBuffer(gl.ARRAY_BUFFER, indexBuffer);
        gl.bufferData(gl.ARRAY_BUFFER, depthIndex, gl.DYNAMIC_DRAW);
        vertexCount = e.data.vertexCount;
        lastSortTime = performance.now() - lastSortReqTime;
    }
};

// ─── Camera (orbit) ───
let camTheta = 4.236;
let camPhi = -0.023;
let camRadius = 6.5;
let camTarget = [0, 0, 0];

// Use antimatter15's focal length (from their default camera)
const focalX = 1159.5880733038064;
const focalY = 1164.6601287484507;

// antimatter15 projection matrix (positive Z forward convention)
function getProjectionMatrix(fx, fy, width, height) {
    const znear = 0.2;
    const zfar = 200;
    return new Float32Array([
        (2 * fx) / width, 0, 0, 0,
        0, -(2 * fy) / height, 0, 0,
        0, 0, zfar / (zfar - znear), 1,
        0, 0, -(zfar * znear) / (zfar - znear), 0,
    ]);
}

// View matrix matching antimatter15's convention (cam.z > 0 for visible objects)
// Uses the same matrix layout as antimatter15's getViewMatrix
function getViewMatrix(eye, target, up) {
    // Forward = normalize(target - eye) → maps to +Z in camera space
    let fx = target[0] - eye[0], fy = target[1] - eye[1], fz = target[2] - eye[2];
    let len = Math.sqrt(fx * fx + fy * fy + fz * fz) || 1;
    fx /= len; fy /= len; fz /= len;

    // Right = normalize(worldUp × forward) → matches antimatter15 convention
    let rx = up[1] * fz - up[2] * fy;
    let ry = up[2] * fx - up[0] * fz;
    let rz = up[0] * fy - up[1] * fx;
    len = Math.sqrt(rx * rx + ry * ry + rz * rz) || 1;
    rx /= len; ry /= len; rz /= len;

    // True up = normalize(forward × right) → maps to +Y in camera space
    const ux = fy * rz - fz * ry;
    const uy = fz * rx - fx * rz;
    const uz = fx * ry - fy * rx;

    // Column-major 4x4 (same layout as antimatter15's view matrices)
    return new Float32Array([
        rx, ux, fx, 0,
        ry, uy, fy, 0,
        rz, uz, fz, 0,
        -(rx * eye[0] + ry * eye[1] + rz * eye[2]),
        -(ux * eye[0] + uy * eye[1] + uz * eye[2]),
        -(fx * eye[0] + fy * eye[1] + fz * eye[2]),
        1,
    ]);
}

function multiply4(a, b) {
    return new Float32Array([
        b[0]*a[0]+b[1]*a[4]+b[2]*a[8]+b[3]*a[12],
        b[0]*a[1]+b[1]*a[5]+b[2]*a[9]+b[3]*a[13],
        b[0]*a[2]+b[1]*a[6]+b[2]*a[10]+b[3]*a[14],
        b[0]*a[3]+b[1]*a[7]+b[2]*a[11]+b[3]*a[15],
        b[4]*a[0]+b[5]*a[4]+b[6]*a[8]+b[7]*a[12],
        b[4]*a[1]+b[5]*a[5]+b[6]*a[9]+b[7]*a[13],
        b[4]*a[2]+b[5]*a[6]+b[6]*a[10]+b[7]*a[14],
        b[4]*a[3]+b[5]*a[7]+b[6]*a[11]+b[7]*a[15],
        b[8]*a[0]+b[9]*a[4]+b[10]*a[8]+b[11]*a[12],
        b[8]*a[1]+b[9]*a[5]+b[10]*a[9]+b[11]*a[13],
        b[8]*a[2]+b[9]*a[6]+b[10]*a[10]+b[11]*a[14],
        b[8]*a[3]+b[9]*a[7]+b[10]*a[11]+b[11]*a[15],
        b[12]*a[0]+b[13]*a[4]+b[14]*a[8]+b[15]*a[12],
        b[12]*a[1]+b[13]*a[5]+b[14]*a[9]+b[15]*a[13],
        b[12]*a[2]+b[13]*a[6]+b[14]*a[10]+b[15]*a[14],
        b[12]*a[3]+b[13]*a[7]+b[14]*a[11]+b[15]*a[15],
    ]);
}

function getCameraPos() {
    return [
        camTarget[0] + camRadius * Math.cos(camPhi) * Math.sin(camTheta),
        camTarget[1] + camRadius * Math.sin(camPhi),
        camTarget[2] + camRadius * Math.cos(camPhi) * Math.cos(camTheta),
    ];
}

// ─── Resize ───
function resize() {
    canvas.width = innerWidth;
    canvas.height = innerHeight;
    gl.viewport(0, 0, canvas.width, canvas.height);
    gl.uniform2fv(u_focal, new Float32Array([focalX, focalY]));
    gl.uniform2fv(u_viewport, new Float32Array([innerWidth, innerHeight]));
    gl.uniformMatrix4fv(u_proj, false,
        getProjectionMatrix(focalX, focalY, innerWidth, innerHeight));
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

// ─── Touch: Orbit + Pinch-to-Zoom ───
let touchDragging = false;
let touchLastX = 0, touchLastY = 0;
let touchStartDist = 0, touchStartRadius = 0;

canvas.addEventListener('touchstart', e => {
    e.preventDefault();
    if (e.touches.length === 1) {
        touchDragging = true;
        touchLastX = e.touches[0].clientX;
        touchLastY = e.touches[0].clientY;
    } else if (e.touches.length === 2) {
        touchDragging = false;
        const dx = e.touches[0].clientX - e.touches[1].clientX;
        const dy = e.touches[0].clientY - e.touches[1].clientY;
        touchStartDist = Math.sqrt(dx * dx + dy * dy);
        touchStartRadius = camRadius;
    }
}, { passive: false });

canvas.addEventListener('touchmove', e => {
    e.preventDefault();
    if (e.touches.length === 1 && touchDragging) {
        const dx = e.touches[0].clientX - touchLastX;
        const dy = e.touches[0].clientY - touchLastY;
        touchLastX = e.touches[0].clientX;
        touchLastY = e.touches[0].clientY;
        camTheta -= dx * 0.005;
        camPhi += dy * 0.005;
        camPhi = Math.max(-1.5, Math.min(1.5, camPhi));
    } else if (e.touches.length === 2) {
        const dx = e.touches[0].clientX - e.touches[1].clientX;
        const dy = e.touches[0].clientY - e.touches[1].clientY;
        const dist = Math.sqrt(dx * dx + dy * dy);
        const scale = touchStartDist / dist;
        camRadius = Math.max(0.1, Math.min(100, touchStartRadius * scale));
    }
}, { passive: false });

canvas.addEventListener('touchend', e => {
    e.preventDefault();
    if (e.touches.length === 0) touchDragging = false;
    else if (e.touches.length === 1) {
        touchDragging = true;
        touchLastX = e.touches[0].clientX;
        touchLastY = e.touches[0].clientY;
    }
}, { passive: false });

// ─── Keyboard ───
document.addEventListener('keydown', (e) => {
    if (e.key === 'r' || e.key === 'R') {
        camTheta = 4.236; camPhi = -0.023; camRadius = 6.5; camTarget = [0, 0, 0];
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

// ─── Load scene ───
async function loadScene(name) {
    const url = SCENES[name];
    if (!url) return;

    loading.style.display = 'flex';
    barFill.style.width = '0%';
    barText.textContent = `Fetching ${name}...`;
    splatCount = 0;
    vertexCount = 0;

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

        const rowLength = 32;
        splatCount = Math.floor(received / rowLength);

        // Send buffer to worker (it generates texture + sorts)
        worker.postMessage({
            buffer: buffer,
            vertexCount: splatCount,
        });

        camTarget = [0, 0, 0];
        camRadius = 6.5; camTheta = 4.236; camPhi = -0.023;

        document.getElementById('s-count').textContent = splatCount.toLocaleString();
        document.getElementById('i-file').textContent = name + '.splat';
        document.getElementById('i-size').textContent = (received / 1e6).toFixed(1) + ' MB';
        document.getElementById('i-gaussians').textContent = splatCount.toLocaleString();

        loading.style.display = 'none';
    } catch (err) {
        barText.textContent = `Error: ${err.message}`;
        console.error(err);
    }
}

// ─── Render ───
let lastTime = performance.now();
let frames = 0, fps = 0;
let lastSortTime = 0, lastSortReqTime = 0;

function render(now) {
    const eye = getCameraPos();
    const viewMatrix = getViewMatrix(eye, camTarget, [0, 1, 0]);
    const projMatrix = getProjectionMatrix(focalX, focalY, canvas.width, canvas.height);
    const viewProj = multiply4(projMatrix, viewMatrix);
    lastSortReqTime = performance.now();
    worker.postMessage({ view: viewProj });

    gl.clear(gl.COLOR_BUFFER_BIT);

    if (vertexCount > 0) {
        gl.uniformMatrix4fv(u_view, false, viewMatrix);
        gl.drawArraysInstanced(gl.TRIANGLE_FAN, 0, 4, vertexCount);
    }

    // HUD
    document.getElementById('i-camera').textContent =
        `θ=${(camTheta * 180 / Math.PI).toFixed(0)}° φ=${(camPhi * 180 / Math.PI).toFixed(0)}° r=${camRadius.toFixed(1)}`;

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
