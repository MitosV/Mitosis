mod config;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use anyhow::Result;
use tokio::sync::RwLock;
use tracing::info;

use config::AppConfig;
use mitosis_benchmarking::BenchmarkRunner;
use mitosis_bot_runtime::{BotGroup, BotMode, GroupConfig};
use mitosis_identity::IdentityManager;
use mitosis_metrics::MetricsCollector;
use mitosis_plugin_host::PluginHost;
use mitosis_proxy::ProxyType;
use mitosis_skins::SkinManager;
use mitosis_web_api::{start_server, AppState};
use mitosis_world_cache::SharedWorldCache;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,mitosis_orchestrator=debug".into()),
        )
        .init();

    info!("============================================================");
    info!("  MITOSIS — Minecraft Headless Bot Engine");
    info!("  Author: MitosV | Target: Minecraft 1.21.1 (Protocol 767)");
    info!("============================================================");

    let config_path = Path::new("config.toml");
    let cfg = if config_path.exists() {
        AppConfig::load_from_file(config_path)?
    } else {
        info!("config.toml not found, creating default...");
        let default_cfg = AppConfig {
            orchestrator: config::OrchestratorSection {
                web_host: "0.0.0.0".into(),
                web_port: 8080,
                secret_key: "mitosis_secret_key_change_me".into(),
            },
            performance: config::PerformanceSection {
                worker_threads: 0,
                memory_limit_mb: 4096,
                metrics: true,
            },
            group: HashMap::new(),
        };
        default_cfg
    };

    info!(
        "Performance limits: {} MB RAM, {} worker threads, metrics={}",
        cfg.performance.memory_limit_mb, cfg.performance.worker_threads, cfg.performance.metrics
    );
    let identity = IdentityManager::with_default_namespace(cfg.orchestrator.secret_key.as_bytes());
    let world_cache = SharedWorldCache::new("minecraft:overworld");
    let skin_manager = SkinManager::new("skins");
    let metrics = MetricsCollector::new();
    let benchmark_runner = Arc::new(BenchmarkRunner::new(metrics.clone()));

    let mut plugin_host = PluginHost::new()?;
    let plugins_dir = Path::new("plugins");
    if plugins_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(plugins_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("wasm") {
                    info!("Loading WebAssembly plugin: {:?}", path);
                    match plugin_host.load_plugin(&path) {
                        Ok(meta) => info!("Loaded plugin '{}' ({}) by {:?}", meta.name, meta.version, meta.authors),
                        Err(e) => tracing::warn!("Failed to load plugin {:?}: {}", path, e),
                    }
                }
            }
        }
    }
    let plugin_host = Arc::new(RwLock::new(plugin_host));

    let groups_map = Arc::new(RwLock::new(HashMap::new()));

    for (group_name, group_sec) in &cfg.group {
        let (host, port) = parse_address(&group_sec.server);
        let mode = match group_sec.mode.to_lowercase().as_str() {
            "fidelity" => BotMode::Fidelity,
            _ => BotMode::Minimal,
        };

        let group_cfg = GroupConfig {
            name: group_name.clone(),
            server_host: host,
            server_port: port,
            prefix: group_sec.prefix.clone(),
            count: group_sec.count,
            mode,
            proxy: ProxyType::None,
            plugins: group_sec.plugins.clone().unwrap_or_default(),
        };

        let mut group = BotGroup::new(
            group_cfg,
            world_cache.clone(),
            identity.clone(),
            Some(plugin_host.clone()),
            Some(skin_manager.clone()),
        );

        group.spawn_all();
        groups_map.write().await.insert(group_name.clone(), group);
    }

    let app_state = AppState {
        metrics,
        groups: groups_map.clone(),
        plugin_host,
        skin_manager,
        benchmark_runner,
    };

    let bind_addr: SocketAddr = format!("{}:{}", cfg.orchestrator.web_host, cfg.orchestrator.web_port).parse()?;
    tokio::spawn(start_server(bind_addr, app_state));

    info!("Mitosis Orchestrator is operational. Press Ctrl+C to stop.");
    tokio::signal::ctrl_c().await?;

    info!("Gracefully shutting down Mitosis Orchestrator...");
    let groups = groups_map.read().await;
    for group in groups.values() {
        group.disconnect_all();
    }
    info!("All bots disconnected. Goodbye!");

    Ok(())
}

fn parse_address(addr: &str) -> (String, u16) {
    if let Some((h, p)) = addr.split_once(':') {
        (h.to_string(), p.parse().unwrap_or(25565))
    } else {
        (addr.to_string(), 25565)
    }
}
