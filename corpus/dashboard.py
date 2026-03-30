#!/usr/bin/env python3
"""
Generate a corpus dashboard from all captured ground truth files.

Summarizes the CSS property coverage, layout patterns, and complexity
across all corpus sites. Outputs corpus/dashboard.json.

Usage:
    python3 corpus/dashboard.py
"""

import json
import sys
from collections import Counter
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
TRUTH_DIR = REPO / "corpus" / "ground-truth"
SITES_DIR = REPO / "corpus" / "sites"


def analyze_site(truth_path):
    """Analyze a single ground truth file."""
    gt = json.loads(truth_path.read_text())

    # CSS property usage across all elements
    prop_usage = Counter()
    prop_values = {}  # prop -> set of unique values (sample)
    display_values = Counter()
    position_values = Counter()
    has_gradient = 0
    has_shadow = 0
    has_transform = 0
    has_animation = 0
    has_transition = 0
    has_border_radius = 0
    has_overflow_hidden = 0
    has_grid = 0
    has_flex = 0
    total_styles = 0

    for el in gt["elements"]:
        styles = el.get("styles", {})
        total_styles += len(styles)

        for prop, val in styles.items():
            prop_usage[prop] += 1
            if prop not in prop_values:
                prop_values[prop] = set()
            if len(prop_values[prop]) < 20:  # cap unique values
                prop_values[prop].add(str(val)[:100])

        d = styles.get("display", "")
        if d:
            display_values[d] += 1
        p = styles.get("position", "")
        if p:
            position_values[p] += 1

        if styles.get("backgroundImage", "none") != "none":
            has_gradient += 1
        if styles.get("boxShadow", "none") != "none":
            has_shadow += 1
        if styles.get("transform", "none") != "none":
            has_transform += 1
        if el.get("animation"):
            has_animation += 1
        if el.get("transition"):
            has_transition += 1
        br = styles.get("borderTopLeftRadius", "0px")
        if br and br != "0px":
            has_border_radius += 1
        ov = styles.get("overflow", "visible")
        if ov == "hidden":
            has_overflow_hidden += 1
        if "grid" in d:
            has_grid += 1
        if d == "flex":
            has_flex += 1

    return {
        "file": truth_path.name,
        "siteName": gt.get("siteName", truth_path.stem),
        "url": gt.get("url", ""),
        "viewport": gt.get("viewportLabel", ""),
        "elementCount": gt["elementCount"],
        "pageScroll": gt.get("pageScroll", {}),
        "scrollContainers": len(gt.get("scrollContainers", [])),
        "fixedSticky": len(gt.get("fixedStickyElements", [])),
        "images": len(gt.get("images", [])),
        "interactive": len(gt.get("interactiveElements", [])),
        "textBlocks": len(gt.get("textElements", [])),
        "stackingContexts": len(gt.get("stackingContexts", [])),
        "scrollBehaviorPositions": len(gt.get("scrollBehavior", [])),
        "cssPropertyCount": len(prop_usage),
        "totalStyleValues": total_styles,
        "avgStylesPerElement": round(total_styles / max(gt["elementCount"], 1), 1),
        "displayBreakdown": dict(display_values.most_common(10)),
        "positionBreakdown": dict(position_values.most_common(10)),
        "featureCounts": {
            "flex": has_flex,
            "grid": has_grid,
            "gradient": has_gradient,
            "shadow": has_shadow,
            "transform": has_transform,
            "animation": has_animation,
            "transition": has_transition,
            "borderRadius": has_border_radius,
            "overflowHidden": has_overflow_hidden,
        },
        "topProperties": dict(prop_usage.most_common(30)),
        "uniqueValuesPerProp": {
            prop: sorted(list(vals))[:10]
            for prop, vals in sorted(prop_values.items())
            if len(vals) > 1
        },
    }


def main():
    truth_files = sorted(TRUTH_DIR.glob("*.json"))
    if not truth_files:
        print("No ground truth files found.", file=sys.stderr)
        return 1

    # Group by site (desktop viewport only for the summary)
    sites = {}
    all_analyses = []

    for tf in truth_files:
        analysis = analyze_site(tf)
        all_analyses.append(analysis)

        site_name = tf.stem.rsplit("-", 1)[0]  # "gumroad-desktop" -> "gumroad"
        viewport = tf.stem.rsplit("-", 1)[1] if "-" in tf.stem else "unknown"

        if site_name not in sites:
            sites[site_name] = {}
        sites[site_name][viewport] = analysis

    # Aggregate stats across all desktop captures
    desktop_analyses = [a for a in all_analyses if a["viewport"] == "desktop"]

    total_elements = sum(a["elementCount"] for a in desktop_analyses)
    total_scroll_containers = sum(a["scrollContainers"] for a in desktop_analyses)
    total_fixed_sticky = sum(a["fixedSticky"] for a in desktop_analyses)
    total_images = sum(a["images"] for a in desktop_analyses)
    total_interactive = sum(a["interactive"] for a in desktop_analyses)
    total_text_blocks = sum(a["textBlocks"] for a in desktop_analyses)
    total_stacking = sum(a["stackingContexts"] for a in desktop_analyses)

    # CSS feature coverage across all sites
    all_features = Counter()
    for a in desktop_analyses:
        for feat, count in a["featureCounts"].items():
            if count > 0:
                all_features[feat] += 1  # count sites that use this feature

    # Display type coverage
    all_display = Counter()
    for a in desktop_analyses:
        for val, count in a["displayBreakdown"].items():
            all_display[val] += count

    dashboard = {
        "generatedAt": __import__("time").strftime("%Y-%m-%dT%H:%M:%SZ", __import__("time").gmtime()),
        "corpusSummary": {
            "siteCount": len(sites),
            "viewportsPerSite": len(VIEWPORTS) if "VIEWPORTS" in dir() else 3,
            "totalGroundTruthFiles": len(truth_files),
            "totalElements": total_elements,
            "totalScrollContainers": total_scroll_containers,
            "totalFixedSticky": total_fixed_sticky,
            "totalImages": total_images,
            "totalInteractive": total_interactive,
            "totalTextBlocks": total_text_blocks,
            "totalStackingContexts": total_stacking,
        },
        "featureCoverageAcrossSites": dict(all_features.most_common()),
        "displayTypeCoverage": dict(all_display.most_common(15)),
        "sites": {
            site_name: {
                viewport: {
                    "elements": data["elementCount"],
                    "scrollHeight": data["pageScroll"].get("scrollHeight", 0),
                    "features": data["featureCounts"],
                    "scrollContainers": data["scrollContainers"],
                    "fixedSticky": data["fixedSticky"],
                }
                for viewport, data in viewports.items()
            }
            for site_name, viewports in sites.items()
        },
        "perSiteDetail": {
            name: sites[name].get("desktop", {})
            for name in sites
        },
    }

    out_path = REPO / "corpus" / "dashboard.json"
    out_path.write_text(json.dumps(dashboard, indent=2))
    print(f"Dashboard: {out_path}")
    print()
    print(f"Sites: {len(sites)}")
    print(f"Ground truth files: {len(truth_files)}")
    print(f"Total elements (desktop): {total_elements}")
    print(f"Total interactive elements: {total_interactive}")
    print(f"Total scroll containers: {total_scroll_containers}")
    print(f"Total fixed/sticky: {total_fixed_sticky}")
    print(f"Total stacking contexts: {total_stacking}")
    print()
    print("Feature coverage (sites using each):")
    for feat, count in all_features.most_common():
        print(f"  {feat}: {count}/{len(sites)} sites")
    print()
    print("Display types across corpus:")
    for val, count in all_display.most_common(10):
        print(f"  {val}: {count} elements")

    return 0


if __name__ == "__main__":
    sys.exit(main())
