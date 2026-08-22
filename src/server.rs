use crate::config::Config;
use anyhow::{Context, Result};
use axum::{
    extract::{DefaultBodyLimit, Multipart},
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::{get, post},
    Router,
};
use qrcode::render::unicode;
use qrcode::QrCode;
use std::fs;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::cors::CorsLayer;

struct ServerState {
    inbox_dir: PathBuf,
}

const HTML_PAGE: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>vj mobile inbox</title>
  <style>
    :root {
      --bg: #0f1117;
      --card: #161b22;
      --border: #30363d;
      --text: #e6edf3;
      --accent: #58a6ff;
      --accent-hover: #1f6feb;
      --success: #238636;
      --muted: #8b949e;
    }
    * { box-sizing: border-box; margin: 0; padding: 0; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; }
    body { background: var(--bg); color: var(--text); padding: 20px; display: flex; justify-content: center; min-height: 100vh; }
    .container { width: 100%; max-width: 500px; }
    header { text-align: center; margin-bottom: 24px; padding-top: 10px; }
    h1 { font-size: 1.8rem; font-weight: 700; color: var(--accent); margin-bottom: 6px; letter-spacing: -0.5px; }
    p.sub { color: var(--muted); font-size: 0.95rem; }
    .dropzone {
      background: var(--card);
      border: 2px dashed var(--border);
      border-radius: 12px;
      padding: 40px 20px;
      text-align: center;
      cursor: pointer;
      transition: all 0.2s ease;
      margin-bottom: 20px;
    }
    .dropzone.dragover { border-color: var(--accent); background: #1c2128; }
    .dropzone svg { width: 48px; height: 48px; fill: var(--accent); margin-bottom: 12px; }
    .btn {
      display: inline-block;
      background: var(--accent);
      color: #ffffff;
      font-weight: 600;
      padding: 12px 24px;
      border-radius: 8px;
      border: none;
      cursor: pointer;
      font-size: 1rem;
      transition: background 0.2s;
      width: 100%;
      margin-top: 8px;
    }
    .btn:hover { background: var(--accent-hover); }
    .btn-camera { background: #21262d; border: 1px solid var(--border); margin-top: 12px; }
    .btn-camera:hover { background: #30363d; }
    #file-input, #camera-input { display: none; }
    .progress-wrap { display: none; margin: 20px 0; background: var(--card); border: 1px solid var(--border); border-radius: 8px; padding: 16px; }
    .progress-bar-bg { background: #21262d; height: 10px; border-radius: 5px; overflow: hidden; margin-top: 8px; }
    .progress-bar { background: var(--accent); height: 100%; width: 0%; transition: width 0.1s linear; }
    .file-list { margin-top: 20px; list-style: none; }
    .file-item { background: var(--card); border: 1px solid var(--border); border-radius: 8px; padding: 12px 16px; margin-bottom: 8px; display: flex; justify-content: space-between; align-items: center; font-size: 0.9rem; }
    .file-item.done { border-color: var(--success); }
    .badge { padding: 4px 8px; border-radius: 4px; font-size: 0.75rem; font-weight: bold; background: #238636; color: white; }
  </style>
</head>
<body>
  <div class="container">
    <header>
      <h1>vj inbox</h1>
      <p class="sub">Upload video recordings directly to your PC</p>
    </header>

    <div class="dropzone" id="dropzone" onclick="document.getElementById('file-input').click()">
      <svg viewBox="0 0 24 24"><path d="M19.35 10.04C18.67 6.59 15.64 4 12 4 9.11 4 6.6 5.64 5.35 8.04 2.34 8.36 0 10.91 0 14c0 3.31 2.69 6 6 6h13c2.76 0 5-2.24 5-5 0-2.64-2.05-4.78-4.65-4.96zM14 13v4h-4v-4H7l5-5 5 5h-3z"/></svg>
      <p style="font-weight: 600; margin-bottom: 4px;">Tap to select or drop video files</p>
      <p style="color: var(--muted); font-size: 0.85rem;">Supports MP4, MOV, MKV, WebM, 3GP</p>
    </div>

    <input type="file" id="file-input" multiple accept="video/*" onchange="handleFiles(this.files)">
    <input type="file" id="camera-input" accept="video/*" capture="environment" onchange="handleFiles(this.files)">

    <button class="btn btn-camera" onclick="document.getElementById('camera-input').click()">📹 Record with Camera</button>

    <div class="progress-wrap" id="progress-wrap">
      <div style="display: flex; justify-content: space-between; font-size: 0.9rem;">
        <span id="progress-status">Uploading...</span>
        <span id="progress-percent">0%</span>
      </div>
      <div class="progress-bar-bg">
        <div class="progress-bar" id="progress-bar"></div>
      </div>
    </div>

    <ul class="file-list" id="file-list"></ul>
  </div>

  <script>
    const dropzone = document.getElementById('dropzone');
    ['dragenter', 'dragover'].forEach(e => dropzone.addEventListener(e, (evt) => { evt.preventDefault(); dropzone.classList.add('dragover'); }));
    ['dragleave', 'drop'].forEach(e => dropzone.addEventListener(e, (evt) => { evt.preventDefault(); dropzone.classList.remove('dragover'); }));
    dropzone.addEventListener('drop', (evt) => { handleFiles(evt.dataTransfer.files); });

    function handleFiles(files) {
      if (!files.length) return;
      const progressWrap = document.getElementById('progress-wrap');
      const progressBar = document.getElementById('progress-bar');
      const progressPercent = document.getElementById('progress-percent');
      const progressStatus = document.getElementById('progress-status');
      const fileList = document.getElementById('file-list');

      progressWrap.style.display = 'block';

      let uploaded = 0;
      const total = files.length;

      Array.from(files).forEach((file, index) => {
        const item = document.createElement('li');
        item.className = 'file-item';
        item.innerHTML = `<span>${file.name} (${(file.size / (1024*1024)).toFixed(1)} MB)</span><span class="badge" style="background: #58a6ff;">Uploading</span>`;
        fileList.prepend(item);

        const xhr = new XMLHttpRequest();
        xhr.open('POST', '/upload', true);

        xhr.upload.onprogress = (e) => {
          if (e.lengthComputable) {
            const pct = Math.round((e.loaded / e.total) * 100);
            progressBar.style.width = pct + '%';
            progressPercent.innerText = pct + '%';
            progressStatus.innerText = `Uploading: ${file.name}`;
          }
        };

        xhr.onload = () => {
          if (xhr.status === 200) {
            item.className = 'file-item done';
            item.querySelector('.badge').style.background = '#238636';
            item.querySelector('.badge').innerText = '✓ In Inbox';
            uploaded++;
            if (uploaded === total) {
              progressStatus.innerText = 'All uploads complete!';
              setTimeout(() => { progressWrap.style.display = 'none'; }, 2000);
            }
          } else {
            item.querySelector('.badge').style.background = '#da3633';
            item.querySelector('.badge').innerText = 'Failed';
          }
        };

        xhr.onerror = () => {
          item.querySelector('.badge').style.background = '#da3633';
          item.querySelector('.badge').innerText = 'Error';
        };

        const formData = new FormData();
        formData.append('files', file);
        xhr.send(formData);
      });
    }
  </script>
</body>
</html>"#;

async fn index_handler() -> Html<&'static str> {
    Html(HTML_PAGE)
}

async fn upload_handler(
    state: axum::extract::State<Arc<ServerState>>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, StatusCode> {
    while let Ok(Some(field)) = multipart.next_field().await {
        let filename = field
            .file_name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("upload_{}.mp4", chrono::Utc::now().timestamp()));

        let clean_filename = std::path::Path::new(&filename)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or(filename);

        let out_path = state.inbox_dir.join(&clean_filename);

        if let Ok(data) = field.bytes().await {
            let size_mb = data.len() as f64 / (1024.0 * 1024.0);
            if fs::write(&out_path, &data).is_ok() {
                println!(
                    "  [✓ UPLOADED] {} ({:.1} MB) -> inbox/",
                    clean_filename, size_mb
                );
            }
        }
    }

    Ok((StatusCode::OK, "OK"))
}

pub async fn run_inbox_server(port_opt: Option<u16>, config: &Config) -> Result<()> {
    config.ensure_directories()?;
    let port = port_opt.unwrap_or(config.inbox_port);
    let inbox_dir = config.inbox_path();

    let local_ip = local_ip_address::local_ip()
        .map(|ip| ip.to_string())
        .unwrap_or_else(|_| "127.0.0.1".to_string());

    let server_url = format!("http://{}:{}", local_ip, port);

    println!("========================================================");
    println!("  vj Mobile Inbox Server");
    println!("========================================================");
    println!("  URL:       {}", server_url);
    println!("  Inbox:     {}", inbox_dir.display());
    println!();
    println!("  Scan QR code with your phone camera to upload videos:");
    println!("--------------------------------------------------------");

    if let Ok(code) = QrCode::new(server_url.as_bytes()) {
        let image = code
            .render::<unicode::Dense1x2>()
            .dark_color(unicode::Dense1x2::Dark)
            .light_color(unicode::Dense1x2::Light)
            .build();
        println!("{}", image);
    }

    println!("--------------------------------------------------------");
    println!("Press Ctrl-C to stop the server.");
    println!();

    let state = Arc::new(ServerState { inbox_dir });

    let app = Router::new()
        .route("/", get(index_handler))
        .route("/upload", post(upload_handler))
        .layer(DefaultBodyLimit::max(5 * 1024 * 1024 * 1024)) // 5 GB limit
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("Failed to bind server on 0.0.0.0:{}", port))?;

    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c().await.ok();
            println!("\nStopping inbox server.");
        })
        .await
        .context("Server error occurred")?;

    Ok(())
}
