use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use notify::{RecursiveMode, Watcher};
use std::{path::Path, process::Command, sync::Arc};
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
    <script type="module">
        import init from '/dist/oxirast_demo.js';
        
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
    
    let build_status = Command::new("cargo")
        .args(["build", "-p", "oxirast-demo", "--target", "wasm32-unknown-unknown"])
        .status()
        .expect("Failed to run cargo build");

    if build_status.success() {
        println!("📦 Generating JavaScript bindings...");
        
        let bindgen_output = Command::new("wasm-bindgen")
            .args([
                "--out-dir", "dist",
                "--target", "web",
                "--no-typescript",
                "target/wasm32-unknown-unknown/debug/oxirast_demo.wasm" 
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

#[tokio::main]
async fn main() {
    println!("🔥 Starting Oxirast Dev Server...");

    build_project();

    let (tx, _rx) = broadcast::channel::<String>(100);
    let app_state = Arc::new(tx.clone());

    let watch_dir = Path::new("oxirast-demo/src");
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

    // Here is the Router safely tucked inside the main function!
    let app = Router::new()
        .route("/", get(|| async { Html(INDEX_HTML) }))
        .route("/ws", get(ws_handler))
        .nest_service("/dist", ServeDir::new("dist"))
        // Exposing the public folder to the browser
        .nest_service("/public", ServeDir::new("public")) 
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