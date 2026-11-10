use apolo_ax_scheduler::AxScheduler;
use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::get,
    Json, Router,
};
use serde::Serialize;
use std::{env, net::SocketAddr, sync::Arc};

#[derive(Clone)]
struct AppState {
    scheduler: Arc<AxScheduler>,
}

#[derive(Serialize)]
struct Overview {
    service: &'static str,
    status: &'static str,
    control_plane: &'static str,
    persistence: &'static str,
    node_count: usize,
}

async fn dashboard() -> Html<&'static str> {
    Html(include_str!("dashboard.html"))
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "service": "apolo-ax-api",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

async fn overview(State(state): State<AppState>) -> Result<Json<Overview>, StatusCode> {
    let nodes = state.scheduler.list_nodes().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(Overview {
        service: "APOLO DATA CENTER",
        status: "operational",
        control_plane: "single_process",
        persistence: "volatile_memory",
        node_count: nodes.len(),
    }))
}

async fn nodes(State(state): State<AppState>) -> Result<Json<Vec<apolo_ax_core::NodeInventory>>, StatusCode> {
    state.scheduler.list_nodes()
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

fn app(state: AppState) -> Router {
    Router::new()
        .route("/", get(dashboard))
        .route("/healthz", get(health))
        .route("/api/v1/overview", get(overview))
        .route("/api/v1/nodes", get(nodes))
        .with_state(state)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bind = env::var("APOLO_BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".into());
    let address: SocketAddr = bind.parse()?;
    let state = AppState { scheduler: Arc::new(AxScheduler::new()) };
    let listener = tokio::net::TcpListener::bind(address).await?;
    println!("APOLO AX API listening on {address}");
    axum::serve(listener, app(state))
        .with_graceful_shutdown(async { let _ = tokio::signal::ctrl_c().await; })
        .await?;
    Ok(())
}
