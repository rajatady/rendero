#!/usr/bin/env python3
from __future__ import annotations

import json
import sys
import time
from pathlib import Path

from playwright.sync_api import sync_playwright


def main() -> int:
    if len(sys.argv) != 3:
        print("usage: capture_dom_shim.py <base_url> <out_dir>", file=sys.stderr)
        return 2

    base_url = sys.argv[1].rstrip("/")
    out_dir = Path(sys.argv[2]).resolve()
    out_dir.mkdir(parents=True, exist_ok=True)

    console_events: list[dict[str, str]] = []

    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True)
        page = browser.new_page(viewport={"width": 1440, "height": 900}, device_scale_factor=1)

        def on_console(msg) -> None:
            try:
                text = msg.text
            except Exception:
                text = "<unavailable>"
            console_events.append({"type": msg.type, "text": text})

        page.on("console", on_console)

        page.goto(f"{base_url}/demos/dom-shim/?v={int(time.time() * 1000)}", wait_until="domcontentloaded")
        page.screenshot(path=str(out_dir / "react-web.png"), full_page=True)

        page.get_by_role("button", name="React (Rendero)").click()
        page.wait_for_timeout(3000)
        page.screenshot(path=str(out_dir / "react-rendero.png"), full_page=False)

        state = page.evaluate(
            """() => ({
                title: document.title,
                label: document.getElementById('mode-label')?.textContent || null,
                bodyClass: document.body.className,
                canvas: (() => {
                    const c = document.getElementById('canvas');
                    return c ? {
                        width: c.width,
                        height: c.height,
                        display: getComputedStyle(c).display
                    } : null;
                })()
            })"""
        )
        (out_dir / "browser-state.json").write_text(json.dumps(state, indent=2))
        (out_dir / "browser-console.json").write_text(json.dumps(console_events, indent=2))
        browser.close()

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
