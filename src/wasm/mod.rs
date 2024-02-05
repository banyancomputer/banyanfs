use tracing::Level;
use tracing_wasm::{ConsoleConfig, WASMLayerConfigBuilder};
use wasm_bindgen::prelude::*;

use crate::version::version;

#[wasm_bindgen(start)]
pub fn wasm_init() -> Result<(), JsValue> {
    // Only run this in debug mode, in release mode this bloats up the library quite a bit
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();

    let wasm_log_config = if cfg!(debug_assertions) {
        WASMLayerConfigBuilder::default()
            .set_report_logs_in_timings(true)
            .set_max_level(Level::DEBUG)
            .set_console_config(ConsoleConfig::ReportWithoutConsoleColor)
            .build()
    } else {
        WASMLayerConfigBuilder::default()
            .set_report_logs_in_timings(false)
            .set_max_level(Level::INFO)
            .set_console_config(ConsoleConfig::ReportWithoutConsoleColor)
            .build()
    };

    tracing_wasm::set_as_global_default_with_config(wasm_log_config);
    tracing::debug!("successfully loaded banyanfs WASM module {}", version());

    Ok(())
}
