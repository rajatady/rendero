//! QuickJS runtime — implements JSRuntime trait.
//!
//! QuickJS doesn't have setTimeout/setInterval built-in.
//! We implement them by storing callbacks in a JS array (__pendingTimers)
//! and draining them each frame alongside QuickJS's own job queue.

use rquickjs::{Context, Runtime};
use crate::providers::JSRuntime;

pub struct QuickJSRuntime {
    runtime: Runtime,
    context: Context,
}

impl QuickJSRuntime {
    pub fn new() -> Self {
        let runtime = Runtime::new().expect("Failed to create QuickJS runtime");
        let context = Context::full(&runtime).expect("Failed to create QuickJS context");

        // Install setTimeout/setInterval/queueMicrotask as JS-side callback queue
        context.with(|ctx| {
            ctx.eval::<(), _>(r#"
                var __pendingTimers = [];
                function setTimeout(fn, delay) {
                    __pendingTimers.push(fn);
                    return __pendingTimers.length;
                }
                function clearTimeout(id) {}
                function setInterval(fn, ms) { return setTimeout(fn, ms); }
                function clearInterval(id) {}
                function queueMicrotask(fn) { __pendingTimers.push(fn); }
            "#).expect("Failed to install timer polyfills");
        });

        Self { runtime, context }
    }

    pub fn with_context<F, R>(&self, f: F) -> R
    where
        F: FnOnce(rquickjs::Ctx<'_>) -> R,
    {
        self.context.with(f)
    }
}

impl JSRuntime for QuickJSRuntime {
    fn evaluate(&mut self, code: &str) -> Result<(), String> {
        self.context.with(|ctx| {
            ctx.eval::<(), _>(code)
                .map_err(|e| format!("{e}"))
        })
    }

    fn drain_host_callbacks(&mut self) {
        self.context.with(|ctx| {
            ctx.eval::<(), _>(r#"
                var __batch = __pendingTimers.splice(0);
                for (var i = 0; i < __batch.length; i++) {
                    try { __batch[i](); } catch(e) {
                        if (typeof console !== 'undefined') console.error('timer callback error: ' + e.message);
                    }
                }
            "#).ok();
        });
    }

    fn drain_pending_jobs(&mut self) {
        // Drain both QuickJS's internal job queue and the host callback queue.
        // Loop multiple times because React/Vue schedulers may chain timers/promises.
        for _ in 0..20 {
            loop {
                match self.runtime.execute_pending_job() {
                    Ok(true) => continue,
                    _ => break,
                }
            }

            let has_timers = self.context.with(|ctx| {
                let result = ctx.eval::<bool, _>("__pendingTimers.length > 0");
                result.unwrap_or(false)
            });

            if !has_timers {
                break;
            }

            self.drain_host_callbacks();

            loop {
                match self.runtime.execute_pending_job() {
                    Ok(true) => continue,
                    _ => break,
                }
            }
        }
    }

    fn has_global(&mut self, name: &str) -> bool {
        self.context.with(|ctx| {
            let globals = ctx.globals();
            globals.get::<_, rquickjs::Value>(name)
                .map(|v| !v.is_undefined() && !v.is_null())
                .unwrap_or(false)
        })
    }

    fn call_global(&mut self, name: &str) -> Result<(), String> {
        let code = format!("if (typeof {name} === 'function') {name}();");
        self.evaluate(&code)
    }
}
