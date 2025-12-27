//! Performance logging utilities for frontend debugging.
//!
//! Enable by setting `PERF_LOG=true` in browser console:
//! ```js
//! localStorage.setItem('PERF_LOG', 'true');
//! location.reload();
//! ```

use wasm_bindgen::prelude::*;
use web_sys::window;

/// Check if performance logging is enabled via localStorage
fn is_enabled() -> bool {
    window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|s| s.get_item("PERF_LOG").ok().flatten())
        .map(|v| v == "true")
        .unwrap_or(false)
}

/// Get current high-resolution timestamp in milliseconds
pub fn now() -> f64 {
    window()
        .and_then(|w| w.performance())
        .map(|p| p.now())
        .unwrap_or(0.0)
}

/// Log a performance measurement
pub fn log(label: &str, duration_ms: f64) {
    if !is_enabled() {
        return;
    }

    let color = if duration_ms > 16.0 {
        "color: red; font-weight: bold"
    } else if duration_ms > 8.0 {
        "color: orange"
    } else {
        "color: green"
    };

    web_sys::console::log_1(&format!("%c[PERF] {} took {:.2}ms", label, duration_ms).into());
    // Use styled logging via JS
    log_styled(label, duration_ms, color);
}

#[wasm_bindgen(inline_js = r#"
export function log_styled(label, duration, color) {
    console.log(`%c[PERF] ${label} took ${duration.toFixed(2)}ms`, color);
}
"#)]
extern "C" {
    fn log_styled(label: &str, duration: f64, color: &str);
}

/// RAII guard that logs duration when dropped
pub struct PerfGuard {
    label: String,
    start: f64,
    enabled: bool,
}

impl PerfGuard {
    pub fn new(label: impl Into<String>) -> Self {
        let enabled = is_enabled();
        Self {
            label: label.into(),
            start: if enabled { now() } else { 0.0 },
            enabled,
        }
    }
}

impl Drop for PerfGuard {
    fn drop(&mut self) {
        if self.enabled {
            let duration = now() - self.start;
            log(&self.label, duration);
        }
    }
}

/// Macro for easy performance measurement of a block
#[macro_export]
macro_rules! perf_scope {
    ($label:expr) => {
        let _guard = $crate::utils::perf::PerfGuard::new($label);
    };
}

/// Macro for measuring function execution
#[macro_export]
macro_rules! perf_fn {
    () => {
        let _guard =
            $crate::utils::perf::PerfGuard::new(concat!(module_path!(), "::", stringify!(fn)));
    };
}
