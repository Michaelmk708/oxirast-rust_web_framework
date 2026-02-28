mod components; // Tells Rust to look at the components/mod.rs file

use components::login::Login; 
use oxirast_core::mount_to_body; // Removed web_sys from here!
use oxirast_parser::rsx;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn run() {
    let app = rsx!(
        <div class="app-container">
            <h1>"Welcome to Vyron Trust"</h1>
            <Login />
            <dashboard />
        </div>
    );

    mount_to_body(&app);
}