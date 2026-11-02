use std::collections::HashMap;
use std::fs;
use std::path::Path;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub orchestrator: OrchestratorSection,
    #[serde(default)]
    pub performance: PerformanceSection,
    #[serde(default)]
    pub group: HashMap<String, GroupSection>,
}

#[derive(Debug, Deserialize)]
pub struct OrchestratorSection {
    #[serde(default = "default_host")]
    pub web_host: String,
    #[serde(default = "default_port")]
    pub web_port: u16,
    #[serde(default = "default_secret")]
    pub secret_key: String,
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}
fn default_port() -> u16 {
    8080
}
fn default_secret() -> String {
    "mitosis_default_secret_key_12345".to_string()
}

#[derive(Debug, Deserialize, Default)]
pub struct PerformanceSection {
    pub worker_threads: usize,
    pub memory_limit_mb: usize,
    pub metrics: bool,
}

#[derive(Debug, Deserialize)]
pub struct GroupSection {
    pub server: String,
    pub prefix: String,
    pub count: usize,
    #[serde(default = "default_mode")]
    pub mode: String,
    #[serde(default)]
    pub plugins: Option<Vec<String>>,
}

fn default_mode() -> String {
    "minimal".to_string()
}

impl AppConfig {
    pub fn load_from_file(path: &Path) -> Result<Self, anyhow::Error> {
        let content = fs::read_to_string(path)?;
        let cfg: AppConfig = toml::from_str(&content)?;
        Ok(cfg)
    }
}
