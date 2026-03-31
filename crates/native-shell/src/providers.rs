//! Provider traits for the native shell.
//!
//! The JS runtime is behind a trait so we can swap QuickJS for Hermes later.

/// A JavaScript runtime that can evaluate code, drain async callbacks,
/// and expose native functions to JS.
pub trait JSRuntime {
    /// Evaluate a JS string. Returns error message on failure.
    fn evaluate(&mut self, code: &str) -> Result<(), String>;

    /// Drain callbacks scheduled through the host-managed callback queue.
    /// This is the explicit boundary React/Vue scheduler work funnels through.
    fn drain_host_callbacks(&mut self);

    /// Drain all pending async jobs (setTimeout, Promise, MessageChannel).
    /// Call this each frame to pump React's scheduler.
    fn drain_pending_jobs(&mut self);

    /// Check if a global function exists.
    fn has_global(&mut self, name: &str) -> bool;

    /// Call a global JS function with no arguments.
    fn call_global(&mut self, name: &str) -> Result<(), String>;
}

/// Future native platform surface. v1 is intentionally minimal and mostly stubbed.
pub trait NativeAPI {
    fn storage_get(&self, key: &str) -> Option<String>;
    fn storage_set(&self, key: &str, value: &str) -> Result<(), String>;
    fn clipboard_read_text(&self) -> Option<String>;
    fn clipboard_write_text(&self, text: &str) -> Result<(), String>;
    fn notifications_request_permission(&self) -> bool;
    fn haptics_impact(&self, style: &str) -> Result<(), String>;
    fn media_pick_image(&self) -> Result<Option<String>, String>;
}

/// Default native API implementation used by the macOS-first milestone.
/// It keeps the JS surface stable while we defer real platform bindings.
pub struct NoopNativeAPI;

impl NativeAPI for NoopNativeAPI {
    fn storage_get(&self, _key: &str) -> Option<String> { None }

    fn storage_set(&self, _key: &str, _value: &str) -> Result<(), String> { Ok(()) }

    fn clipboard_read_text(&self) -> Option<String> { None }

    fn clipboard_write_text(&self, _text: &str) -> Result<(), String> { Ok(()) }

    fn notifications_request_permission(&self) -> bool { false }

    fn haptics_impact(&self, _style: &str) -> Result<(), String> { Ok(()) }

    fn media_pick_image(&self) -> Result<Option<String>, String> { Ok(None) }
}
