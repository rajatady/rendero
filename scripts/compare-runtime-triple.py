#!/usr/bin/env python3
"""
Compare browser oracle, browser Rendero (WASM), and native Rendero together.

Outputs pairwise accuracy for:
- browser vs wasm
- browser vs native
- wasm vs native
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

TOLERANCE = 1.0


def load_report(path: str) -> dict:
    return json.loads(Path(path).read_text())


def find_tree_offset(ref_els: list[dict], cmp_els: list[dict]) -> int:
    if not ref_els or not cmp_els:
        return 0
    target_tag = ref_els[0].get("tag")
    target_children = ref_els[0].get("childCount")
    for i, el in enumerate(cmp_els):
        if el.get("tag") != target_tag or el.get("childCount") != target_children:
            continue
        match_count = 0
        for j in range(min(6, len(ref_els), len(cmp_els) - i)):
            if ref_els[j].get("tag") == cmp_els[i + j].get("tag"):
                match_count += 1
        if match_count >= 3:
            return i
    return 0


def pair_elements(ref_els: list[dict], cmp_els: list[dict]) -> tuple[list[tuple[dict, dict]], int]:
    offset = find_tree_offset(ref_els, cmp_els)
    cmp_slice = cmp_els[offset:]
    pairs = [(ref_els[i], cmp_slice[i]) for i in range(min(len(ref_els), len(cmp_slice)))]
    return pairs, offset


def compare_bounds(a: dict, b: dict) -> list[dict]:
    mismatches = []
    for prop in ("x", "y", "width", "height"):
        av = float(a.get(prop, 0))
        bv = float(b.get(prop, 0))
        diff = abs(av - bv)
        if diff >= TOLERANCE:
            mismatches.append({
                "property": prop,
                "a": av,
                "b": bv,
                "diff": round(diff, 1),
            })
    return mismatches


def summarize_surface(name: str, report: dict) -> dict:
    els = report.get("elements") or []
    lead = []
    for el in els[:4]:
        lead.append({
            "tag": el.get("tag"),
            "text": (el.get("text") or "")[:40],
            "height": el.get("bounds", {}).get("height", 0),
            "width": el.get("bounds", {}).get("width", 0),
        })
    return {
        "name": name,
        "elementCount": len(els),
        "viewport": report.get("viewport"),
        "leadingElements": lead,
    }


def compare_pair(name_a: str, report_a: dict, name_b: str, report_b: dict) -> dict:
    els_a = report_a.get("elements") or []
    els_b = report_b.get("elements") or []
    pairs, offset = pair_elements(els_a, els_b)
    origin_a = pairs[0][0]["bounds"] if pairs else {"x": 0, "y": 0}
    origin_b = pairs[0][1]["bounds"] if pairs else {"x": 0, "y": 0}

    mismatches = []
    total_properties = 0
    match_count = 0
    for left, right in pairs:
        lb = dict(left["bounds"])
        rb = dict(right["bounds"])
        lb["x"] -= origin_a["x"]
        lb["y"] -= origin_a["y"]
        rb["x"] -= origin_b["x"]
        rb["y"] -= origin_b["y"]
        mm = compare_bounds(lb, rb)
        total_properties += 4
        match_count += 4 - len(mm)
        if mm:
            mismatches.append({
                "index": left.get("id"),
                "tag": left.get("tag"),
                "text": (left.get("text") or "")[:60],
                "leftBounds": lb,
                "rightBounds": rb,
                "mismatches": mm,
            })

    accuracy = round((match_count / max(total_properties, 1)) * 100, 2)
    return {
        "pair": f"{name_a}Vs{name_b}",
        "left": name_a,
        "right": name_b,
        "leftCount": len(els_a),
        "rightCount": len(els_b),
        "offset": offset,
        "origin": {
            name_a: {"x": origin_a["x"], "y": origin_a["y"]},
            name_b: {"x": origin_b["x"], "y": origin_b["y"]},
        },
        "elementsPaired": len(pairs),
        "totalProperties": total_properties,
        "matchCount": match_count,
        "mismatchCount": total_properties - match_count,
        "elementsWithMismatches": len(mismatches),
        "accuracy": accuracy,
        "mismatches": mismatches,
    }


def main() -> int:
    if len(sys.argv) < 5:
        print(
            f"Usage: {sys.argv[0]} <browser.json> <wasm.json> <native.json> <output.json>",
            file=sys.stderr,
        )
        return 1

    browser = load_report(sys.argv[1])
    wasm = load_report(sys.argv[2])
    native = load_report(sys.argv[3])

    browser_vs_wasm = compare_pair("browser", browser, "wasm", wasm)
    browser_vs_native = compare_pair("browser", browser, "native", native)
    wasm_vs_native = compare_pair("wasm", wasm, "native", native)

    result = {
        "tolerance": TOLERANCE,
        "surfaces": {
            "browser": summarize_surface("browser", browser),
            "wasm": summarize_surface("wasm", wasm),
            "native": summarize_surface("native", native),
        },
        "pairwise": {
            "browserVsWasm": browser_vs_wasm,
            "browserVsNative": browser_vs_native,
            "wasmVsNative": wasm_vs_native,
        },
    }
    output_path = Path(sys.argv[4])
    output_path.write_text(json.dumps(result, indent=2))

    for pair in (browser_vs_wasm, browser_vs_native, wasm_vs_native):
        print(
            f"{pair['pair']}: {pair['accuracy']}% "
            f"({pair['matchCount']}/{pair['totalProperties']}) "
            f"paired={pair['elementsPaired']} mismatchingElements={pair['elementsWithMismatches']}"
        )

    print(f"Saved to {output_path}")
    return 0 if all(p["mismatchCount"] == 0 for p in (browser_vs_wasm, browser_vs_native, wasm_vs_native)) else 1


if __name__ == "__main__":
    raise SystemExit(main())
