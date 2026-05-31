mod config;
mod postprocess;
mod prompt;
mod provider;
mod setup;

use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Deserialize)]
struct TranslateRequest {
    input: String,
    shell: String,
    cwd: String,
    os: String,
    #[serde(default)]
    history: Vec<String>,
}

#[derive(Debug, Serialize)]
struct TranslateResponse {
    command: Option<String>,
    error: Option<String>,
}

struct AppState {
    llm: Box<dyn provider::LlmProvider>,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] == "setup" {
        setup::run_setup();
        return;
    }

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async_main());
}

async fn async_main() {
    tracing_subscriber::fmt::init();

    let cfg = match config::Config::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Config error: {}", e);
            std::process::exit(1);
        }
    };

    let llm = match provider::build_provider(&cfg) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Provider error: {}", e);
            std::process::exit(1);
        }
    };

    tracing::info!(
        "cmd-engine starting on port {} (provider: {}, model: {})",
        cfg.daemon_port,
        cfg.provider_type,
        cfg.model
    );

    let state = Arc::new(AppState { llm });

    let app = Router::new()
        .route("/translate", post(handle_translate))
        .route("/health", axum::routing::get(handle_health))
        .with_state(state);

    let addr = format!("127.0.0.1:{}", cfg.daemon_port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    tracing::info!("Listening on {}", addr);
    axum::serve(listener, app).await.unwrap();
}

async fn handle_translate(
    State(state): State<Arc<AppState>>,
    Json(req): Json<TranslateRequest>,
) -> (StatusCode, Json<TranslateResponse>) {
    let system_prompt = prompt::build_system_prompt();
    let user_prompt = prompt::build_user_prompt(&req);

    match state.llm.translate(system_prompt, &user_prompt).await {
        Ok(raw) => {
            let cmd = postprocess::postprocess(&raw);
            (
                StatusCode::OK,
                Json(TranslateResponse {
                    command: Some(cmd),
                    error: None,
                }),
            )
        }
        Err(e) => (
            StatusCode::OK,
            Json(TranslateResponse {
                command: None,
                error: Some(e),
            }),
        ),
    }
}

async fn handle_health() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::OK,
        Json(serde_json::json!({"status": "ok"})),
    )
}