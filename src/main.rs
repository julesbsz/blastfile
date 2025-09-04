use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use axum::body::Body;
use axum::extract::{State, Path};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::Response;
use axum::Router;
use axum::routing::put;
use futures_util::StreamExt;
use regex::Regex;
use tower_http::services::ServeDir;
use tokio::io::AsyncWriteExt;
use axum::response::IntoResponse;

#[derive(Clone)]
struct AppState {
    data_dir: Arc<PathBuf>,
    max_bytes: u64,
    filename_regex: Arc<Regex>,
    public_base_url: Arc<String>,
}

fn normalize_base_url(var: String, bind: SocketAddr) -> String {
    let trimmed = var.trim();
    if trimmed.is_empty() {
        // fallback: http://localhost:<port>
        return format!("http://localhost:{}", bind.port());
    }
    let with_scheme = if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        trimmed.to_string()
    } else {
        format!("https://{}", trimmed)
    };
    with_scheme.trim_end_matches('/').to_string()
}


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let bind_addr: SocketAddr = env::var("BIND")
        .unwrap_or_else(|_| "0.0.0.0:8080".into())
        .parse()
        .expect("Invalid bind address");

    let data_dir = PathBuf::from(env::var("DATA_DIR").unwrap_or_else(|_| "./data".into()));

    let public_base_url = normalize_base_url(
        env::var("PUBLIC_BASE_URL").unwrap_or_default(),
        bind_addr,
    );

    let max_bytes: u64 = env::var("MAX_BYTES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1024 * 1024 * 1024); // 1 GiB by default

    tokio::fs::create_dir_all(&data_dir).await?; // Create upload dir

    let app_state = AppState {
        data_dir: Arc::new(data_dir),
        max_bytes,
        filename_regex: Arc::new(Regex::new(r"^[A-Za-z0-9._-]{1,200}$")?),
        public_base_url: Arc::new(public_base_url),
    };

    let app = Router::new()
        .route("/{filename}", put(upload))
        .nest_service("/files", ServeDir::new(app_state.data_dir.as_ref()))
        .with_state(app_state);

    println!("Listening on http://{bind_addr}");
    let listener = tokio::net::TcpListener::bind(bind_addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn upload(
    State(st): State<AppState>,
    Path(filename): Path<String>,
    headers: HeaderMap,
    body: Body,
) -> Result<Response, (StatusCode, &'static str)> {
    // Sanitize filename
    if !st.filename_regex.is_match(&filename) {
        return Err((StatusCode::BAD_REQUEST, "Invalid filename"));
    }

    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return Err((StatusCode::BAD_REQUEST, "Invalid filename"));
    }

    let dest_path = st.data_dir.join(&filename);

    let mut stream = body.into_data_stream();
    let tmp_path = st.data_dir.join(format!(".{}.part", &filename));
    let mut file = tokio::fs::File::create(&tmp_path).await.map_err(internal_err)?;

    let mut written: u64 = 0;
    while let Some(chunk_res) = stream.next().await {
        let chunk = chunk_res.map_err(|_| (StatusCode::BAD_REQUEST, "Invalid body"))?;
        written += chunk.len() as u64;
        if written > st.max_bytes {
            let _ = tokio::fs::remove_file(&tmp_path).await;
            return Err((StatusCode::PAYLOAD_TOO_LARGE, "File too large"));
        }
        file.write_all(&chunk).await.map_err(internal_err)?;
    }
    file.flush().await.map_err(internal_err)?;
    drop(file);

    tokio::fs::rename(&tmp_path, &dest_path).await.map_err(internal_err)?;

    // Construct wget url
    let scheme = headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("http");

    let host = headers
        .get("x-forwarded-host")
        .or_else(|| headers.get(header::HOST))
        .and_then(|v| v.to_str().ok())
        .unwrap_or("localhost:8080");

    let path = format!("/files/{}", &filename);
    let full_url = format!("{}/files/{}", st.public_base_url, &filename);
    let wget_cmd = format!("wget {}", sh_quote(&full_url));

    // Response
    Ok((
        StatusCode::CREATED,
        [
            (header::LOCATION, HeaderValue::from_str(&path).unwrap()),
            (header::CONTENT_TYPE, HeaderValue::from_static("text/plain; charset=utf-8")),
        ],
        format!(
            "Upload OK\nwget: {wget_cmd}\nsize: {written} bytes\n"
        ),
    ).into_response())
}

fn sh_quote(s: &str) -> String {
    if s.chars().all(|c| c.is_ascii_alphanumeric() || "-_./:@%".contains(c)) {
        s.to_string()
    } else {
        format!("'{}'", s.replace('\'', r"'\''"))
    }
}

// Map internal errors as 500
fn internal_err<E: std::fmt::Debug>(_: E) -> (StatusCode, &'static str) {
    (StatusCode::INTERNAL_SERVER_ERROR, "Internal error")
}