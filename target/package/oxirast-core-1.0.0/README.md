
# Oxirast

**The fine-grained, lightning-fast WebAssembly framework for Rust.**


## Why Oxirast?

Most frontend frameworks rely on a **Virtual DOM** — when state changes, they re-render your entire component, diff a new VDOM tree against the old one, and patch the real DOM. This diffing step is expensive and grows with your component tree.

Oxirast takes a fundamentally different approach: **Fine-Grained Reactivity**.

State lives in **Signals**. When a Signal mutates, Oxirast does not re-render the component. It reaches directly into the live browser DOM and surgically updates *only* the specific text node or attribute that depends on that Signal. The rest of your tree is never touched.

| | Virtual DOM (React) | Fine-Grained (Oxirast) |
|---|---|---|
| State change triggers | Full component re-render | Targeted Signal update only |
| DOM update method | Diff + patch VDOM tree | Direct DOM mutation |
| Re-render overhead | O(component size) | O(1) per subscriber |
| Memory model | GC-managed heap (JS) | Rust ownership + Wasm GC |
| Language | JavaScript / TypeScript | Rust → WebAssembly |

---

## Features

- 🎯 **True Fine-Grained Reactivity** — Signals mutate, DOM updates. No re-renders. No diffing.
- 🦀 **100% Rust** — Write your entire frontend in one language with full type safety.
- 📝 **Declarative UI via `rsx!`** — HTML-like syntax compiled to optimised Wasm at build time.
- 🔀 **Built-in SPA Router** — History-based client-side routing with automatic memory cleanup.
- ⚡ **Zero-Config CLI** — Scaffold, compile, and hot-reload with a single command.
- 🧹 **Automatic Memory Management** — Wasm-to-JS garbage collector purges orphaned closures and Signals on page transitions, preventing memory leaks.

---

## Workspace Structure

This repository is a Cargo workspace containing three crates:
```
oxirast/
├── oxirast-core/      # Runtime engine — Signals, VNode, Router, DOM bindings
├── oxirast-parser/    # Compile-time procedural macro engine — rsx! transformation
└── oxirast-cli/       # Developer CLI — scaffold, serve with hot-reload
```

### `oxirast-core`
The runtime engine. Contains the Virtual DOM, the `Signal<T>` reactivity system, the `Router`, async fetch hooks (`use_fetch`), the Context API (`provide_context` / `use_context`), and all `wasm-bindgen` DOM bindings.

### `oxirast-parser`
The compile-time macro engine. Transforms your `rsx!` HTML-like syntax into optimised Rust/VNode instructions. Handles `bind_text` directive wiring and `on_*` event listener compilation.

### `oxirast-cli`
The developer toolkit. A globally installed binary that scaffolds new projects and runs a hot-reloading development server at `http://localhost:3000`.

---

## Quick Start

**1. Install the CLI**
```bash
cargo install oxirast-cli
```

**2. Scaffold a new project**
```bash
oxirast-cli init my_app
cd my_app
```

**3. Start the dev server**
```bash
oxirast-cli
```

Your app is now compiling to WebAssembly and running at `http://localhost:3000` with hot-reloading enabled.

---

## Usage

### Components & the `rsx!` Macro
```rust
use oxirast_core::VNode;
use oxirast_parser::rsx;

#[allow(non_snake_case)]
pub fn WelcomeCard() -> VNode {
    rsx!(
        <div class="card">
            <h1>"Welcome to Oxirast"</h1>
            <p>"Compiled to WebAssembly."</p>
        </div>
    )
}
```

### Reactivity with Signals
```rust
use oxirast_core::{use_state, VNode};
use oxirast_parser::rsx;

#[allow(non_snake_case)]
pub fn Counter() -> VNode {
    let count        = use_state(0);
    let display_text = use_state(String::from("Clicks: 0"));

    let btn_count = count.clone();
    let btn_text  = display_text.clone();

    let handle_click = move |_e| {
        let next = btn_count.get() + 1;
        btn_count.set(next);
        btn_text.set(format!("Clicks: {}", next));
    };

    rsx!(
        <div class="counter-box">
            <h2 bind_text={display_text}></h2>
            <button on_click={handle_click}>"Increment"</button>
        </div>
    )
}
```

### Single Page Routing
```rust
use oxirast_core::{Router, Signal, VNode};
use oxirast_parser::rsx;

#[allow(non_snake_case)]
pub fn Home(nav: Signal<String>) -> VNode {
    let nav_clone = nav.clone();
    let go_about = move |_| nav_clone.set(String::from("/about"));

    rsx!(
        <div>
            <h1>"Home Page"</h1>
            <button on_click={go_about}>"Go to About"</button>
        </div>
    )
}

#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn main() {
    Router::new("root")
        .route("/",      Home)
        .route("/about", About)
        .start();
}
```

---

## `Cargo.toml` Dependencies

Add the following to your project's `Cargo.toml`:
```toml
[dependencies]
oxirast-core   = "0.1.1"
oxirast-parser = "0.1.1"
wasm-bindgen   = "0.2"

[lib]
crate-type = ["cdylib"]
```

---

## Project Structure (Scaffolded App)
```
my_app/
├── Cargo.toml
├── Cargo.lock
├── public/
│   ├── index.html     # HTML shell — Oxirast mounts into <div id="root">
│   └── style.css      # Global styles
└── src/
    ├── lib.rs         # Entry point & Router configuration
    └── pages/
        ├── home.rs
        └── about.rs
```

---

## Roadmap

- [x] `Signal<T>` fine-grained reactivity
- [x] `rsx!` macro — HTML-like declarative UI
- [x] `bind_text` directive
- [x] `on_*` event listener attributes
- [x] Client-side SPA Router with History API
- [x] `use_state` / `use_fetch` hooks
- [x] Context API (`provide_context` / `use_context`)
- [x] Automatic Wasm-to-JS GC (memory reaper)
- [x] Zero-config CLI (`init` + `serve`)
- [ ] `oxirast-cli build` — optimised release builds with `wasm-opt`
- [ ] `oxirast-cli clean`
- [ ] `--template` flag for `init`
- [ ] `oxirast.toml` project configuration
- [ ] Server-Side Rendering (SSR)
- [ ] Component hot-module replacement (HMR)
- [ ] `bind_attr` directive for reactive HTML attributes

---

## Contributing

Contributions are welcome. Please open an issue first to discuss any significant changes.

1. Fork the repository
2. Create a feature branch — `git checkout -b feat/your-feature`
3. Commit your changes — `git commit -m "feat: add your feature"`
4. Push to the branch — `git push origin feat/your-feature`
5. Open a Pull Request

---

## License

MIT License © [Michael (kinuthia)](https://mkportifolio.netlify.app)

See [LICENSE](LICENSE) for the full text.

---

<div align="center">
  <sub>Built with 🦀 Rust · Compiled to WebAssembly · Designed for the modern web</sub>
</div>
