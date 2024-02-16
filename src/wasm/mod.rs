use tracing::Level;
use tracing_wasm::{ConsoleConfig, WASMLayerConfigBuilder};
use wasm_bindgen::prelude::*;

#[cfg(feature = "tomb-compat")]
pub(crate) mod tomb_compat;

#[cfg(feature = "tomb-compat")]
pub use tomb_compat::*;

use crate::version::version;

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
    tracing::info!("successfully loaded banyanfs WASM module {}", version());

    Ok(())
}

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
