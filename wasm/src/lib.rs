use std::cell::Cell;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn start(w: i32, h: i32, frozen: Box<[u8]>) -> Result<(), JsValue> {
    let frozen = bincode::deserialize(&frozen).map_err(|e| JsValue::from_str(&e.to_string()))?;
    pyckitup_core::FROZEN.set(&Cell::new(frozen), || {
        pyckitup_core::run(w, h);
    });
    Ok(())
}
