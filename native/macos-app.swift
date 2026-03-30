#!/usr/bin/env swift
//
//  macos-app.swift
//  Rendero macOS native app — single file, no Xcode.
//
//  Build & run:
//    cd /path/to/rendero
//    cargo build --release -p rendero-native-ffi
//    swiftc native/macos-app.swift \
//      -L target/release -lrendero_native_ffi \
//      -framework AppKit -framework JavaScriptCore \
//      -o native/RenderoApp && native/RenderoApp
//

import AppKit
import JavaScriptCore

// ─── C FFI imports ───

@_silgen_name("rendero_create")
func rendero_create(_ name: UnsafePointer<CChar>, _ clientId: UInt32) -> UnsafeMutableRawPointer

@_silgen_name("rendero_destroy")
func rendero_destroy(_ engine: UnsafeMutableRawPointer)

@_silgen_name("rendero_set_viewport")
func rendero_set_viewport(_ engine: UnsafeMutableRawPointer, _ w: UInt32, _ h: UInt32)

@_silgen_name("rendero_set_camera")
func rendero_set_camera(_ engine: UnsafeMutableRawPointer, _ x: Float, _ y: Float, _ zoom: Float)

@_silgen_name("rendero_get_camera")
func rendero_get_camera(_ engine: UnsafeMutableRawPointer, _ out: UnsafeMutablePointer<Float>)

@_silgen_name("rendero_add_frame")
func rendero_add_frame(_ engine: UnsafeMutableRawPointer, _ name: UnsafePointer<CChar>,
                       _ x: Float, _ y: Float, _ w: Float, _ h: Float,
                       _ r: Float, _ g: Float, _ b: Float, _ a: Float) -> UInt64

@_silgen_name("rendero_add_text")
func rendero_add_text(_ engine: UnsafeMutableRawPointer, _ name: UnsafePointer<CChar>,
                      _ text: UnsafePointer<CChar>,
                      _ x: Float, _ y: Float, _ fontSize: Float,
                      _ r: Float, _ g: Float, _ b: Float, _ a: Float) -> UInt64

@_silgen_name("rendero_set_insert_parent")
func rendero_set_insert_parent(_ engine: UnsafeMutableRawPointer, _ counter: UInt32, _ clientId: UInt32)

@_silgen_name("rendero_clear_insert_parent")
func rendero_clear_insert_parent(_ engine: UnsafeMutableRawPointer)

@_silgen_name("rendero_set_node_position")
func rendero_set_node_position(_ engine: UnsafeMutableRawPointer, _ counter: UInt32, _ clientId: UInt32, _ x: Float, _ y: Float)

@_silgen_name("rendero_set_node_size")
func rendero_set_node_size(_ engine: UnsafeMutableRawPointer, _ counter: UInt32, _ clientId: UInt32, _ w: Float, _ h: Float)

@_silgen_name("rendero_set_node_fill")
func rendero_set_node_fill(_ engine: UnsafeMutableRawPointer, _ counter: UInt32, _ clientId: UInt32,
                           _ r: Float, _ g: Float, _ b: Float, _ a: Float)

@_silgen_name("rendero_set_node_corner_radius")
func rendero_set_node_corner_radius(_ engine: UnsafeMutableRawPointer, _ counter: UInt32, _ clientId: UInt32,
                                    _ tl: Float, _ tr: Float, _ br: Float, _ bl: Float)

@_silgen_name("rendero_set_node_opacity")
func rendero_set_node_opacity(_ engine: UnsafeMutableRawPointer, _ counter: UInt32, _ clientId: UInt32, _ opacity: Float)

@_silgen_name("rendero_set_node_text")
func rendero_set_node_text(_ engine: UnsafeMutableRawPointer, _ counter: UInt32, _ clientId: UInt32, _ text: UnsafePointer<CChar>)

@_silgen_name("rendero_set_node_font_size")
func rendero_set_node_font_size(_ engine: UnsafeMutableRawPointer, _ counter: UInt32, _ clientId: UInt32, _ size: Float)

@_silgen_name("rendero_set_node_font_weight")
func rendero_set_node_font_weight(_ engine: UnsafeMutableRawPointer, _ counter: UInt32, _ clientId: UInt32, _ weight: UInt16)

@_silgen_name("rendero_set_auto_layout")
func rendero_set_auto_layout(_ engine: UnsafeMutableRawPointer, _ counter: UInt32, _ clientId: UInt32,
                             _ direction: UInt32, _ spacing: Float,
                             _ padTop: Float, _ padRight: Float, _ padBottom: Float, _ padLeft: Float)

@_silgen_name("rendero_select_node")
func rendero_select_node(_ engine: UnsafeMutableRawPointer, _ counter: UInt32, _ clientId: UInt32)

@_silgen_name("rendero_delete_selected")
func rendero_delete_selected(_ engine: UnsafeMutableRawPointer)

@_silgen_name("rendero_get_node_bounds")
func rendero_get_node_bounds(_ engine: UnsafeMutableRawPointer, _ counter: UInt32, _ clientId: UInt32,
                             _ outX: UnsafeMutablePointer<Float>, _ outY: UnsafeMutablePointer<Float>,
                             _ outW: UnsafeMutablePointer<Float>, _ outH: UnsafeMutablePointer<Float>)

@_silgen_name("rendero_render_pixels")
func rendero_render_pixels(_ engine: UnsafeMutableRawPointer, _ buffer: UnsafeMutablePointer<UInt8>, _ width: UInt32, _ height: UInt32)


// ═══════════════════════════════════════════════════════════════
// RenderoView — NSView that renders via Rust engine + JSC
// ═══════════════════════════════════════════════════════════════

class RenderoView: NSView {
    private var engine: UnsafeMutableRawPointer!
    private var jsContext: JSContext!
    private var timer: Timer?
    private var pixelBuffer: [UInt8] = []
    private var bufferWidth: UInt32 = 0
    private var bufferHeight: UInt32 = 0
    private var frameCount = 0

    func setup() {
        wantsLayer = true
        layer?.backgroundColor = NSColor.black.cgColor

        appLog("[Rendero] Creating native engine...")
        engine = "CodexNative".withCString { rendero_create($0, 1) }
        appLog("[Rendero] Engine created: \(engine!)")

        // Quick smoke test — create a frame directly without JS
        // Set up JSContext
        appLog("[Rendero] Setting up JavaScriptCore...")
        jsContext = JSContext()!
        jsContext.exceptionHandler = { _, exception in
            appLog("[JSC ERROR] \(exception?.toString() ?? "unknown")")
        }
        registerJSFunctions()

        // Load the JS bundle
        loadBundle()

        // CRITICAL: Drain pending callbacks immediately after bundle eval.
        // React's scheduler posts the commit via MessageChannel/setTimeout.
        // The commit creates the DOM tree and appends to container.
        // Without draining, the commit never fires.
        for _ in 0..<10 {
            jsContext?.evaluateScript("""
                var __b = __pendingCallbacks.splice(0);
                for (var i = 0; i < __b.length; i++) {
                    try { __b[i](); } catch(e) { console.error('init callback: ' + e.message); }
                }
            """)
        }
        appLog("[Rendero] Post-init callback drain complete")

        // Start render timer
        timer = Timer.scheduledTimer(withTimeInterval: 1.0/60.0, repeats: true) { [weak self] _ in
            self?.renderFrame()
        }

        appLog("[Rendero] Setup complete. Rendering at 60fps.")
    }

    private func registerJSFunctions() {
        let eng = engine!

        // console.log
        let log: @convention(block) (String) -> Void = { msg in
            appLog("[JS] \(msg)")
        }
        let consoleObj = JSValue(newObjectIn: jsContext)!
        consoleObj.setObject(log, forKeyedSubscript: "log" as NSString)
        consoleObj.setObject(log, forKeyedSubscript: "warn" as NSString)
        consoleObj.setObject(log, forKeyedSubscript: "error" as NSString)
        jsContext.setObject(consoleObj, forKeyedSubscript: "console" as NSString)

        // __rendero_log
        jsContext.setObject(log, forKeyedSubscript: "__rendero_log" as NSString)

        // performance.now
        let perfNow: @convention(block) () -> Double = {
            ProcessInfo.processInfo.systemUptime * 1000
        }
        let perfObj = JSValue(newObjectIn: jsContext)!
        perfObj.setObject(perfNow, forKeyedSubscript: "now" as NSString)
        jsContext.setObject(perfObj, forKeyedSubscript: "performance" as NSString)

        // Screen info
        let screen = NSScreen.main ?? NSScreen.screens[0]
        jsContext.setObject(Int(screen.frame.width), forKeyedSubscript: "__screenWidth" as NSString)
        jsContext.setObject(Int(screen.frame.height), forKeyedSubscript: "__screenHeight" as NSString)
        jsContext.setObject(screen.backingScaleFactor, forKeyedSubscript: "__screenScale" as NSString)

        // setTimeout / queueMicrotask
        // Timer queue — store callbacks in JS, drain each frame from Swift
        jsContext.evaluateScript("""
            var __timerQueue = [];
            var __nextTimerId = 1;
        """)

        // setTimeout/queueMicrotask — store callbacks in JS array, drained each frame
        jsContext.evaluateScript("""
            var __pendingCallbacks = [];
            function setTimeout(fn, delay) {
                __pendingCallbacks.push(fn);
                return __pendingCallbacks.length;
            }
            function clearTimeout(id) {}
            function setInterval(fn, ms) { return setTimeout(fn, ms); }
            function clearInterval(id) {}
            function queueMicrotask(fn) { __pendingCallbacks.push(fn); }
        """)

        // Browser globals that React and the DOM shim expect
        jsContext.evaluateScript("""
            var window = this;
            var self = this;
            var global = this;
            var globalThis = this;
            var document = {
                createElement: function(tag) {
                    return {
                        tagName: tag.toUpperCase(),
                        style: {},
                        childNodes: [],
                        setAttribute: function() {},
                        getAttribute: function() { return null; },
                        removeAttribute: function() {},
                        addEventListener: function() {},
                        removeEventListener: function() {},
                        appendChild: function(c) { this.childNodes.push(c); return c; },
                        removeChild: function() {},
                        insertBefore: function() {},
                        contains: function() { return false; },
                        cloneNode: function() { return document.createElement(tag); },
                        nodeType: 1,
                        parentNode: null,
                        firstChild: null,
                        lastChild: null,
                        nextSibling: null,
                        ownerDocument: document
                    };
                },
                createTextNode: function(t) { return { nodeType: 3, textContent: t, parentNode: null }; },
                createComment: function() { return { nodeType: 8 }; },
                createDocumentFragment: function() { return { nodeType: 11, childNodes: [], appendChild: function(c) { this.childNodes.push(c); return c; } }; },
                createEvent: function() { return { initEvent: function() {} }; },
                body: null,
                documentElement: null,
                activeElement: null,
                defaultView: null,
                addEventListener: function() {},
                removeEventListener: function() {},
                getElementById: function() { return null; },
                querySelector: function() { return null; },
                querySelectorAll: function() { return []; },
                implementation: { hasFeature: function() { return true; } }
            };
            document.body = document.createElement('body');
            document.documentElement = document.createElement('html');
            document.documentElement.appendChild(document.body);
            document.defaultView = window;
            document.activeElement = document.body;
            var navigator = { userAgent: 'CodexNative/1.0', platform: 'macOS' };
            var HTMLElement = function HTMLElement() {};
            var HTMLIFrameElement = function HTMLIFrameElement() {};
            var HTMLInputElement = function HTMLInputElement() {};
            var HTMLTextAreaElement = function HTMLTextAreaElement() {};
            var HTMLSelectElement = function HTMLSelectElement() {};
            var Node = function Node() {};
            var Event = function Event(type) { this.type = type; };
            var CustomEvent = Event;
            var MutationObserver = function(cb) { this.observe = function() {}; this.disconnect = function() {}; };
            var requestAnimationFrame = function(cb) { setTimeout(cb, 16); return 0; };
            var cancelAnimationFrame = function() {};
            var setInterval = function(cb, ms) { return setTimeout(cb, ms); };
            var clearInterval = function() {};
        """)

        // MessageChannel polyfill for React scheduler
        // CRITICAL: React's scheduler uses MessageChannel.postMessage to schedule work.
        // Must use setTimeout(0) NOT queueMicrotask — queueMicrotask routes through
        // DispatchQueue.main.async which doesn't fire during Timer callbacks.
        jsContext.evaluateScript("""
            function MessageChannel() {
                this.port1 = { onmessage: null };
                var self = this;
                this.port2 = { postMessage: function() {
                    if (self.port1 && self.port1.onmessage) {
                        var cb = self.port1.onmessage;
                        setTimeout(function() { cb({ data: undefined }); }, 0);
                    }
                }};
            }
        """)

        // ─── Rendero engine functions ───

        let setViewport: @convention(block) (UInt32, UInt32) -> Void = { w, h in
            rendero_set_viewport(eng, w, h)
        }
        jsContext.setObject(setViewport, forKeyedSubscript: "__rendero_set_viewport" as NSString)

        let setCamera: @convention(block) (Float, Float, Float) -> Void = { x, y, zoom in
            rendero_set_camera(eng, x, y, zoom)
        }
        jsContext.setObject(setCamera, forKeyedSubscript: "__rendero_set_camera" as NSString)

        let getCamera: @convention(block) () -> [String: Float] = {
            var cam: [Float] = [0, 0, 1]
            rendero_get_camera(eng, &cam)
            return ["x": cam[0], "y": cam[1], "zoom": cam[2]]
        }
        jsContext.setObject(getCamera, forKeyedSubscript: "__rendero_get_camera" as NSString)

        let setParent: @convention(block) (UInt32, UInt32) -> Void = { c, ci in
            rendero_set_insert_parent(eng, c, ci)
        }
        jsContext.setObject(setParent, forKeyedSubscript: "__rendero_set_insert_parent" as NSString)

        let clearParent: @convention(block) () -> Void = {
            rendero_clear_insert_parent(eng)
        }
        jsContext.setObject(clearParent, forKeyedSubscript: "__rendero_clear_insert_parent" as NSString)

        let addFrame: @convention(block) (String, Float, Float, Float, Float, Float, Float, Float, Float) -> Double = {
            name, x, y, w, h, r, g, b, a in
            let packed = name.withCString { rendero_add_frame(eng, $0, x, y, w, h, r, g, b, a) }
            return Double(packed)
        }
        jsContext.setObject(addFrame, forKeyedSubscript: "__rendero_add_frame" as NSString)

        let addText: @convention(block) (String, String, Float, Float, Float, Float, Float, Float, Float) -> Double = {
            name, text, x, y, fontSize, r, g, b, a in
            let packed = name.withCString { cname in
                text.withCString { ctext in
                    rendero_add_text(eng, cname, ctext, x, y, fontSize, r, g, b, a)
                }
            }
            return Double(packed)
        }
        jsContext.setObject(addText, forKeyedSubscript: "__rendero_add_text" as NSString)

        let setPos: @convention(block) (UInt32, UInt32, Float, Float) -> Void = { c, ci, x, y in
            rendero_set_node_position(eng, c, ci, x, y)
        }
        jsContext.setObject(setPos, forKeyedSubscript: "__rendero_set_node_position" as NSString)

        let setSize: @convention(block) (UInt32, UInt32, Float, Float) -> Void = { c, ci, w, h in
            rendero_set_node_size(eng, c, ci, w, h)
        }
        jsContext.setObject(setSize, forKeyedSubscript: "__rendero_set_node_size" as NSString)

        let setFill: @convention(block) (UInt32, UInt32, Float, Float, Float, Float) -> Void = { c, ci, r, g, b, a in
            rendero_set_node_fill(eng, c, ci, r, g, b, a)
        }
        jsContext.setObject(setFill, forKeyedSubscript: "__rendero_set_node_fill" as NSString)

        let setRadius: @convention(block) (UInt32, UInt32, Float, Float, Float, Float) -> Void = { c, ci, tl, tr, br, bl in
            rendero_set_node_corner_radius(eng, c, ci, tl, tr, br, bl)
        }
        jsContext.setObject(setRadius, forKeyedSubscript: "__rendero_set_node_corner_radius" as NSString)

        let setOpacity: @convention(block) (UInt32, UInt32, Float) -> Void = { c, ci, o in
            rendero_set_node_opacity(eng, c, ci, o)
        }
        jsContext.setObject(setOpacity, forKeyedSubscript: "__rendero_set_node_opacity" as NSString)

        let setTextFn: @convention(block) (UInt32, UInt32, String) -> Void = { c, ci, text in
            text.withCString { rendero_set_node_text(eng, c, ci, $0) }
        }
        jsContext.setObject(setTextFn, forKeyedSubscript: "__rendero_set_node_text" as NSString)

        let setFontSize: @convention(block) (UInt32, UInt32, Float) -> Void = { c, ci, s in
            rendero_set_node_font_size(eng, c, ci, s)
        }
        jsContext.setObject(setFontSize, forKeyedSubscript: "__rendero_set_node_font_size" as NSString)

        let setFontWeight: @convention(block) (UInt32, UInt32, UInt16) -> Void = { c, ci, w in
            rendero_set_node_font_weight(eng, c, ci, w)
        }
        jsContext.setObject(setFontWeight, forKeyedSubscript: "__rendero_set_node_font_weight" as NSString)

        let setLayout: @convention(block) (UInt32, UInt32, UInt32, Float, Float, Float, Float, Float) -> Void = {
            c, ci, dir, spacing, pt, pr, pb, pl in
            rendero_set_auto_layout(eng, c, ci, dir, spacing, pt, pr, pb, pl)
        }
        jsContext.setObject(setLayout, forKeyedSubscript: "__rendero_set_auto_layout" as NSString)

        let selectNode: @convention(block) (UInt32, UInt32) -> Void = { c, ci in
            rendero_select_node(eng, c, ci)
        }
        jsContext.setObject(selectNode, forKeyedSubscript: "__rendero_select_node" as NSString)

        let deleteSelected: @convention(block) () -> Void = {
            rendero_delete_selected(eng)
        }
        jsContext.setObject(deleteSelected, forKeyedSubscript: "__rendero_delete_selected" as NSString)

        let getBounds: @convention(block) (UInt32, UInt32) -> [String: Float] = { c, ci in
            var x: Float = 0, y: Float = 0, w: Float = 0, h: Float = 0
            rendero_get_node_bounds(eng, c, ci, &x, &y, &w, &h)
            return ["x": x, "y": y, "w": w, "h": h]
        }
        jsContext.setObject(getBounds, forKeyedSubscript: "__rendero_get_node_bounds" as NSString)

        let requestRender: @convention(block) () -> Void = { /* timer handles rendering */ }
        jsContext.setObject(requestRender, forKeyedSubscript: "__rendero_request_render" as NSString)
    }

    private func loadBundle() {
        // Look for macos-bundle.js next to the executable
        let execDir = (CommandLine.arguments[0] as NSString).deletingLastPathComponent
        let candidates = [
            "\(execDir)/macos-bundle.js",
            "\(execDir)/../docs/demos/dom-shim/dist/macos-bundle.js",
            // Absolute fallback
            "/Users/kumardivyarajat/WebstormProjects/rendero/docs/demos/dom-shim/dist/macos-bundle.js",
        ]

        for path in candidates {
            if FileManager.default.fileExists(atPath: path) {
                do {
                    let js = try String(contentsOfFile: path, encoding: .utf8)
                    appLog("[Rendero] Loading JS bundle from \(path) (\(js.count) chars)")
                    jsContext.evaluateScript(js)
                    appLog("[Rendero] JS bundle executed successfully")
                    return
                } catch {
                    appLog("[Rendero] Error reading \(path): \(error)")
                }
            }
        }
        appLog("[Rendero] WARNING: macos-bundle.js not found!")
    }

    func renderFrame() {
        // Drain pending JS callbacks (setTimeout/queueMicrotask from React scheduler)
        // This is the key: React's scheduler posts work via setTimeout(fn, 0).
        // We stored those callbacks in __pendingCallbacks and drain them here.
        jsContext?.evaluateScript("""
            var __batch = __pendingCallbacks.splice(0);
            for (var i = 0; i < __batch.length; i++) {
                try { __batch[i](); } catch(e) { console.error('callback error: ' + e.message); }
            }
        """)
        // Drain again — callbacks may have scheduled more callbacks
        jsContext?.evaluateScript("""
            var __batch2 = __pendingCallbacks.splice(0);
            for (var i = 0; i < __batch2.length; i++) {
                try { __batch2[i](); } catch(e) {}
            }
        """)
        // And once more for good measure (React's scheduler can chain 3 deep)
        jsContext?.evaluateScript("""
            var __batch3 = __pendingCallbacks.splice(0);
            for (var i = 0; i < __batch3.length; i++) {
                try { __batch3[i](); } catch(e) {}
            }
        """)

        // Flush DOM shim ops to engine
        jsContext?.evaluateScript("if (typeof __shimFlushAndRender === 'function') __shimFlushAndRender();")

        let scale = window?.backingScaleFactor ?? 2.0
        let w = UInt32(bounds.width * scale)
        let h = UInt32(bounds.height * scale)
        if w == 0 || h == 0 { return }

        if w != bufferWidth || h != bufferHeight {
            bufferWidth = w
            bufferHeight = h
            pixelBuffer = [UInt8](repeating: 0, count: Int(w * h * 4))
            rendero_set_viewport(engine, w, h)
            rendero_set_camera(engine, 0, 0, Float(scale))
            appLog("[Rendero] Viewport: \(w)x\(h) @\(scale)x")
        }

        rendero_render_pixels(engine, &pixelBuffer, w, h)

        // Blit RGBA pixels to CALayer
        let colorSpace = CGColorSpaceCreateDeviceRGB()
        if let provider = CGDataProvider(data: Data(pixelBuffer) as CFData),
           let image = CGImage(
            width: Int(w), height: Int(h),
            bitsPerComponent: 8, bitsPerPixel: 32,
            bytesPerRow: Int(w) * 4,
            space: colorSpace,
            bitmapInfo: CGBitmapInfo(rawValue: CGImageAlphaInfo.premultipliedLast.rawValue),
            provider: provider,
            decode: nil, shouldInterpolate: false, intent: .defaultIntent
           ) {
            layer?.contents = image
            if frameCount == 0 {
                appLog("[Rendero] First frame rendered: \(w)x\(h)")
            }
            frameCount += 1
        }
    }

    override func layout() {
        super.layout()
        // Reset buffer on resize
        bufferWidth = 0
        bufferHeight = 0
    }

    deinit {
        timer?.invalidate()
        if engine != nil { rendero_destroy(engine) }
    }
}


// ═══════════════════════════════════════════════════════════════
// App — minimal AppKit app, no Xcode needed
// ═══════════════════════════════════════════════════════════════

// File logger — Swift print() doesn't flush to piped stdout
let logFile = fopen("/tmp/rendero-app.log", "w")
func appLog(_ msg: String) {
    print(msg)
    if let f = logFile {
        fputs(msg + "\n", f)
        fflush(f)
    }
}

class AppDelegate: NSObject, NSApplicationDelegate {
    var window: NSWindow!

    func applicationDidFinishLaunching(_ notification: Notification) {
        let rect = NSRect(x: 100, y: 100, width: 1024, height: 768)
        window = NSWindow(
            contentRect: rect,
            styleMask: [.titled, .closable, .resizable, .miniaturizable],
            backing: .buffered,
            defer: false
        )
        window.title = "Codex Version — Rendero Native Runtime"
        window.minSize = NSSize(width: 400, height: 300)

        let view = RenderoView(frame: rect)
        view.setup()
        window.contentView = view
        window.makeKeyAndOrderFront(nil)

        appLog("[Rendero] Window open. Pipeline: DOM Shim → Rust Engine → Pixels (Codex Version)")
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
        return true
    }
}

// Launch
let app = NSApplication.shared
let delegate = AppDelegate()
app.delegate = delegate
app.run()
