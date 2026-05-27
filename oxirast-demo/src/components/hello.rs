use oxirast_core::{mount_to_body , render_vnode, VNode};
use oxirast_parser::rsx;

#[allow(non_snake_case)]
pub fn HelloWorld()-> VNode {
    rsx!(
        <div>
        <h1>"hello oxirast"</h1>
        <p> "my first wasm component is running"</p>
        </div>
    )
}
//rust program must have the main function its the entry point
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn main(){
    //call the function to generate vnode blueprint
    let blueprint = HelloWorld();
//ask the engine to turn blueprint into real html nodes
    let real_html=render_vnode(&blueprint);
    //inject the nodes directly into the browsers <body > tag
    mount_to_body(&real_html);

  

}