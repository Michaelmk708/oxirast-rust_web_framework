use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, Request},
    http::{header, HeaderValue},
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use notify::{RecursiveMode, Watcher};
use std::{env, fs, path::Path, process::Command, sync::Arc};
use tokio::sync::broadcast;
use tower_http::services::ServeDir;

// ==========================================
// CONFIGURATION ENGINE (Reads oxirast.toml)
// ==========================================
fn get_config_port() -> u16 {
    if let Ok(config) = fs::read_to_string("oxirast.toml") {
        for line in config.lines() {
            if line.trim().starts_with("port =") {
                if let Some(port_str) = line.split('=').nth(1) {
                    return port_str.trim().parse().unwrap_or(3000);
                }
            }
        }
    }
    3000 // Default fallback
}

// ==========================================
// PILLAR 2: Aggressive Content Security Policy
// ==========================================
const INDEX_HTML: &str = r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>Oxirast App</title>
    <meta http-equiv="Content-Security-Policy" content="default-src 'self'; script-src 'self' 'unsafe-inline' 'wasm-unsafe-eval'; connect-src 'self' ws://localhost:*; style-src 'self' 'unsafe-inline';">
    <link rel="stylesheet" href="/public/style.css">
</head>
<body>
    <div id="root"></div>

    <script type="module">
        import init from '/dist/app.js';
        
        init().then(() => {
            console.log("🚀 Oxirast Framework Initialized!");
        });

        const port = window.location.port || "3000";
        const ws = new WebSocket(`ws://localhost:${port}/ws`);
        ws.onmessage = (event) => {
            if (event.data === "RELOAD") {
                console.log("♻️ File changed! Reloading...");
                window.location.reload();
            }
        };
    </script>
</body>
</html>
"#;

// ==========================================
// PILLAR 4: Security Audit Engine
// ==========================================
fn audit_project() -> bool {
    println!("🛡️  Running Security Audit (cargo audit)...");
    let status = Command::new("cargo").arg("audit").status();
    
    match status {
        Ok(s) if s.success() => {
            println!("✅ Security Audit passed! Zero vulnerabilities detected.");
            true
        }
        Ok(_) => {
            println!("❌ SECURITY ALERT: Vulnerabilities found in dependencies! Check Cargo.lock.");
            false 
        }
        Err(_) => {
            println!("⚠️  'cargo-audit' is not installed. Run 'cargo install cargo-audit' for zero-day protection.");
            true 
        }
    }
}

// ==========================================
// TESTING ENGINE (Headless Wasm)
// ==========================================
fn test_project() {
    println!("🧪 Running Headless WebAssembly Tests...");
    let status = Command::new("cargo")
        .args(["test", "--target", "wasm32-unknown-unknown"])
        .status();
        
    match status {
        Ok(s) if s.success() => println!("✅ All Oxirast tests passed!"),
        _ => println!("❌ Tests failed. Make sure you have 'wasm-bindgen-test' configured."),
    }
}

// ==========================================
// BUILD ENGINE (With Tailwind Auto-Detect)
// ==========================================
fn build_project(is_release: bool) {
    if is_release {
        println!("🚀 Compiling highly optimized Oxirast App for PRODUCTION...");
    } else {
        println!("⚙️  Compiling Oxirast App for DEVELOPMENT...");
    }
    
    // --- TAILWIND CSS AUTO-COMPILER ---
    if Path::new("tailwind.config.js").exists() {
        println!("🎨 Tailwind CSS detected! Compiling styles...");
        let mut tailwind_args = vec!["tailwindcss", "-i", "public/input.css", "-o", "public/style.css"];
        if is_release { tailwind_args.push("--minify"); }
        
        let _ = Command::new("npx").args(&tailwind_args).status();
    }

    let mut cargo_args = vec!["build", "--target", "wasm32-unknown-unknown"];
    if is_release { cargo_args.push("--release"); }

    let build_status = Command::new("cargo").args(&cargo_args).status().expect("Failed to run cargo build");

    if build_status.success() {
        println!("📦 Generating JavaScript bindings...");
        
        let cargo_toml = fs::read_to_string("Cargo.toml").expect("Cargo.toml not found!");
        let mut proj_name = String::new();
        for line in cargo_toml.lines() {
            if line.trim().starts_with("name =") {
                proj_name = line.split('"').nth(1).unwrap_or("").replace("-", "_");
                break;
            }
        }
        
        let target_dir = if is_release { "release" } else { "debug" };
        let wasm_path = format!("target/wasm32-unknown-unknown/{}/{}.wasm", target_dir, proj_name);

        let bindgen_output = Command::new("wasm-bindgen")
            .args(["--out-dir", "dist", "--out-name", "app", "--target", "web", "--no-typescript", &wasm_path ])
            .output()
            .expect("Failed to execute wasm-bindgen tool.");
            
        if bindgen_output.status.success() {
            println!("✅ JavaScript bindings generated in /dist");

            if is_release {
                println!("🗜️ Optimizing WebAssembly binary size (wasm-opt)...");
                let opt_status = Command::new("wasm-opt").args(["-Oz", "-o", "dist/app_bg.wasm", "dist/app_bg.wasm"]).status();
                match opt_status {
                    Ok(s) if s.success() => println!("✅ Wasm optimization complete!"),
                    _ => println!("⚠️  'wasm-opt' not found or failed. Install binaryen for smaller builds."),
                }
            }
        } else {
            println!("❌ wasm-bindgen failed!\n{}", String::from_utf8_lossy(&bindgen_output.stderr));
        }
    } else {
        println!("❌ Cargo build failed. Check your Rust code.");
    }
}

// ==========================================
// CLEAN ENGINE
// ==========================================
fn clean_project() {
    println!("🧹 Cleaning Oxirast project...");
    let _ = Command::new("cargo").arg("clean").status();
    if Path::new("dist").exists() { fs::remove_dir_all("dist").unwrap(); }
    println!("✅ Clean complete.");
}

async fn disable_cache(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    response.headers_mut().insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store, no-cache, must-revalidate"));
    response
}

// ==========================================
// THE SCAFFOLDING ENGINE (With Tailwind Support)
// ==========================================
fn scaffold_project(project_name: &str, template: &str) {
    println!("🚀 Initializing new Oxirast project: {} (Template: {})", project_name, template);

    fs::create_dir_all(format!("{}/src/pages", project_name)).unwrap();
    fs::create_dir_all(format!("{}/public/assets", project_name)).unwrap();

    let cargo_toml = format!(
r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
oxirast-core = "0.1.0"
oxirast-parser = "0.1.0"
wasm-bindgen = "0.2"
serde = {{ version = "1.0", features = ["derive"] }}
"#, project_name);
    fs::write(format!("{}/Cargo.toml", project_name), cargo_toml).unwrap();

    let oxirast_toml = format!(
r#"[project]
name = "{}"
version = "0.1.0"
template = "{}"

[build]
out_dir = "dist"

[server]
port = 3000
"#, project_name, template);
    fs::write(format!("{}/oxirast.toml", project_name), oxirast_toml).unwrap();

    if template == "tailwind" {
        // Generate Tailwind Configuration
        fs::write(format!("{}/tailwind.config.js", project_name), 
            "module.exports = { content: ['./src/**/*.rs', './public/index.html'], theme: { extend: {} }, plugins: [], }"
        ).unwrap();
        
        fs::write(format!("{}/public/input.css", project_name), 
            "@tailwind base;\n@tailwind components;\n@tailwind utilities;\n\nbody { @apply bg-zinc-950 text-white flex items-center justify-center h-screen; }"
        ).unwrap();

        let lib_rs = r#"use oxirast_core::{mount_to_body, render_vnode, VNode};
use oxirast_parser::rsx;

#[allow(non_snake_case)]
pub fn App() -> VNode {
    rsx!(
        <div class="p-8 bg-zinc-900 border border-zinc-800 rounded-2xl shadow-2xl text-center max-w-md">
            <h1 class="text-4xl font-black text-transparent bg-clip-text bg-gradient-to-r from-orange-500 to-red-500 mb-4">"Oxirast + Tailwind"</h1>
            <p class="text-zinc-400 mb-6">"Blistering fast WebAssembly styling."</p>
            <button class="px-6 py-2 bg-orange-500 hover:bg-orange-600 rounded-lg font-bold transition-all">"Get Started"</button>
        </div>
    )
}

#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn main() { mount_to_body(&render_vnode(&App())); }
"#;
        fs::write(format!("{}/src/lib.rs", project_name), lib_rs).unwrap();
    } else {
        // Default Template
        let lib_rs = r#"use oxirast_core::{mount_to_body, render_vnode, VNode};
use oxirast_parser::rsx;

#[allow(non_snake_case)]
pub fn App() -> VNode { rsx!( <div class="container"><h1>"Welcome to Oxirast"</h1></div> ) }

#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn main() { mount_to_body(&render_vnode(&App())); }
"#;
        fs::write(format!("{}/src/lib.rs", project_name), lib_rs).unwrap();
        let index_css = r#"body { font-family: system-ui; background: #090b11; color: white; display: flex; justify-content: center; align-items: center; height: 100vh; }"#;
        fs::write(format!("{}/public/style.css", project_name), index_css).unwrap();
    }

    println!("✅ Project {} created successfully!", project_name);
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() >= 2 {
        match args[1].as_str() {
            "init" => {
                let name = if args.len() >= 3 { &args[2] } else { "oxirast_app" };
                let mut template = "default";
                if args.len() >= 5 && args[3] == "--template" { template = &args[4]; }
                scaffold_project(name, template);
                return;
            }
            "build" => {
                if audit_project() { build_project(true); }
                return;
            }
            "clean" => { clean_project(); return; }
            "audit" => { audit_project(); return; }
            "test"  => { test_project(); return; }
            "serve" | _ => {} 
        }
    }

    let port = get_config_port();
    println!("🔥 Starting Oxirast Dev Server on port {}...", port);

    build_project(false); 

    let (tx, _rx) = broadcast::channel::<String>(100);
    let app_state = Arc::new(tx.clone());

    let watch_dir = Path::new("src");
    if !watch_dir.exists() { std::fs::create_dir_all(watch_dir).unwrap(); }

    tokio::spawn(async move {
        let mut watcher = notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
            if let Ok(event) = res {
                if event.kind.is_modify() {
                    println!("\n📝 Detected file change!");
                    build_project(false);
                    let _ = tx.send("RELOAD".to_string());
                }
            }
        }).unwrap();
        watcher.watch(watch_dir, RecursiveMode::Recursive).unwrap();
        loop { tokio::time::sleep(std::time::Duration::from_secs(1)).await; }
    });

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .nest_service("/dist", ServeDir::new("dist"))
        .nest_service("/public", ServeDir::new("public")) 
        .fallback(get(|| async { Html(INDEX_HTML) }))
        .layer(middleware::from_fn(disable_cache)) 
        .with_state(app_state);

    let addr = format!("127.0.0.1:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    println!("🌐 Server running at http://localhost:{}", port);
    axum::serve(listener, app).await.unwrap();
}

async fn ws_handler(ws: WebSocketUpgrade, axum::extract::State(state): axum::extract::State<Arc<broadcast::Sender<String>>>) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}
async fn handle_socket(mut socket: WebSocket, state: Arc<broadcast::Sender<String>>) {
    let mut rx = state.subscribe();
    while let Ok(msg) = rx.recv().await { if socket.send(Message::Text(msg)).await.is_err() { break; } }
}