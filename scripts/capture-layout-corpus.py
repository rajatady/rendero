#!/usr/bin/env python3
"""
Capture the synthetic layout corpus benchmark report.

Loads docs/demos/dom-shim/accuracy/layout-accuracy.html, waits for the browser
vs Rendero comparison to finish, then saves the report JSON and screenshot.
"""

import json
import sys
import time
from pathlib import Path

try:
    from playwright.sync_api import sync_playwright
except ImportError:
    print("Install playwright: pip install playwright && playwright install chromium", file=sys.stderr)
    sys.exit(1)


def main():
    if len(sys.argv) < 3:
        print(f"Usage: {sys.argv[0]} <base-url> <output.json>", file=sys.stderr)
        return 1

    base_url = sys.argv[1].rstrip("/")
    output_path = Path(sys.argv[2]).resolve()
    output_path.parent.mkdir(parents=True, exist_ok=True)
    url = f"{base_url}/demos/dom-shim/accuracy/layout-accuracy.html?report=1&v={int(time.time() * 1000)}"

    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True)
        page = browser.new_page(viewport={"width": 1440, "height": 1800}, device_scale_factor=1)
        page.goto(url, wait_until="networkidle")
        page.wait_for_function("window.__LAYOUT_ACCURACY_REPORT__ && window.__LAYOUT_ACCURACY_REPORT__.status === 'ready'")
        report = page.evaluate("window.__LAYOUT_ACCURACY_REPORT__")
        output_path.write_text(json.dumps(report, indent=2))
        screenshot_path = output_path.with_suffix(".png")
        page.screenshot(path=str(screenshot_path), full_page=True)
        print(f"[Layout Corpus] Saved report to {output_path}")
        print(f"[Layout Corpus] Screenshot: {screenshot_path}")
        property_matches = report.get("propertyMatches", report.get("matchCount", 0))
        total_properties = report.get("totalProperties", report.get("total", 0))
        print(f"[Layout Corpus] Property accuracy: {property_matches}/{total_properties}")
        browser.close()

    return 0


if __name__ == "__main__":
    sys.exit(main())
