#!/usr/bin/env python3
"""
Extract ALL weights from SmolLM2-135M into per-tensor binary files for the neural net visualizer.

Output:
  data/meta.json        — tensor metadata (names, shapes, positions, stats, file refs)
  data/tensor_NNN.bin   — per-tensor float32 arrays: [x, y, w, h, r, g, b, a] × N

134,515,008 total parameters. Each weight = one GPU point cloud point.
Large tensors (>5M params) are split into chunks for browser memory safety.
"""

import struct
import json
import math
import numpy as np
from pathlib import Path
from safetensors import safe_open

MODEL_PATH = Path.home() / ".cache/huggingface/hub/models--HuggingFaceTB--SmolLM2-135M/snapshots/93efa2f097d58c2a74874c7e644dbc9b0cee75a2/model.safetensors"
OUT_DIR = Path(__file__).parent / "data"

# Layout constants
POINT_SIZE = 1.0
TENSOR_GAP = 200
LAYER_GAP = 600
BLOCK_GAP = 400

# Max points per file (~32MB per chunk = 1M points × 8 floats × 4 bytes)
CHUNK_SIZE = 1_000_000


def weight_to_rgba(val, vmin, vmax):
    """Diverging colormap: blue (negative) → near-black (zero) → amber (positive)."""
    absmax = max(abs(vmin), abs(vmax), 1e-8)
    t = np.clip(val / absmax, -1.0, 1.0)
    r = np.where(t > 0, 0.1 + 0.9 * t, 0.1 + 0.4 * (-t))
    g = np.where(t > 0, 0.1 + 0.6 * t, 0.1 + 0.2 * (-t))
    b = np.where(t > 0, 0.1 + 0.1 * t, 0.1 + 0.9 * (-t))
    a = np.full_like(t, 0.85)
    return r.astype(np.float32), g.astype(np.float32), b.astype(np.float32), a.astype(np.float32)


def get_tensor_order():
    return {
        'self_attn.q_proj.weight': (0, 'Q Projection'),
        'self_attn.k_proj.weight': (1, 'K Projection'),
        'self_attn.v_proj.weight': (2, 'V Projection'),
        'self_attn.o_proj.weight': (3, 'O Projection'),
        'mlp.gate_proj.weight': (4, 'Gate Projection'),
        'mlp.up_proj.weight': (5, 'Up Projection'),
        'mlp.down_proj.weight': (6, 'Down Projection'),
        'input_layernorm.weight': (7, 'Input LayerNorm'),
        'post_attention_layernorm.weight': (8, 'Post-Attn LayerNorm'),
    }


def build_points(rows, cols, base_x, base_y, vals):
    """Build the full [x,y,w,h,r,g,b,a] array for a weight matrix."""
    ys_grid, xs_grid = np.mgrid[0:rows, 0:cols]
    xs = (xs_grid.ravel().astype(np.float32) * POINT_SIZE) + base_x
    ys = (ys_grid.ravel().astype(np.float32) * POINT_SIZE) + base_y
    r, g, b, a = weight_to_rgba(vals, vals.min(), vals.max())
    w = np.full(len(vals), POINT_SIZE, dtype=np.float32)
    h = np.full(len(vals), POINT_SIZE, dtype=np.float32)
    return np.column_stack([xs, ys, w, h, r, g, b, a]).ravel()


def write_chunks(data, tensor_idx, out_dir):
    """Write point data as one or more chunk files. Returns list of {file, points}."""
    total_floats = len(data)
    total_points = total_floats // 8
    chunks = []

    if total_points <= CHUNK_SIZE:
        fname = f"tensor_{tensor_idx:03d}.bin"
        data.tofile(str(out_dir / fname))
        chunks.append({"file": fname, "points": total_points})
    else:
        # Split into chunks
        chunk_floats = CHUNK_SIZE * 8
        offset = 0
        part = 0
        while offset < total_floats:
            end = min(offset + chunk_floats, total_floats)
            fname = f"tensor_{tensor_idx:03d}_{part:02d}.bin"
            data[offset:end].tofile(str(out_dir / fname))
            pts = (end - offset) // 8
            chunks.append({"file": fname, "points": pts})
            offset = end
            part += 1

    return chunks


def main():
    OUT_DIR.mkdir(parents=True, exist_ok=True)

    print(f"Loading model from {MODEL_PATH}")
    f = safe_open(str(MODEL_PATH), framework="pt")
    keys = list(f.keys())
    print(f"Found {len(keys)} tensors")

    tensor_order = get_tensor_order()
    meta = {"tensors": [], "total_points": 0, "model": "SmolLM2-135M"}
    tensor_idx = 0
    cursor_y = 0.0
    total_points = 0
    total_bytes = 0

    # ─── Embedding ───
    print("Processing embedding...")
    emb = f.get_tensor("model.embed_tokens.weight").float().numpy()
    rows, cols = emb.shape
    vals = emb.ravel()
    data = build_points(rows, cols, 0, cursor_y, vals)
    chunks = write_chunks(data, tensor_idx, OUT_DIR)
    fsize = sum(c["points"] for c in chunks) * 32
    total_bytes += fsize

    meta["tensors"].append({
        "name": "model.embed_tokens.weight",
        "label": "Token Embedding",
        "shape": [rows, cols],
        "numel": int(vals.size),
        "x": 0, "y": float(cursor_y),
        "w": cols * POINT_SIZE, "h": rows * POINT_SIZE,
        "min": float(vals.min()), "max": float(vals.max()),
        "mean": float(vals.mean()), "std": float(vals.std()),
        "chunks": chunks,
    })
    print(f"  embed_tokens: {rows}x{cols} = {vals.size:,} points ({len(chunks)} chunks, {fsize/1024/1024:.1f}MB)")
    total_points += vals.size
    tensor_idx += 1

    cursor_y += rows * POINT_SIZE + LAYER_GAP
    del emb, vals, data

    # ─── Transformer layers ───
    for layer_idx in range(30):
        prefix = f"model.layers.{layer_idx}"
        print(f"Processing layer {layer_idx}/29...")

        layer_tensors = []
        for k in keys:
            if k.startswith(prefix + "."):
                suffix = k[len(prefix) + 1:]
                if suffix in tensor_order:
                    order, label = tensor_order[suffix]
                    layer_tensors.append((order, k, suffix, label))
        layer_tensors.sort()

        # Compute attention block width
        attn_max_cols = 0
        for order, k, suffix, label in layer_tensors:
            if order <= 3:
                t = f.get_tensor(k)
                shape = list(t.shape)
                if len(shape) == 1:
                    shape = [1, shape[0]]
                attn_max_cols = max(attn_max_cols, shape[1])

        mlp_x = attn_max_cols * POINT_SIZE + BLOCK_GAP
        layer_max_y = cursor_y
        attn_cy = cursor_y
        mlp_cy = cursor_y

        for order, k, suffix, label in layer_tensors:
            t = f.get_tensor(k).float().numpy()
            if t.ndim == 1:
                t = t.reshape(1, -1)
            rows, cols = t.shape
            vals = t.ravel()

            if order <= 3:
                bx, by = 0.0, attn_cy
                attn_cy += rows * POINT_SIZE + TENSOR_GAP
            elif order <= 6:
                bx, by = mlp_x, mlp_cy
                mlp_cy += rows * POINT_SIZE + TENSOR_GAP
            else:
                bx, by = mlp_x + 1600 + BLOCK_GAP, cursor_y + (order - 7) * (POINT_SIZE + TENSOR_GAP)

            data = build_points(rows, cols, bx, by, vals)
            chunks = write_chunks(data, tensor_idx, OUT_DIR)
            fsize = sum(c["points"] for c in chunks) * 32

            meta["tensors"].append({
                "name": k,
                "label": f"Layer {layer_idx} — {label}",
                "shape": list(t.shape),
                "numel": int(vals.size),
                "x": float(bx), "y": float(by),
                "w": cols * POINT_SIZE, "h": rows * POINT_SIZE,
                "min": float(vals.min()), "max": float(vals.max()),
                "mean": float(vals.mean()), "std": float(vals.std()),
                "chunks": chunks,
            })

            total_points += vals.size
            total_bytes += fsize
            layer_max_y = max(layer_max_y, by + rows * POINT_SIZE)
            tensor_idx += 1
            del t, vals, data

        cursor_y = layer_max_y + LAYER_GAP

    # ─── Final norm ───
    print("Processing final norm...")
    norm = f.get_tensor("model.norm.weight").float().numpy().reshape(1, -1)
    rows, cols = norm.shape
    vals = norm.ravel()
    data = build_points(rows, cols, 0, cursor_y, vals)
    chunks = write_chunks(data, tensor_idx, OUT_DIR)

    meta["tensors"].append({
        "name": "model.norm.weight",
        "label": "Final LayerNorm",
        "shape": [rows, cols],
        "numel": int(vals.size),
        "x": 0, "y": float(cursor_y),
        "w": cols * POINT_SIZE, "h": rows * POINT_SIZE,
        "min": float(vals.min()), "max": float(vals.max()),
        "mean": float(vals.mean()), "std": float(vals.std()),
        "chunks": chunks,
    })
    total_points += vals.size

    # ─── Write metadata ───
    meta["total_points"] = total_points
    meta["world_width"] = float(max(t["x"] + t["w"] for t in meta["tensors"]))
    meta["world_height"] = float(cursor_y + rows * POINT_SIZE)

    json_path = OUT_DIR / "meta.json"
    with open(json_path, "w") as jf:
        json.dump(meta, jf, indent=2)

    print(f"\nTotal points: {total_points:,}")
    print(f"Total data: {total_bytes / 1024 / 1024:.1f} MB across {tensor_idx + 1} tensors")
    print(f"World size: {meta['world_width']:.0f} x {meta['world_height']:.0f}")
    print(f"Files written to {OUT_DIR}")


if __name__ == "__main__":
    main()
