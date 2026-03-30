#!/usr/bin/env python3
"""
Compare browser ground truth vs Rendero engine layout.

Reads two JSON files (browser + engine), matches elements by tree position,
compares bounds, reports mismatches.

Usage:
    python3 scripts/compare-layout.py accuracy/apple-web.json accuracy/apple-engine.json

Output:
    Per-element comparison with pass/fail, overall accuracy percentage.
"""

import json
import sys
from pathlib import Path

TOLERANCE = 1.0  # px — mismatch threshold


def load_report(path):
    return json.loads(Path(path).read_text())


def find_tree_offset(browser_els, engine_els):
    """Find the offset in engine_els that aligns with browser_els[0].

    The browser extractor starts from #root, the engine extractor starts from
    <body>. We look for the engine element whose (tag, childCount) matches
    browser_els[0] and whose subtree structure matches best.
    """
    if not browser_els or not engine_els:
        return 0

    target_tag = browser_els[0]['tag']
    target_children = browser_els[0]['childCount']

    # Look for matching root in engine tree
    for i, eng_el in enumerate(engine_els):
        if eng_el['tag'] == target_tag and eng_el['childCount'] == target_children:
            # Check next few elements match too
            match_count = 0
            for j in range(min(5, len(browser_els), len(engine_els) - i)):
                if browser_els[j]['tag'] == engine_els[i + j]['tag']:
                    match_count += 1
            if match_count >= 3:
                return i
    return 0


def match_elements(browser_els, engine_els):
    """Match elements by tree structure, accounting for root offset."""
    offset = find_tree_offset(browser_els, engine_els)
    if offset > 0:
        print(f"  (aligned engine elements at offset {offset})")

    pairs = []
    engine_slice = engine_els[offset:]
    for i in range(min(len(browser_els), len(engine_slice))):
        pairs.append((browser_els[i], engine_slice[i]))
    return pairs


def compare_bounds(browser_bounds, engine_bounds):
    """Compare two bounds dicts. Returns list of mismatching properties."""
    mismatches = []
    for prop in ['x', 'y', 'width', 'height']:
        bv = browser_bounds.get(prop, 0)
        ev = engine_bounds.get(prop, 0)
        diff = abs(bv - ev)
        if diff >= TOLERANCE:
            mismatches.append({
                'property': prop,
                'browser': bv,
                'engine': ev,
                'diff': round(diff, 1),
            })
    return mismatches


def main():
    if len(sys.argv) < 3:
        print(f"Usage: {sys.argv[0]} <browser.json> <engine.json>", file=sys.stderr)
        return 1

    browser_report = load_report(sys.argv[1])
    engine_report = load_report(sys.argv[2])

    browser_els = browser_report['elements']
    engine_els = engine_report['elements']

    print(f"Browser: {len(browser_els)} elements")
    print(f"Engine:  {len(engine_els)} elements")
    print(f"Tolerance: {TOLERANCE}px")
    print()

    if len(browser_els) != len(engine_els):
        print(f"⚠ Element count mismatch: browser={len(browser_els)} engine={len(engine_els)}")
        print()

    pairs = match_elements(browser_els, engine_els)

    # Normalize coordinates: subtract root element's origin so both start at (0,0)
    browser_origin_x = pairs[0][0]['bounds']['x'] if pairs else 0
    browser_origin_y = pairs[0][0]['bounds']['y'] if pairs else 0
    engine_origin_x = pairs[0][1]['bounds']['x'] if pairs else 0
    engine_origin_y = pairs[0][1]['bounds']['y'] if pairs else 0
    print(f"  Browser origin: ({browser_origin_x}, {browser_origin_y})")
    print(f"  Engine origin:  ({engine_origin_x}, {engine_origin_y})")
    print()

    total_properties = 0
    match_count = 0
    mismatch_count = 0
    element_mismatches = []

    for browser_el, engine_el in pairs:
        # Normalize positions relative to root
        browser_el = dict(browser_el)
        engine_el = dict(engine_el)
        browser_el['bounds'] = dict(browser_el['bounds'])
        engine_el['bounds'] = dict(engine_el['bounds'])
        browser_el['bounds']['x'] -= browser_origin_x
        browser_el['bounds']['y'] -= browser_origin_y
        engine_el['bounds']['x'] -= engine_origin_x
        engine_el['bounds']['y'] -= engine_origin_y
        mismatches = compare_bounds(browser_el['bounds'], engine_el['bounds'])
        total_properties += 4  # x, y, width, height

        if mismatches:
            mismatch_count += len(mismatches)
            match_count += 4 - len(mismatches)
            element_mismatches.append({
                'index': browser_el['id'],
                'tag': browser_el['tag'],
                'text': browser_el.get('text', '')[:40],
                'browser_bounds': browser_el['bounds'],
                'engine_bounds': engine_el['bounds'],
                'mismatches': mismatches,
            })
        else:
            match_count += 4

    # Summary
    pct = (match_count / total_properties * 100) if total_properties > 0 else 0
    print(f"{'='*60}")
    print(f"RESULTS: {match_count}/{total_properties} properties match ({pct:.1f}%)")
    print(f"         {len(pairs)} elements compared, {len(element_mismatches)} with mismatches")
    print(f"{'='*60}")
    print()

    if element_mismatches:
        print("MISMATCHES:")
        print()
        for em in element_mismatches[:50]:  # Show first 50
            tag = em['tag']
            text = em['text']
            label = f"<{tag}>" + (f' "{text}"' if text else '')
            print(f"  [{em['index']}] {label}")
            print(f"       browser: x={em['browser_bounds']['x']}, y={em['browser_bounds']['y']}, "
                  f"w={em['browser_bounds']['width']}, h={em['browser_bounds']['height']}")
            print(f"       engine:  x={em['engine_bounds']['x']}, y={em['engine_bounds']['y']}, "
                  f"w={em['engine_bounds']['width']}, h={em['engine_bounds']['height']}")
            for m in em['mismatches']:
                print(f"       ✗ {m['property']}: browser={m['browser']} engine={m['engine']} (diff={m['diff']}px)")
            print()

        if len(element_mismatches) > 50:
            print(f"  ... and {len(element_mismatches) - 50} more")

    # Save results
    if len(sys.argv) > 3:
        output_path = Path(sys.argv[3])
        results = {
            'totalProperties': total_properties,
            'matchCount': match_count,
            'mismatchCount': mismatch_count,
            'accuracy': round(pct, 2),
            'elementsPaired': len(pairs),
            'elementsWithMismatches': len(element_mismatches),
            'mismatches': element_mismatches,
        }
        output_path.write_text(json.dumps(results, indent=2))
        print(f"Saved to {output_path}")

    return 0 if mismatch_count == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
