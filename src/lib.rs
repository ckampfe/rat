#![recursion_limit = "2048"]

mod app;
mod rasterize;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn main() {
    yew::start_app::<app::Model>();
}
