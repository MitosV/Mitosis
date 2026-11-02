use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tracing::info;

use mitosis_benchmarking::{BenchmarkRunner, BenchmarkScenario};
use mitosis_bot_runtime::{BotAction, BotGroup};
use mitosis_metrics::MetricsCollector;
use mitosis_plugin_host::PluginHost;
use mitosis_skins::SkinManager;

#[derive(Clone)]
pub struct AppState {
    pub metrics: MetricsCollector,
    pub groups: Arc<RwLock<HashMap<String, BotGroup>>>,
    pub plugin_host: Arc<RwLock<PluginHost>>,
    pub skin_manager: SkinManager,
    pub benchmark_runner: Arc<BenchmarkRunner>,
}

#[derive(Debug, Deserialize)]
pub struct PluginActionRequest {
    pub bot_id: u32,
    pub action: String,
    pub args: String,
}

#[derive(Debug, Deserialize)]
pub struct GroupActionRequest {
    pub action: BotAction,
}

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/", get(serve_dashboard))
        .route("/api/v1/status", get(get_status))
        .route("/api/v1/metrics", get(get_metrics))
        .route("/api/v1/plugins", get(get_plugins))
        .route("/api/v1/plugins/{id}/action", post(execute_plugin_action))
        .route("/api/v1/skins", get(get_skins))
        .route("/api/v1/groups/{name}/action", post(send_group_action))
        .route("/api/v1/groups/{name}/disconnect", post(disconnect_group))
        .route("/api/v1/benchmarks/run", post(run_benchmark))
        .route("/ws/telemetry", get(ws_telemetry_handler))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

pub async fn start_server(addr: SocketAddr, state: AppState) -> Result<(), std::io::Error> {
    let app = create_router(state);
    info!("[Web API] Listening on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await
}

// ---------------------------------------------------------------------------
// Route Handlers
// ---------------------------------------------------------------------------

async fn get_status(State(state): State<AppState>) -> impl IntoResponse {
    let groups = state.groups.read().await;
    let mut total_bots = 0;
    for g in groups.values() {
        total_bots += g.bot_count();
    }
    let snap = state.metrics.snapshot(total_bots, total_bots);
    Json(snap)
}

async fn get_metrics(State(state): State<AppState>) -> impl IntoResponse {
    let groups = state.groups.read().await;
    let total_bots: usize = groups.values().map(|g| g.bot_count()).sum();
    let text = state.metrics.to_prometheus(total_bots, total_bots);
    Response::builder()
        .header("content-type", "text/plain; version=0.0.4")
        .body(text)
        .unwrap()
}

async fn get_plugins(State(state): State<AppState>) -> impl IntoResponse {
    let host = state.plugin_host.read().await;
    let plugins = host.list_plugins();
    let actions = host.get_registered_actions();
    Json(serde_json::json!({
        "plugins": plugins,
        "actions": actions
    }))
}

async fn execute_plugin_action(
    Path(id): Path<String>,
    State(state): State<AppState>,
    Json(req): Json<PluginActionRequest>,
) -> impl IntoResponse {
    let host = state.plugin_host.read().await;
    match host.execute_action(&id, req.bot_id, &req.action, &req.args) {
        Ok(code) => Json(serde_json::json!({ "success": code == 0, "exit_code": code })),
        Err(e) => Json(serde_json::json!({ "success": false, "error": e.to_string() })),
    }
}

async fn get_skins(State(state): State<AppState>) -> impl IntoResponse {
    let skins = state.skin_manager.list_skins();
    Json(skins)
}

async fn send_group_action(
    Path(name): Path<String>,
    State(state): State<AppState>,
    Json(req): Json<GroupActionRequest>,
) -> impl IntoResponse {
    let groups = state.groups.read().await;
    if let Some(group) = groups.get(&name) {
        group.send_action_all(req.action).await;
        Json(serde_json::json!({ "success": true }))
    } else {
        Json(serde_json::json!({ "success": false, "error": "Group not found" }))
    }
}

async fn disconnect_group(
    Path(name): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let groups = state.groups.read().await;
    if let Some(group) = groups.get(&name) {
        group.disconnect_all();
        Json(serde_json::json!({ "success": true }))
    } else {
        Json(serde_json::json!({ "success": false, "error": "Group not found" }))
    }
}

async fn run_benchmark(
    State(state): State<AppState>,
    Json(scenario): Json<BenchmarkScenario>,
) -> impl IntoResponse {
    let groups = state.groups.read().await;
    if let Some(group) = groups.values().next() {
        let result = state.benchmark_runner.run_scenario(scenario, group).await;
        Json(serde_json::json!({ "success": true, "result": result }))
    } else {
        Json(serde_json::json!({ "success": false, "error": "No active bot group to benchmark" }))
    }
}

async fn ws_telemetry_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_ws(socket, state))
}

async fn handle_ws(mut socket: WebSocket, state: AppState) {
    let mut interval = tokio::time::interval(Duration::from_millis(1000));
    loop {
        interval.tick().await;
        let groups = state.groups.read().await;
        let total_bots: usize = groups.values().map(|g| g.bot_count()).sum();
        let snap = state.metrics.snapshot(total_bots, total_bots);

        if let Ok(json) = serde_json::to_string(&snap) {
            if socket.send(Message::Text(json.into())).await.is_err() {
                break;
            }
        } else {
            break;
        }
    }
}

async fn serve_dashboard() -> Html<&'static str> {
    Html(include_str!("../../../web/index.html"))
}
