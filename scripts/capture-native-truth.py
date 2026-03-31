#!/usr/bin/env python3
"""
Capture native Rendero headless output and metadata in a browser-comparable form.

Runs the native shell in headless mode, saves:
- native headless metadata JSON
- native headless screenshot PNG
- synthesized `elements` list for pairwise comparison against browser/WASM

Usage:
    python3 scripts/capture-native-truth.py <output.json> [width height]
"""

import json
import os
import shutil
import subprocess
import sys
from pathlib import Path


def derive_tag(name: str | None, kind: str | None) -> str:
    if name:
        prefix = name.split("_", 1)[0].strip()
        if prefix:
            return prefix.lower()
    if kind:
        return kind.lower()
    return "unknown"


def synthesize_elements(metadata: dict) -> list[dict]:
    engine_model = metadata.get("engineModel") or []
    engine_layout = metadata.get("engineLayout") or []
    model_by_key = {entry.get("key"): entry for entry in engine_model if entry.get("key")}
    child_counts: dict[str, int] = {}
    for entry in engine_layout:
        parent_key = entry.get("parentKey")
        if parent_key:
            child_counts[parent_key] = child_counts.get(parent_key, 0) + 1

    elements = []
    next_id = 0
    id_by_key: dict[str, int] = {}
    for entry in engine_layout:
        key = entry.get("key")
        model = model_by_key.get(key, {})
        name = entry.get("name")
        kind = entry.get("kind")
        tag = derive_tag(name, kind)
        if tag == "document":
            continue

        parent_key = entry.get("parentKey")
        if parent_key and parent_key not in id_by_key and parent_key in model_by_key:
            # Parent may be the skipped document root.
            parent_id = None
        else:
            parent_id = id_by_key.get(parent_key)

        text_payload = model.get("text") or {}
        text = (text_payload.get("text") or "")[:200]
        bounds = entry.get("bounds") or {}
        element = {
            "id": next_id,
            "parentId": parent_id,
            "depth": 0,
            "tag": tag,
            "engineId": None,
            "text": text,
            "bounds": {
                "x": round(float(bounds.get("x", 0)), 1),
                "y": round(float(bounds.get("y", 0)), 1),
                "width": round(float(bounds.get("width", 0)), 1),
                "height": round(float(bounds.get("height", 0)), 1),
            },
            "styles": {},
            "computedStyle": {},
            "childCount": child_counts.get(key, 0),
            "engineModel": model,
        }
        elements.append(element)
        if key:
            id_by_key[key] = next_id
        next_id += 1

    # Derive depth once ids are stable.
    by_id = {el["id"]: el for el in elements}
    for el in elements:
        depth = 0
        cursor = el.get("parentId")
        while cursor is not None and cursor in by_id:
            depth += 1
            cursor = by_id[cursor].get("parentId")
        el["depth"] = depth
    return elements


def main() -> int:
    if len(sys.argv) < 2:
        print(f"Usage: {sys.argv[0]} <output.json> [width height]", file=sys.stderr)
        return 1

    output_json = Path(sys.argv[1]).resolve()
    output_json.parent.mkdir(parents=True, exist_ok=True)
    width = int(sys.argv[2]) if len(sys.argv) > 2 else 1440
    height = int(sys.argv[3]) if len(sys.argv) > 3 else 900

    ppm_path = output_json.with_suffix(".ppm")
    png_path = output_json.with_suffix(".png")
    metadata_path = output_json.with_suffix(".raw.json")

    env = os.environ.copy()
    env["RENDERO_DEMO"] = env.get("RENDERO_DEMO", "react")
    env["RENDERO_HEADLESS_DUMP"] = str(ppm_path)
    env["RENDERO_HEADLESS_METADATA"] = str(metadata_path)
    env["RENDERO_HEADLESS_WIDTH"] = str(width)
    env["RENDERO_HEADLESS_HEIGHT"] = str(height)

    root = Path(__file__).resolve().parent.parent
    subprocess.run(
        ["cargo", "run", "-p", "rendero-native-shell"],
        cwd=root,
        env=env,
        check=True,
    )

    if shutil.which("sips") and ppm_path.exists():
        subprocess.run(
            ["sips", "-s", "format", "png", str(ppm_path), "--out", str(png_path)],
            check=True,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )

    metadata = json.loads(metadata_path.read_text())
    elements = synthesize_elements(metadata)
    report = {
        "url": "native://rendero-headless",
        "title": "Rendero Native Headless",
        "timestamp": metadata.get("timestamp"),
        "viewport": metadata.get("viewport", {"width": width, "height": height, "dpr": 1}),
        "elementCount": len(elements),
        "elements": elements,
        "engineDocument": metadata.get("engineDocument"),
        "engineModel": metadata.get("engineModel"),
        "engineLayout": metadata.get("engineLayout"),
        "artifacts": {
            "ppm": str(ppm_path),
            "png": str(png_path),
            "rawMetadata": str(metadata_path),
        },
    }
    output_json.write_text(json.dumps(report, indent=2))
    print(f"[Native] {report['elementCount']} elements from native headless")
    print(f"[Native] Viewport: {width}x{height}")
    print(f"[Native] Saved to {output_json}")
    if png_path.exists():
        print(f"[Native] Screenshot: {png_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
