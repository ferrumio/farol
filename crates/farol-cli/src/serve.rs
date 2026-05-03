use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use axum::{
    body::Body,
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    http::{header, StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use farol_core::{build, BuildOptions, Config};
use futures_util::stream::StreamExt;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use tokio::{
    sync::{broadcast, Mutex},
    time::sleep,
};

const RELOAD_PATH: &str = "/__farol/reload";
const DEBOUNCE_MS: u64 = 100;

/// Run the dev server until the user hits Ctrl-C.
pub async fn run(
    config: Config,
    project_root: PathBuf,
    port: u16,
    host: String,
) -> miette::Result<()> {
    let site_dir = project_root.join(&config.site_dir);

    // Shared state.
    let (reload_tx, _reload_rx) = broadcast::channel::<()>(16);
    let last_error: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));

    // Initial build so the site exists before requests arrive.
    if let Err(e) = do_build(&config, &project_root, &last_error).await {
        tracing::warn!(error = %e, "initial build failed - server will show overlay");
    }

    // File watcher in its own blocking thread (notify callback is sync).
    spawn_watcher(&project_root, &config, reload_tx.clone(), last_error.clone())?;

    // Router.
    let state = AppState { reload_tx: reload_tx.clone(), last_error: last_error.clone(), site_dir };
    let app = Router::new()
        .route(RELOAD_PATH, get(ws_handler))
        .fallback(static_handler)
        .with_state(state);

    let addr: SocketAddr =
        format!("{host}:{port}").parse().map_err(|e| miette::miette!("invalid host/port: {e}"))?;
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| miette::miette!("bind {addr}: {e}"))?;

    print_banner(&addr);

    let shutdown = async {
        let _ = tokio::signal::ctrl_c().await;
        println!("\nshutting down.");
    };

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await
        .map_err(|e| miette::miette!("server error: {e}"))?;

    Ok(())
}

#[derive(Clone)]
struct AppState {
    reload_tx: broadcast::Sender<()>,
    last_error: Arc<Mutex<Option<String>>>,
    site_dir: PathBuf,
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state.reload_tx))
}

async fn handle_socket(mut socket: WebSocket, reload_tx: broadcast::Sender<()>) {
    let mut rx = reload_tx.subscribe();
    loop {
        tokio::select! {
            res = rx.recv() => {
                match res {
                    Ok(()) => {
                        if socket.send(Message::Text("reload".into())).await.is_err() {
                            return;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => return,
                }
            }
            msg = socket.next() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => return,
                    _ => {}
                }
            }
        }
    }
}

async fn static_handler(State(state): State<AppState>, uri: Uri) -> Response {
    // If the last build failed, always show the overlay page.
    if let Some(msg) = state.last_error.lock().await.clone() {
        return error_overlay(&msg).into_response();
    }

    let rel = uri.path().trim_start_matches('/');
    let safe = sanitize(rel);
    let is_root = safe.as_os_str().is_empty();
    let mut path = state.site_dir.join(&safe);

    // Directory -> index.html.
    if path.is_dir() || is_root {
        path = path.join("index.html");
    }

    let bytes = match tokio::fs::read(&path).await {
        Ok(b) => b,
        Err(_) => return (StatusCode::NOT_FOUND, "not found").into_response(),
    };

    let mime = mime_type(&path);
    if mime.starts_with("text/html") {
        let text = String::from_utf8_lossy(&bytes).into_owned();
        let injected = inject_reload_script(&text);
        ([(header::CONTENT_TYPE, mime)], Body::from(injected)).into_response()
    } else {
        ([(header::CONTENT_TYPE, mime)], Body::from(bytes)).into_response()
    }
}

fn sanitize(rel: &str) -> PathBuf {
    // Reject any `..` segments to avoid escaping the site dir.
    let mut out = PathBuf::new();
    for part in rel.split('/') {
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." {
            continue;
        }
        out.push(part);
    }
    out
}

fn mime_type(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase).as_deref() {
        Some("html") | Some("htm") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") | Some("mjs") => "text/javascript; charset=utf-8",
        Some("json") => "application/json",
        Some("xml") => "application/xml",
        Some("txt") | Some("md") => "text/plain; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("ico") => "image/x-icon",
        Some("woff") => "font/woff",
        Some("woff2") => "font/woff2",
        _ => "application/octet-stream",
    }
}

fn inject_reload_script(html: &str) -> String {
    let script = format!(
        r#"<script>(function(){{try{{var ws=new WebSocket((location.protocol=='https:'?'wss://':'ws://')+location.host+'{RELOAD_PATH}');ws.onmessage=function(e){{if(e.data=='reload')location.reload();}};ws.onclose=function(){{setTimeout(function(){{location.reload();}},1000);}};}}catch(e){{}}}})();</script>"#
    );
    if let Some(idx) = html.rfind("</body>") {
        let mut out = String::with_capacity(html.len() + script.len());
        out.push_str(&html[..idx]);
        out.push_str(&script);
        out.push_str(&html[idx..]);
        out
    } else {
        let mut out = html.to_string();
        out.push_str(&script);
        out
    }
}

fn error_overlay(message: &str) -> impl IntoResponse {
    let escaped = message.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
    let body = format!(
        r#"<!doctype html><html><head><meta charset="utf-8"><title>farol: build error</title>
<style>
body{{margin:0;font:14px/1.5 ui-monospace,Menlo,monospace;background:#1a0f0f;color:#ffdede;padding:2rem}}
h1{{color:#ff8b8b;font:600 18px/1.2 system-ui;margin:0 0 1rem}}
pre{{white-space:pre-wrap;background:#2a1616;padding:1rem;border-radius:6px;border-left:3px solid #ff5050}}
</style></head><body>
<h1>farol: build failed</h1>
<pre>{escaped}</pre>
<script>(function(){{try{{var ws=new WebSocket((location.protocol=='https:'?'wss://':'ws://')+location.host+'{RELOAD_PATH}');ws.onmessage=function(e){{if(e.data=='reload')location.reload();}};ws.onclose=function(){{setTimeout(function(){{location.reload();}},1000);}};}}catch(e){{}}}})();</script>
</body></html>"#
    );
    (StatusCode::OK, [(header::CONTENT_TYPE, "text/html; charset=utf-8")], body)
}

fn spawn_watcher(
    project_root: &Path,
    config: &Config,
    reload_tx: broadcast::Sender<()>,
    last_error: Arc<Mutex<Option<String>>>,
) -> miette::Result<()> {
    let docs_dir = project_root.join(&config.docs_dir);
    let overrides_dir = project_root.join("overrides");
    let config_path = project_root.join("farol.toml");
    let site_dir_abs = project_root.join(&config.site_dir);
    let project_root = project_root.to_path_buf();
    let config = config.clone();

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<()>();

    let mut watcher: RecommendedWatcher =
        notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            if let Ok(event) = res {
                // Ignore anything inside the output site_dir.
                let skip = event.paths.iter().all(|p| p.starts_with(&site_dir_abs));
                if !skip {
                    let _ = tx.send(());
                }
            }
        })
        .map_err(|e| miette::miette!("watcher init: {e}"))?;

    if docs_dir.exists() {
        watcher
            .watch(&docs_dir, RecursiveMode::Recursive)
            .map_err(|e| miette::miette!("watch {}: {e}", docs_dir.display()))?;
    }
    if overrides_dir.exists() {
        let _ = watcher.watch(&overrides_dir, RecursiveMode::Recursive);
    }
    if config_path.exists() {
        let _ = watcher.watch(&config_path, RecursiveMode::NonRecursive);
    }

    // Keep watcher alive on the Tokio runtime by stashing it in a task.
    tokio::spawn(async move {
        let _keep_alive = watcher;
        let debounce = Duration::from_millis(DEBOUNCE_MS);
        while rx.recv().await.is_some() {
            // Drain further events that arrive during the debounce window.
            let deadline = Instant::now() + debounce;
            while Instant::now() < deadline {
                sleep(debounce).await;
                if rx.try_recv().is_err() {
                    break;
                }
            }

            let started = Instant::now();
            let build_result = do_build(&config, &project_root, &last_error).await;
            match build_result {
                Ok(report) => {
                    println!(
                        "rebuilt {} pages in {:.0?} ({} hit, {} miss)",
                        report.pages,
                        started.elapsed(),
                        report.graph.as_ref().map(|g| g.cache_hits).unwrap_or(0),
                        report.graph.as_ref().map(|g| g.cache_misses).unwrap_or(0),
                    );
                }
                Err(e) => {
                    println!("build failed: {e}");
                }
            }
            let _ = reload_tx.send(());
        }
    });

    Ok(())
}

async fn do_build(
    config: &Config,
    project_root: &Path,
    last_error: &Arc<Mutex<Option<String>>>,
) -> miette::Result<farol_core::BuildReport> {
    let config = config.clone();
    let project_root = project_root.to_path_buf();
    let opts = BuildOptions { timings: true, ..BuildOptions::default() };

    // Run the (blocking, rayon-heavy) build off the async runtime.
    let handle =
        tokio::task::spawn_blocking(move || build::build_with(&config, &project_root, &opts));
    let result = handle.await.map_err(|e| miette::miette!("build join: {e}"))?;

    match result {
        Ok(report) => {
            *last_error.lock().await = None;
            Ok(report)
        }
        Err(e) => {
            let msg = format!("{e:?}");
            *last_error.lock().await = Some(msg.clone());
            Err(miette::miette!(msg))
        }
    }
}

fn print_banner(addr: &SocketAddr) {
    println!();
    println!("   ╔═╗");
    println!("   ║ ║  ◦ ◦ ◦ ◦");
    println!("   ║ ║");
    println!("  ╔╩═╩╗");
    println!("  ║   ║         farol dev server");
    println!(" ╔╝   ╚╗        serving at http://{addr}");
    println!(" ╚═════╝        press Ctrl-C to stop");
    println!();
}
