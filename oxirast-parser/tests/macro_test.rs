use oxirast_parser::rsx;

#[test]
fn test_complex_rsx_component() {
    // We are now testing a fully nested login form with attributes and self-closing tags!
    let generated_ui = rsx!(
        <form class="login-form">
            <div>"Welcome to Oxirast"</div>
            <input type="text" placeholder="Username" />
            <input type="password" placeholder="Password" />
            <button type="submit">"Login"</button>
        </form>
    );

    let expected_output = "<form class=\"login-form\"><div>Welcome to Oxirast</div><input type=\"text\" placeholder=\"Username\"/><input type=\"password\" placeholder=\"Password\"/><button type=\"submit\">Login</button></form>";

    assert_eq!(generated_ui, expected_output);
    
    println!("Complex Parsed Output:\n{}", generated_ui);
}