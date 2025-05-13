use gloo_worker::Registrable;
use wasm_bindgen::prelude::*;

use shared::JokeStream;

#[wasm_bindgen(start)]
pub async fn start_worker() {
    JokeStream::registrar().register();
}
