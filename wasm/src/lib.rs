use rustpython_bytecode::bytecode;
use rustpython_compiler as compile;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn init() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
}

#[wasm_bindgen]
pub fn start(module: String, width: i32, height: i32, frozen: Box<[u8]>) -> Result<(), JsValue> {
    let frozen = bincode::deserialize(&frozen).map_err(|e| JsValue::from_str(&e.to_string()))?;
    pyckitup_core::run(pyckitup_core::InitOptions {
        width,
        height,
        entry_module: Some(module),
        frozen: Some(frozen),
        ..Default::default()
    })
}

#[wasm_bindgen]
pub fn start_source(source: String, width: i32, height: i32) -> Result<(), JsValue> {
    let code = compile::compile(
        &source,
        compile::Mode::Exec,
        "<qs>".to_owned(),
        Default::default(),
    )
    .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let frozen = bytecode::FrozenModule {
        code,
        package: false,
    };
    let frozen = {
        let mut m = HashMap::with_capacity(1);
        m.insert("run".to_owned(), frozen);
        m
    };
    pyckitup_core::run(pyckitup_core::InitOptions {
        width,
        height,
        entry_module: Some("run".to_owned()),
        frozen: Some(frozen),
        ..Default::default()
    })
}
