use oxirast_core::web_sys;
use oxirast_parser::rsx;
// Import the Dashboard component so we can render it
use crate::components::dashboard::Dashboard; 

#[allow(non_snake_case)] 
pub fn Login() -> web_sys::Element {
    let handle_click = move |_e| {
        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        let body = document.body().unwrap();

        // 1. Clear the current screen (removes the Login component)
        body.set_inner_html("");

        // 2. Generate the Dashboard component
        let dashboard_page = Dashboard();

        // 3. Mount the new page to the screen
        body.append_child(&dashboard_page).unwrap();
    };

    rsx!(
        <div class="react-card">
            <h2>"Secure Login"</h2>
            <input type="text" class="input-field" placeholder="Username" />
            <input type="password" class="input-field" placeholder="Password" />
            
            // When clicked, it will run our DOM-swapping closure above
            <button on_click={handle_click}>"Submit"</button>
        </div>
    )
}