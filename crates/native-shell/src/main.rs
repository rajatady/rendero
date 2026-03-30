//! Rendero native shell — pure Rust, cross-platform.
//!
//! Opens a window, runs React via QuickJS, renders via the Rendero engine.
//! `cargo run -p rendero-native-shell` — works on macOS, Windows, Linux.

mod providers;
mod quickjs_runtime;
mod engine_bridge;

use std::cell::RefCell;
use std::num::NonZeroU32;
use std::rc::Rc;

use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

use crate::providers::{JSRuntime, NoopNativeAPI};
use crate::quickjs_runtime::QuickJSRuntime;
use crate::engine_bridge::{Engine, register_engine_functions, BROWSER_POLYFILLS};

struct App {
    window: Option<Window>,
    surface: Option<softbuffer::Surface<Rc<Window>, Rc<Window>>>,
    window_rc: Option<Rc<Window>>,
    engine: Rc<RefCell<Engine>>,
    js: QuickJSRuntime,
    initialized: bool,
    frame_count: u64,
}

impl App {
    fn new() -> Self {
        let engine = Rc::new(RefCell::new(Engine::new("RenderoNative", 1)));
        let mut js = QuickJSRuntime::new();
        let native_api = NoopNativeAPI;

        // Register engine functions in JS context
        js.with_context(|ctx| {
            register_engine_functions(&ctx, &engine, &native_api);
        });

        // Load browser polyfills
        if let Err(e) = js.evaluate(BROWSER_POLYFILLS) {
            eprintln!("[Rendero] Polyfill error: {e}");
        }

        // Load the React + DOM shim bundle
        let bundle_path = Self::find_bundle();
        match std::fs::read_to_string(&bundle_path) {
            Ok(code) => {
                println!("[Rendero] Loading bundle: {bundle_path} ({} chars)", code.len());
                if let Err(e) = js.evaluate(&code) {
                    eprintln!("[Rendero] Bundle error: {e}");
                }
            }
            Err(e) => eprintln!("[Rendero] Bundle not found at {bundle_path}: {e}"),
        }

        // CRITICAL: Drain pending jobs — React's scheduler posts the commit
        // via MessageChannel/setTimeout. This fires the commit synchronously.
        println!("[Rendero] Draining JS jobs (React commit)...");
        js.drain_pending_jobs();
        println!("[Rendero] Jobs drained. Ready to render.");

        Self {
            window: None,
            surface: None,
            window_rc: None,
            engine,
            js,
            initialized: false,
            frame_count: 0,
        }
    }

    fn find_bundle() -> String {
        let demo = std::env::var("RENDERO_DEMO").unwrap_or_else(|_| "react".to_string());
        let candidates = [
            format!("docs/demos/dom-shim/dist/macos-{demo}-bundle.js"),
            format!("../docs/demos/dom-shim/dist/macos-{demo}-bundle.js"),
            format!("dist/macos-{demo}-bundle.js"),
            "docs/demos/dom-shim/dist/macos-bundle.js".to_string(),
            "../docs/demos/dom-shim/dist/macos-bundle.js".to_string(),
            "dist/macos-bundle.js".to_string(),
        ];
        for c in &candidates {
            if std::path::Path::new(c).exists() {
                return c.to_string();
            }
        }
        candidates[0].to_string()
    }

    fn render_frame(&mut self) {
        // 1. Drain JS callbacks (React state updates, setTimeout, etc.)
        self.js.drain_pending_jobs();

        // 2. Call the shim's flush function
        let _ = self.js.call_global("__shimFlushAndRender");

        // 3. Render pixels from engine
        let Some(window) = &self.window_rc else { return };
        let size = window.inner_size();
        let w = size.width;
        let h = size.height;
        if w == 0 || h == 0 { return; }

        // Update engine viewport
        {
            let mut e = self.engine.borrow_mut();
            if e.viewport_width != w || e.viewport_height != h {
                e.viewport_width = w;
                e.viewport_height = h;
                let scale = window.scale_factor() as f32;
                e.cam_zoom = scale;
                println!("[Rendero] Viewport: {w}x{h} @{scale}x");
            }
        }

        let pixels = self.engine.borrow_mut().render_pixels(w, h);

        if self.frame_count == 0 {
            println!("[Rendero] First frame: {w}x{h}, {} pixels", pixels.len() / 4);
            if let Ok(path) = std::env::var("RENDERO_DUMP_FRAME") {
                if let Err(err) = Self::dump_first_frame(&path, &pixels, w, h) {
                    eprintln!("[Rendero] Frame dump failed: {err}");
                } else {
                    println!("[Rendero] Wrote first frame to {path}");
                }
            }
        }
        self.frame_count += 1;

        // 4. Blit to window surface
        let Some(surface) = &mut self.surface else { return };
        let Ok(mut buffer) = surface.buffer_mut() else { return };

        // Convert RGBA u8 → u32 (softbuffer uses 0x00RRGGBB)
        for (i, pixel) in buffer.iter_mut().enumerate() {
            let offset = i * 4;
            if offset + 3 < pixels.len() {
                let r = pixels[offset] as u32;
                let g = pixels[offset + 1] as u32;
                let b = pixels[offset + 2] as u32;
                *pixel = (r << 16) | (g << 8) | b;
            }
        }

        let _ = buffer.present();
    }

    fn dump_first_frame(path: &str, pixels: &[u8], width: u32, height: u32) -> std::io::Result<()> {
        use std::io::Write;

        let mut file = std::fs::File::create(path)?;
        write!(file, "P6\n{} {}\n255\n", width, height)?;
        for rgba in pixels.chunks_exact(4) {
            file.write_all(&rgba[..3])?;
        }
        file.flush()?;
        Ok(())
    }
}

fn dump_ppm(path: &str, pixels: &[u8], width: u32, height: u32) -> std::io::Result<()> {
    use std::io::Write;

    let mut file = std::fs::File::create(path)?;
    write!(file, "P6\n{} {}\n255\n", width, height)?;
    for rgba in pixels.chunks_exact(4) {
        file.write_all(&rgba[..3])?;
    }
    file.flush()?;
    Ok(())
}

fn run_headless_dump(path: &str) -> Result<(), String> {
    let engine = Rc::new(RefCell::new(Engine::new("RenderoNativeHeadless", 1)));
    let mut js = QuickJSRuntime::new();
    let native_api = NoopNativeAPI;

    let width = std::env::var("RENDERO_HEADLESS_WIDTH")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(1024);
    let height = std::env::var("RENDERO_HEADLESS_HEIGHT")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(768);
    {
        let mut e = engine.borrow_mut();
        e.viewport_width = width;
        e.viewport_height = height;
        e.cam_zoom = 1.0;
    }

    js.with_context(|ctx| {
        register_engine_functions(&ctx, &engine, &native_api);
    });
    js.evaluate(&format!(
        "var __screenWidth = {w}; var __screenHeight = {h}; var __screenScale = 1;\
         if (globalThis.__RenderoHostBridge) {{\
           __RenderoHostBridge.screen.width = {w};\
           __RenderoHostBridge.screen.height = {h};\
           __RenderoHostBridge.screen.scale = 1;\
         }}",
        w = width, h = height
    ))?;

    js.evaluate(BROWSER_POLYFILLS)?;

    let bundle_path = App::find_bundle();
    let code = std::fs::read_to_string(&bundle_path)
        .map_err(|e| format!("bundle read failed at {bundle_path}: {e}"))?;
    println!("[Rendero] Loading bundle: {bundle_path} ({} chars)", code.len());
    js.evaluate(&code)?;
    println!("[Rendero] Draining JS jobs (headless)...");
    js.drain_pending_jobs();
    let _ = js.call_global("__shimFlushAndRender");
    js.drain_pending_jobs();
    let _ = js.call_global("__shimFlushAndRender");

    let pixels = engine.borrow_mut().render_pixels(width, height);
    dump_ppm(path, &pixels, width, height).map_err(|e| e.to_string())?;
    println!("[Rendero] Headless frame written to {path}");
    Ok(())
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.initialized { return; }
        self.initialized = true;

        let attrs = WindowAttributes::default()
            .with_title("Rendero — React on Native Rust Engine")
            .with_inner_size(LogicalSize::new(1024u32, 768u32));

        let window = event_loop.create_window(attrs).expect("Failed to create window");
        let window_rc = Rc::new(window);

        let context = softbuffer::Context::new(window_rc.clone()).expect("Failed to create softbuffer context");
        let surface = softbuffer::Surface::new(&context, window_rc.clone()).expect("Failed to create surface");

        self.window_rc = Some(window_rc);
        self.surface = Some(surface);

        println!("[Rendero] Window created. Running render loop.");
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                println!("[Rendero] Window closed.");
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if let Some(surface) = &mut self.surface {
                    let _ = surface.resize(
                        NonZeroU32::new(size.width.max(1)).unwrap(),
                        NonZeroU32::new(size.height.max(1)).unwrap(),
                    );
                }
            }
            WindowEvent::RedrawRequested => {
                self.render_frame();
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let dy = match delta {
                    MouseScrollDelta::LineDelta(_, y) => -y * 40.0,
                    MouseScrollDelta::PixelDelta(pos) => -pos.y as f32,
                };
                if dy.abs() > 0.0 {
                    // Move camera directly — no JS round-trip needed
                    let mut e = self.engine.borrow_mut();
                    e.cam_y = (e.cam_y + dy).max(0.0);
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        // Request a redraw every frame
        if let Some(w) = &self.window_rc {
            w.request_redraw();
        }
    }
}

fn main() {
    println!("[Rendero] Starting native shell...");

    if let Ok(path) = std::env::var("RENDERO_HEADLESS_DUMP") {
        if let Err(err) = run_headless_dump(&path) {
            eprintln!("[Rendero] Headless dump failed: {err}");
            std::process::exit(1);
        }
        return;
    }

    let event_loop = EventLoop::new().expect("Failed to create event loop");
    let mut app = App::new();

    event_loop.run_app(&mut app).expect("Event loop failed");
}
