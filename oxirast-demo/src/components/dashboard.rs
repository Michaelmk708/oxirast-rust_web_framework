use oxirast_core::web_sys;
use oxirast_parser::rsx;

#[allow(non_snake_case)]
pub fn Dashboard() -> web_sys::Element {
    rsx!(
        <div class="react-card">
            <h2>"Vyron Trust Dashboard"</h2>
            <p>"Authentication successful."</p>
            <p class="live-text">"Welcome to your secure portal."</p>
        </div>
    )
}