//! This module contains a thin wrapper around the BanyanFS library to expose the functionality to
//! browsers. The current content of this module is largely implemented in a way to drop into the
//! BanyanFS platform with minimal changes relative to how the prior filesystem format was
//! integrated and should all be considered deprecated.
//!
//! A more idiomatic and consistent API for browser clients is in the works but hasn't been
//! released.

use tracing::Level;
use tracing_wasm::{ConsoleConfig, WASMLayerConfigBuilder};
use wasm_bindgen::prelude::*;

pub(crate) mod data_storage;
pub(crate) use data_storage::WasmDataStorage;

#[cfg(feature = "tomb-compat")]
pub(crate) mod tomb_compat;

#[cfg(feature = "tomb-compat")]
pub use tomb_compat::*;

use crate::error::BanyanFsError;
use crate::version::full_version;

use tracing::info;

// Pending needed improvements for the WASM components:
//
// - Ensure all methods that need it return a result with an effective error message instead of
//   panicing. Need to get all the unwraps() out but we need to maintain the type signatures for
//   the time being.

/// Performs first time setup to the WASM environment once this library is loaded. This primarily
/// sets up logging and reports the library version.
#[wasm_bindgen(start)]
pub fn wasm_init() -> Result<(), JsValue> {
    // Only run this in debug mode, in release mode this bloats up the library quite a bit
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();

    let wasm_log_config = WASMLayerConfigBuilder::default()
        .set_report_logs_in_timings(cfg!(debug_assertions))
        .set_max_level(configured_log_level())
        .set_console_config(ConsoleConfig::ReportWithoutConsoleColor)
        .build();

    tracing_wasm::set_as_global_default_with_config(wasm_log_config);
    info!(
        "successfully loaded banyanfs WASM module {}",
        full_version()
    );

    Ok(())
}

/// Sets up our default logging level and allows for in the field configuration for additional
/// details by setting the "banaynfs.log_level" item in local storage to the desired log level.
/// Valid values are "trace", "debug", "info", "warn", and "error". All other values are ignored.
///
/// This dynamic feature is very useful especially during active development but may be removed or
/// become opt in in the future as it significantly increases the size of the WASM library.
fn configured_log_level() -> Level {
    let default_level = if cfg!(debug_assertions) {
        Level::TRACE
    } else {
        Level::DEBUG
    };

    // todo(sstelfox): this dynamic log config level may not be worth it, it bloats our WASM
    // library about 25%...
    let maybe_config_item = web_sys::window()
        .and_then(|w| w.local_storage().ok())
        .flatten()
        .and_then(|ls| ls.get_item("banyanfs.log_level").ok())
        .flatten()
        .map(|ll| ll.to_lowercase());

    match maybe_config_item.as_deref() {
        Some("trace") => Level::TRACE,
        Some("debug") => Level::DEBUG,
        Some("info") => Level::INFO,
        Some("warn") => Level::WARN,
        Some("error") => Level::ERROR,
        _ => default_level,
    }
}

impl From<BanyanFsError> for JsValue {
    fn from(error: BanyanFsError) -> Self {
        JsValue::from_str(&error.to_string())
    }
}

impl From<serde_wasm_bindgen::Error> for BanyanFsError {
    fn from(error: serde_wasm_bindgen::Error) -> Self {
        Self::from(error.to_string())
    }
}
