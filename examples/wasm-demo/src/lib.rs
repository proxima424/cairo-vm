mod utils;

use cairo_vm::{
    cairo_run::{cairo_run, CairoRunConfig},
    hint_processor::builtin_hint_processor::builtin_hint_processor_definition::BuiltinHintProcessor,
};
use wasm_bindgen::prelude::*;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(msg: &str);
}

#[cfg(feature = "console_error_panic_hook")]
#[wasm_bindgen(start)]
pub fn start() {
    crate::utils::set_panic_hook();
}

// TODO: check why this is needed. Seems wasm-bindgen expects us to use
// `std::error::Error` even if it's not yet in `core`
macro_rules! wrap_error {
    ($xp: expr) => {
        $xp.map_err(|e| JsError::new(e.to_string().as_str()))
    };
}

#[wasm_bindgen(js_name = runCairoProgram)]
pub fn run_cairo_program() -> Result<String, JsError> {
    const PROGRAM_JSON: &[u8] = include_bytes!("./array_sum.json");

    let mut hint_executor = BuiltinHintProcessor::new_empty();

    let cairo_run_config = CairoRunConfig {
        layout: "all_cairo",
        relocate_mem: true,
        trace_enabled: true,
        ..Default::default()
    };

    let (_runner, mut vm) = wrap_error!(cairo_run(
        PROGRAM_JSON,
        &cairo_run_config,
        &mut hint_executor
    ))?;

    let mut buffer = String::new();

    wrap_error!(vm.write_output(&mut buffer))?;

    log(buffer.as_str());

    Ok(buffer)
}
