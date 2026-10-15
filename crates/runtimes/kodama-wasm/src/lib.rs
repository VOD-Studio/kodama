use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn execute(input: &str) -> String {
    kodama_core::execute(input)
}
