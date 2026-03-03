// Depth sort worker for gaussian splats
// Uses a 16-bit counting sort (256x256 histogram) — same approach as antimatter15/splat
// Sorts back-to-front (farthest first) for alpha compositing

self.onmessage = (e) => {
    const { positions, viewProj, count } = e.data;
    const t0 = performance.now();

    // Compute depth for each splat using the view-projection matrix row 2
    const m2 = viewProj[2], m6 = viewProj[6], m10 = viewProj[10], m14 = viewProj[14];

    // Find depth range
    let minDepth = Infinity, maxDepth = -Infinity;
    const depths = new Float32Array(count);
    for (let i = 0; i < count; i++) {
        const off = i * 3;
        const d = m2 * positions[off] + m6 * positions[off + 1] + m10 * positions[off + 2] + m14;
        depths[i] = d;
        if (d < minDepth) minDepth = d;
        if (d > maxDepth) maxDepth = d;
    }

    // Map depths to 16-bit integers (0 = farthest, 65535 = closest for back-to-front)
    const range = maxDepth - minDepth || 1;
    const depthInv = 65535 / range;

    const sortKeys = new Uint32Array(count);
    for (let i = 0; i < count; i++) {
        // Invert: farthest gets smallest key → sorted first (back-to-front)
        sortKeys[i] = ((maxDepth - depths[i]) * depthInv) | 0;
    }

    // 16-bit counting sort via two 8-bit passes
    const result = new Uint32Array(count);
    const temp = new Uint32Array(count);

    // Initialize result as identity
    for (let i = 0; i < count; i++) result[i] = i;

    // Pass 1: sort by low 8 bits
    const counts1 = new Uint32Array(256);
    for (let i = 0; i < count; i++) counts1[sortKeys[i] & 0xFF]++;
    let sum = 0;
    for (let i = 0; i < 256; i++) { const c = counts1[i]; counts1[i] = sum; sum += c; }
    for (let i = 0; i < count; i++) {
        const idx = result[i];
        temp[counts1[sortKeys[idx] & 0xFF]++] = idx;
    }

    // Pass 2: sort by high 8 bits
    const counts2 = new Uint32Array(256);
    for (let i = 0; i < count; i++) counts2[(sortKeys[temp[i]] >> 8) & 0xFF]++;
    sum = 0;
    for (let i = 0; i < 256; i++) { const c = counts2[i]; counts2[i] = sum; sum += c; }
    for (let i = 0; i < count; i++) {
        const idx = temp[i];
        result[counts2[(sortKeys[idx] >> 8) & 0xFF]++] = idx;
    }

    const timeMs = performance.now() - t0;
    self.postMessage({ sortedIndices: result, timeMs }, [result.buffer]);
};
