#!/usr/bin/env python3
"""
Capture layout results from the Rendero engine (WASM canvas mode).

Loads the demo page, switches to React (Rendero) mode, extracts
engine node bounds, saves as JSON for comparison with browser ground truth.

Usage:
    python3 scripts/capture-engine-truth.py <url> <output.json>
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


ENGINE_EXTRACTOR_JS = """
() => {
    // The engine exposes node bounds through the Rendero namespace
    const debug = window.__RENDERO_DEBUG__;
    if (!debug || !debug.engine) {
        return { error: 'Rendero engine not found. Is the page in Rendero mode?' };
    }

    const engine = debug.engine;
    const shimDoc = debug.shimDocument;
    const debugSnapshot =
        window.__RENDERO_DEBUG__?.getLayeredState?.()
        || window.__RENDERO_DEBUG__?.layered
        || window.__RENDERO_DEBUG_STATE__
        || window.Rendero?.debug?.getSnapshot?.()
        || {};
    if (!shimDoc) {
        return { error: 'Shim document not found.' };
    }

    const elements = [];
    const engineTextNodes = [];
    let nextId = 0;

    function idKey(value) {
        if (!value || typeof value !== 'object') return null;
        const counter = value.counter ?? value.Counter ?? value.id?.counter;
        const clientId = value.client_id ?? value.clientId ?? value.client_id;
        if (counter == null || clientId == null) return null;
        return `${counter}:${clientId}`;
    }

    function kindName(node) {
        const kind = node?.kind;
        if (!kind || typeof kind !== 'object') return 'Unknown';
        const keys = Object.keys(kind);
        return keys.length > 0 ? keys[0] : 'Unknown';
    }

    function summarizeNode(node) {
        const kind = kindName(node);
        const summary = {
            name: node?.name || null,
            kind,
            width: node?.width || 0,
            height: node?.height || 0,
            transform: {
                tx: node?.transform?.tx || 0,
                ty: node?.transform?.ty || 0,
            },
            sizing: {
                horizontal: node?.horizontal_sizing || null,
                vertical: node?.vertical_sizing || null,
            },
            margin: node?.margin || null,
            layoutPosition: node?.layout_position || null,
            sizeConstraints: node?.size_constraints || null,
            autoLayout: node?.kind?.Frame?.auto_layout || null,
            fills: node?.style?.fills || [],
            opacity: node?.style?.opacity ?? 1,
        };
        if (kind === 'Text') {
            const runs = node?.kind?.Text?.runs || [];
            summary.text = {
                text: runs.map((run) => run.text || '').join(''),
                runs: runs.map((run) => ({
                    text: run.text || '',
                    fontFamily: run.font_family || null,
                    fontSize: run.font_size || null,
                    fontWeight: run.font_weight || null,
                    lineHeight: run.line_height ?? null,
                    letterSpacing: run.letter_spacing || 0,
                })),
                align: node?.kind?.Text?.align || null,
            };
        }
        return summary;
    }

    function extractEnginePipeline() {
        const json = engine.export_document_json?.();
        if (!json) {
            return { engineDocument: null, engineModel: [], engineLayout: [] };
        }

        let engineDocument = null;
        try {
            engineDocument = JSON.parse(json);
        } catch (e) {
            return {
                engineDocument: { error: String(e) },
                engineModel: [],
                engineLayout: [],
            };
        }

        const flatNodes = engineDocument?.pages?.[0]?.tree?.nodes || [];
        const registry = debugSnapshot.nodes || {};
        const nodeKeyToEngineId = {};
        for (const [engineId, ids] of Object.entries(registry)) {
            const key = `${ids.counter}:${ids.clientId}`;
            nodeKeyToEngineId[key] = Number(engineId);
        }

        const entries = flatNodes.map(([node, parentId]) => {
            const key = idKey(node?.id);
            return {
                key,
                parentKey: idKey(parentId),
                engineId: key ? (nodeKeyToEngineId[key] ?? null) : null,
                node,
            };
        });

        const byKey = new Map(entries.filter((entry) => entry.key).map((entry) => [entry.key, entry]));
        const worldCache = new Map();

        function worldFor(entry) {
            if (!entry?.key) return { x: 0, y: 0 };
            if (worldCache.has(entry.key)) return worldCache.get(entry.key);
            const local = {
                x: entry.node?.transform?.tx || 0,
                y: entry.node?.transform?.ty || 0,
            };
            if (!entry.parentKey) {
                worldCache.set(entry.key, local);
                return local;
            }
            const parent = byKey.get(entry.parentKey);
            const parentWorld = worldFor(parent);
            const result = {
                x: parentWorld.x + local.x,
                y: parentWorld.y + local.y,
            };
            worldCache.set(entry.key, result);
            return result;
        }

        const engineModel = entries.map((entry) => ({
            key: entry.key,
            parentKey: entry.parentKey,
            engineId: entry.engineId,
            ...summarizeNode(entry.node),
        }));

        const engineLayout = entries.map((entry) => {
            const world = worldFor(entry);
            return {
                key: entry.key,
                parentKey: entry.parentKey,
                engineId: entry.engineId,
                name: entry.node?.name || null,
                kind: kindName(entry.node),
                local: {
                    x: entry.node?.transform?.tx || 0,
                    y: entry.node?.transform?.ty || 0,
                },
                world: {
                    x: world.x,
                    y: world.y,
                },
                size: {
                    width: entry.node?.width || 0,
                    height: entry.node?.height || 0,
                },
                bounds: {
                    x: world.x,
                    y: world.y,
                    width: entry.node?.width || 0,
                    height: entry.node?.height || 0,
                },
            };
        });

        return { engineDocument, engineModel, engineLayout };
    }

    function collectEngineTextNodes() {
        try {
            const json = engine.export_document_json?.();
            if (!json) return;
            const doc = JSON.parse(json);

            function walk(value) {
                if (!value) return;
                if (Array.isArray(value)) {
                    for (const item of value) walk(item);
                    return;
                }
                if (typeof value !== 'object') return;

                const kind = value.kind;
                if (kind && kind.Text && Array.isArray(kind.Text.runs)) {
                    const runs = kind.Text.runs;
                    const text = runs.map((run) => run.text || '').join('');
                    engineTextNodes.push({
                        name: value.name || null,
                        width: value.width || 0,
                        height: value.height || 0,
                        text,
                        runs: runs.map((run) => ({
                            text: run.text || '',
                            font_family: run.font_family || null,
                            font_size: run.font_size || null,
                            font_weight: run.font_weight || null,
                            letter_spacing: run.letter_spacing || 0,
                            line_height: run.line_height ?? null,
                        })),
                    });
                }

                for (const child of Object.values(value)) {
                    walk(child);
                }
            }

            walk(doc);
        } catch (e) {
            engineTextNodes.push({ error: String(e) });
        }
    }

    function walkShimNode(node, depth, parentId) {
        const id = nextId++;

        // Get engine bounds if available
        let bounds = { x: 0, y: 0, width: 0, height: 0 };
        if (node._engineId != null) {
            try {
                // Try the Rendero namespace first
                const b = window.Rendero?.engine?.getNodeBounds?.(node._engineId);
                if (b) {
                    bounds = {
                        x: Math.round((b.x || 0) * 10) / 10,
                        y: Math.round((b.y || 0) * 10) / 10,
                        width: Math.round((b.width || 0) * 10) / 10,
                        height: Math.round((b.height || 0) * 10) / 10,
                    };
                }
            } catch (e) {
                // Ignore
            }
        }

        // Get style values that were set
        const styles = {};
        if (node.style && node.style._values) {
            const v = node.style._values;
            for (const [key, val] of Object.entries(v)) {
                if (val) styles[key] = String(val);
            }
        }

        elements.push({
            id,
            parentId,
            depth,
            tag: node.tagName ? node.tagName.toLowerCase() : '#text',
            engineId: node._engineId || null,
            text: node._isTextElement ? (node.textContent || '').slice(0, 200) : '',
            bounds,
            styles,
            shimStyle: node._engineId != null ? (debugSnapshot.styles?.[String(node._engineId)] || null) : null,
            bridgeOps: node._engineId != null ? (debugSnapshot.bridgeOps || []).filter((op) => op.engineId === node._engineId) : [],
            registry: node._engineId != null ? (debugSnapshot.nodes?.[String(node._engineId)] || null) : null,
            childCount: node.childNodes ? node.childNodes.length : 0,
        });

        // Walk children
        if (node.childNodes) {
            for (const child of node.childNodes) {
                if (child.nodeType === 1 || (child.tagName && child.tagName !== '#text')) {
                    walkShimNode(child, depth + 1, id);
                }
            }
        }
    }

    // Start from shim document body
    const body = shimDoc.body;
    if (body) {
        walkShimNode(body, 0, null);
    }
    collectEngineTextNodes();

    const pipeline = extractEnginePipeline();

    return {
        pipelineVersion: 1,
        url: location.href,
        title: document.title,
        timestamp: new Date().toISOString(),
        viewport: {
            width: window.innerWidth,
            height: window.innerHeight,
        },
        elementCount: elements.length,
        elements,
        engineTextNodes,
        debugSnapshot,
        engineDocument: pipeline.engineDocument,
        engineModel: pipeline.engineModel,
        engineLayout: pipeline.engineLayout,
    };
}
"""


def main():
    if len(sys.argv) < 3:
        print(f"Usage: {sys.argv[0]} <url> <output.json>", file=sys.stderr)
        return 1

    url = sys.argv[1]
    output_path = Path(sys.argv[2]).resolve()
    output_path.parent.mkdir(parents=True, exist_ok=True)

    viewport_width = int(sys.argv[3]) if len(sys.argv) > 3 else 1440
    viewport_height = int(sys.argv[4]) if len(sys.argv) > 4 else 900

    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True)
        page = browser.new_page(
            viewport={"width": viewport_width, "height": viewport_height},
            device_scale_factor=1,
        )

        cache_bust = int(time.time() * 1000)
        separator = "&" if "?" in url else "?"
        page.goto(f"{url}{separator}v={cache_bust}", wait_until="networkidle")
        page.wait_for_timeout(2000)

        # Switch to React (Rendero) mode by clicking the button
        try:
            rendero_btn = page.get_by_role("button", name="React (Rendero)")
            rendero_btn.click()
            page.wait_for_timeout(3000)
        except Exception as e:
            print(f"[Engine] Could not switch to Rendero mode: {e}", file=sys.stderr)

        # Take screenshot
        screenshot_path = output_path.with_suffix('.png')
        page.screenshot(path=str(screenshot_path), full_page=False)
        print(f"[Engine] Screenshot: {screenshot_path}")

        # Extract engine bounds
        report = page.evaluate(ENGINE_EXTRACTOR_JS)

        if 'error' in report:
            print(f"[Engine] Error: {report['error']}", file=sys.stderr)
            return 1

        output_path.write_text(json.dumps(report, indent=2))
        print(f"[Engine] {report['elementCount']} elements from Rendero engine")
        print(f"[Engine] Saved to {output_path}")

        browser.close()

    return 0


if __name__ == "__main__":
    sys.exit(main())
