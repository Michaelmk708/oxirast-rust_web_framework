# 🔥 Oxirast Framework

**Oxirast** is a high-performance, fine-grained reactive web framework
built in Rust. It aims to provide the familiar Developer Experience (DX)
of React while leveraging the speed and safety of WebAssembly---without
the overhead of a Virtual DOM.

------------------------------------------------------------------------

## 🚀 Key Features

-   **Zero Virtual DOM:** Direct DOM manipulation for maximum
    performance.
-   **Reactive Signals:** Fine-grained updates using a `use_state` hook
    system.
-   **RSX Syntax:** Write HTML-in-Rust using a custom procedural macro.
-   **Modular Architecture:** Support for Capitalized Components (e.g.,
    `<Login />`) across multiple files.
-   **Integrated Dev Server:** Auto-reloading CLI with built-in
    WebAssembly compilation.

------------------------------------------------------------------------

## 🏗 Project Architecture

Oxirast is organized as a Rust workspace with four specialized crates:

  -----------------------------------------------------------------------
  Crate                           Purpose
  ------------------------------- ---------------------------------------
  `oxirast-core`                  The runtime engine. Manages DOM
                                  mounting, events, and reactivity.

  `oxirast-parser`                The "compiler" layer. A procedural
                                  macro that transforms RSX into Rust
                                  code.

  `oxirast-cli`                   The developer's companion. Handles file
                                  watching, Wasm builds, and static
                                  serving.

  `oxirast-demo`                  The implementation layer. Where the
                                  modular web app (Login/Dashboard)
                                  lives.
  -----------------------------------------------------------------------

------------------------------------------------------------------------

## 🛠 Getting Started

### Prerequisites

-   Rust (Stable)
-   `wasm32-unknown-unknown` target:

``` bash
rustup target add wasm32-unknown-unknown
```

-   `wasm-bindgen-cli`:

``` bash
cargo install wasm-bindgen-cli
```

------------------------------------------------------------------------

### Installation & Run

Clone the repository:

``` bash
git clone https://github.com/your-username/oxirast.git
cd oxirast
```

Start the development server:

``` bash
cargo run -p oxirast-cli
```

Open your browser to:

http://localhost:3000

------------------------------------------------------------------------

## 💻 Example Usage

Writing a component in Oxirast feels like writing modern React, but with
the power of Rust types.

``` rust
// components/login.rs
pub fn Login() -> Element {
    let username = use_state("Dev".to_string());
    
    let handle_input = move |e| {
        let val = get_input_value(e);
        username.set(val);
    };

    rsx!(
        <div class="card">
            <h1>"Welcome, "{username.get()}</h1>
            <input type="text" on_input={handle_input} />
        </div>
    )
}
```

------------------------------------------------------------------------

## 🛠 Roadmap

-   [x] Custom RSX Parser
-   [x] Fine-grained Reactivity (`use_state`)
-   [x] Modular Component Support
-   [ ] Props System (Passing data between components)
-   [ ] Global Router (URL-based navigation)
-   [ ] SSR (Server Side Rendering) support

------------------------------------------------------------------------

## 📄 License

MIT © Michael (Vyron Trust)
