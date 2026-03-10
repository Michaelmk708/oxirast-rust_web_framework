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

const INDEX_HTML: &str = r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>Oxirast App</title>
    <link rel="stylesheet" href="/public/style.css">
</head>
<body>
    <div id="root"></div>

    <script type="module">
        // FIX 1: The browser now always looks for 'app.js' regardless of the project name!
        import init from '/dist/app.js';
        
        init().then(() => {
            console.log("🚀 Oxirast Framework Initialized!");
        });

        const ws = new WebSocket("ws://localhost:3000/ws");
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

fn build_project() {
    println!("⚙️  Compiling Oxirast App to WebAssembly...");
    
    // FIX 2: Removed "-p oxirast-demo". It now builds whatever directory it is currently running in.
    let build_status = Command::new("cargo")
        .args(["build", "--target", "wasm32-unknown-unknown"])
        .status()
        .expect("Failed to run cargo build");

    if build_status.success() {
        println!("📦 Generating JavaScript bindings...");
        
        // FIX 3: Read Cargo.toml to dynamically find the project's name
        let cargo_toml = fs::read_to_string("Cargo.toml").expect("Cargo.toml not found! Are you in the right directory?");
        let mut proj_name = String::new();
        for line in cargo_toml.lines() {
            if line.trim().starts_with("name =") {
                // Extracts the name and converts hyphens to underscores (Cargo does this automatically for .wasm files)
                proj_name = line.split('"').nth(1).unwrap_or("").replace("-", "_");
                break;
            }
        }
        
        let wasm_path = format!("target/wasm32-unknown-unknown/debug/{}.wasm", proj_name);

        let bindgen_output = Command::new("wasm-bindgen")
            .args([
                "--out-dir", "dist",
                "--out-name", "app", // FIX 4: Force the output file to be named "app"
                "--target", "web",
                "--no-typescript",
                &wasm_path 
            ])
            .output()
            .expect("Failed to execute wasm-bindgen tool.");
            
        if bindgen_output.status.success() {
            println!("✅ Build complete! JavaScript generated in /dist");
        } else {
            println!("❌ wasm-bindgen failed to generate the files!");
            let error_message = String::from_utf8_lossy(&bindgen_output.stderr);
            println!("Error details:\n{}", error_message);
        }
    } else {
        println!("❌ Cargo build failed. Check your Rust code.");
    }
}

// Middleware to stamp the cache-killing headers on every single file
async fn disable_cache(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store, no-cache, must-revalidate, max-age=0"));
    headers.insert(header::PRAGMA, HeaderValue::from_static("no-cache"));
    headers.insert(header::EXPIRES, HeaderValue::from_static("0"));
    response
}

// ==========================================
// THE SCAFFOLDING ENGINE (oxirast init)
// ==========================================
fn scaffold_project(project_name: &str) {
    println!("🚀 Initializing new Oxirast project: {}", project_name);

    // 1. Create the Directory Tree
    fs::create_dir_all(format!("{}/src/pages", project_name)).unwrap();
    fs::create_dir_all(format!("{}/public/assets", project_name)).unwrap();

    // 2. Generate Cargo.toml
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

    // 3. Generate the Entry Point (src/lib.rs)
    let lib_rs = r#"use oxirast_core::{mount_to_body, render_vnode, VNode};
use oxirast_parser::rsx;

#[allow(non_snake_case)]
pub fn App() -> VNode {
    rsx!(
        <div class="container">
            <h1>"Welcome to Oxirast"</h1>
            <p>"Your WebAssembly framework is ready."</p>
        </div>
    )
}

#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn main() {
    let app = App();
    mount_to_body(&render_vnode(&app));
}
"#;
    fs::write(format!("{}/src/lib.rs", project_name), lib_rs).unwrap();

    // 4. Generate the Public Assets (CSS)
    let index_css = r#"body {
    font-family: system-ui, sans-serif;
    background-color: #f4f4f9;
    color: #333;
    display: flex;
    justify-content: center;
    align-items: center;
    height: 100vh;
    margin: 0;
}
.container {
    text-align: center;
    padding: 2rem;
    background: white;
    border-radius: 12px;
    box-shadow: 0 4px 12px rgba(0,0,0,0.1);
}
h1 { color: #ff4500; }
"#;
    fs::write(format!("{}/public/style.css", project_name), index_css).unwrap();

    println!("✅ Project {} created successfully!", project_name);
    println!("👉 Next steps:\n  cd {}\n  oxirast-cli", project_name);
}


#[tokio::main]
async fn main() {
    // --- THE CLI ROUTER ---
    let args: Vec<String> = env::args().collect();
    
    if args.len() >= 3 && args[1] == "init" {
        scaffold_project(&args[2]);
        return; // Important: Exit the program so the server doesn't start!
    }
    // ----------------------

    println!("🔥 Starting Oxirast Dev Server...");

    build_project();

    let (tx, _rx) = broadcast::channel::<String>(100);
    let app_state = Arc::new(tx.clone());

    // FIX 5: Watch the generic "src" folder, not "oxirast-demo/src"
    let watch_dir = Path::new("src");
    if !watch_dir.exists() {
        std::fs::create_dir_all(watch_dir).unwrap();
    }

    tokio::spawn(async move {
        let mut watcher = notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
            if let Ok(event) = res {
                if event.kind.is_modify() {
                    println!("\n📝 Detected file change!");
                    build_project();
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

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    println!("🌐 Server running at http://localhost:3000");
    axum::serve(listener, app).await.unwrap();
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    axum::extract::State(state): axum::extract::State<Arc<broadcast::Sender<String>>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<broadcast::Sender<String>>) {
    let mut rx = state.subscribe();
    while let Ok(msg) = rx.recv().await {
        if socket.send(Message::Text(msg)).await.is_err() { break; }
    }
}