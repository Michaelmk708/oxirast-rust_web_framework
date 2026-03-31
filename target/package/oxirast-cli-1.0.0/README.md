Oxirast 🦀⚡The fine-grained, lightning-fast, Web3-native WebAssembly framework for Rust.Why Oxirast?Most frontend frameworks rely on a Virtual DOM — when state changes, they re-render your entire component, diff a new VDOM tree against the old one, and patch the real DOM. This diffing step is expensive and grows with your component tree.Oxirast takes a fundamentally different approach: Fine-Grained Reactivity.State lives in Signals. When a Signal mutates, Oxirast does not re-render the component. It reaches directly into the live browser DOM and surgically updates only the specific text node or attribute that depends on that Signal. The rest of your tree is never touched.FeatureVirtual DOM FrameworksOxirast (v1.0)State change triggersFull component re-renderTargeted Signal update onlyDOM update methodDiff + patch VDOM treeDirect DOM mutationRe-render overheadO(component size)O(1) per subscriberArchitectureClient-heavy SPAIsomorphic (SSR + True Hydration)Web3 NativeRequires heavy JS bridgingBuilt-in use_wallet hooksLanguageJavaScript / TypeScript100% Rust → WebAssembly🔥 Features (v1.0.0)🎯 True Fine-Grained Reactivity — bind_text, bind_attr, and bind_show. No re-renders. No diffing.🦀 Isomorphic Architecture (SSR) — Write once, render to HTML on your Axum/Actix backend, and truly hydrate on the client.⛓️ Web3 Wallet Engine — Built-in, zero-JS-config hooks to instantly connect MetaMask (EVM) and Phantom (Solana).🔀 Enterprise Routing — Dynamic parameters (/user/:id), query parsing (?sort=asc), nested wildcard routing, and Route Guards for instant auth protection.🛡️ Zero-Day Crash Protection — Built-in React-style "Red Screen of Death" Error Boundaries intercept Rust panics to prevent silent Wasm crashes.🎨 Tailwind CSS Auto-Compiler — The CLI detects tailwind.config.js and automatically compiles your utility classes on the fly.⚡ Production-Ready CLI — Scaffold templates, hot-reload, audit dependencies (cargo audit), and aggressively compress Wasm binaries (wasm-opt) with one command.🚀 Quick Start1. Install the CLIBashcargo install oxirast-cli
2. Scaffold a new project (with Tailwind CSS!)Bashoxirast-cli init my_dapp --template tailwind
cd my_dapp
3. Run Security Audit & Start Dev ServerBashoxirast-cli audit
oxirast-cli serve
Your app is now compiling to WebAssembly, auto-compiling Tailwind, and running at http://localhost:3000 with hot-reloading enabled!🛠️ UsageComponents & Fine-Grained Reactivity (rsx!)Update attributes, text, and CSS classes instantly without re-rendering the component.Rustuse oxirast_core::{use_state, use_memo, VNode};
use oxirast_parser::rsx;

#[allow(non_snake_case)]
pub fn Counter() -> VNode {
    let count = use_state(0);
    
    // Derived state automatically updates when `count` changes!
    let double = use_memo(&count, |c| format!("Double: {}", c * 2));
    let is_high = use_memo(&count, |c| if *c > 5 { "text-red-500" } else { "text-white" });

    let btn_count = count.clone();
    let handle_click = move |_e| btn_count.set(btn_count.get() + 1);

    rsx!(
        <div class="p-4 border rounded">
            <h2 bind_attr:class={is_high} bind_text={double}></h2>
            <button on_click={handle_click}>"Increment"</button>
        </div>
    )
}
Enterprise Routing & GuardsProtect routes and parse dynamic URLs instantly.Rustuse oxirast_core::{Router, Signal, use_params, use_query, VNode};
use oxirast_parser::rsx;

// Fake auth check
fn is_authenticated() -> bool { true }

#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn main() {
    Router::new("root")
        .route("/", Home)
        .route("/login", Login)
        // Redirects to /login if is_authenticated() returns false
        .guarded_route("/dashboard/:id", Dashboard, is_authenticated, "/login")
        .start();
}

#[allow(non_snake_case)]
pub fn Dashboard(nav: Signal<String>) -> VNode {
    let user_id = use_params().get("id").cloned().unwrap_or_default();
    let theme = use_query().get("theme").cloned().unwrap_or_default();

    rsx!(
        <div>
            <h1>"Welcome User: "{user_id}</h1>
            <p>"Current Theme: "{theme}</p>
        </div>
    )
}
Web3 Wallet Integration (Zero JS required)Connect to Solana or Ethereum natively from Rust.Rustuse oxirast_core::{use_wallet, use_memo, VNode};
use oxirast_parser::rsx;

#[allow(non_snake_case)]
pub fn Web3Connect() -> VNode {
    // Built in support for "solana" (Phantom) or "ethereum" (MetaMask)
    let (wallet_address, is_connecting, connect_wallet) = use_wallet("solana"); 

    let display_text = use_memo(&wallet_address, |addr| {
        match addr {
            Some(a) => format!("Connected: {}...", &a[0..6]),
            None => String::from("Not Connected"),
        }
    });

    rsx!(
        <div class="web3-card">
            <h2 bind_text={display_text}></h2>
            <button bind_attr:disabled={is_connecting} on_click={move |_| connect_wallet()}>
                "Connect Wallet"
            </button>
        </div>
    )
}
📦 Cargo.toml DependenciesAdd the following to your project's Cargo.toml:Ini, TOML[dependencies]
oxirast-core   = "1.0.0"
oxirast-parser = "1.0.0"
wasm-bindgen   = "0.2"

[lib]
crate-type = ["cdylib"]
🗺️ Roadmap (What's Next for v2.0)[x] Signal<T> fine-grained reactivity[x] Client-side SPA Router with Route Guards & Dynamic Params[x] oxirast-cli build — optimised release builds with wasm-opt[x] oxirast-cli audit — supply chain zero-day protection[x] --template tailwind and auto-compilation[x] Server-Side Rendering (SSR) & True Hydration[x] Error Boundaries (Crash Protection)[x] Web3 Wallet Integration[ ] Type-Safe RPC Bridge: Seamlessly share Rust Struct definitions between your Axum backend and Oxirast frontend.[ ] Component Hot-Module Replacement (HMR) (Currently supports fast Live-Reload)🤝 ContributingContributions are welcome. Please open an issue first to discuss any significant changes.Fork the repositoryCreate a feature branch — git checkout -b feat/your-featureCommit your changes — git commit -m "feat: add your feature"Push to the branch — git push origin feat/your-featureOpen a Pull Request📄 LicenseMIT License © Michael KinuthiaSee LICENSE for the full text.<div align="center"><sub>Built with 🦀 Rust · Compiled to WebAssembly · Designed for the modern web</sub></div>