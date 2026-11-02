use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;
use mitosis_identity::IdentityManager;
use mitosis_plugin_host::PluginHost;
use mitosis_proxy::ProxyType;
use mitosis_skins::SkinManager;
use mitosis_world_cache::SharedWorldCache;

use crate::action::BotAction;
use crate::bot::{BotConfig, BotHandle, BotMode, BotRunner};

#[derive(Debug, Clone)]
pub struct GroupConfig {
    pub name: String,
    pub server_host: String,
    pub server_port: u16,
    pub prefix: String,
    pub count: usize,
    pub mode: BotMode,
    pub proxy: ProxyType,
    pub plugins: Vec<String>,
}

pub struct BotGroup {
    pub config: GroupConfig,
    pub bots: HashMap<u32, BotHandle>,
    world: SharedWorldCache,
    identity: IdentityManager,
    plugin_host: Option<Arc<RwLock<PluginHost>>>,
    skin_manager: Option<SkinManager>,
}

impl BotGroup {
    pub fn new(
        config: GroupConfig,
        world: SharedWorldCache,
        identity: IdentityManager,
        plugin_host: Option<Arc<RwLock<PluginHost>>>,
        skin_manager: Option<SkinManager>,
    ) -> Self {
        Self {
            config,
            bots: HashMap::new(),
            world,
            identity,
            plugin_host,
            skin_manager,
        }
    }

    /// Spawns and launches all bots in the group.
    pub fn spawn_all(&mut self) {
        info!("[Group {}] Spawning {} bots...", self.config.name, self.config.count);
        for i in 1..=self.config.count {
            let bot_id = i as u32;
            let internal_id = IdentityManager::make_internal_id(&self.config.name, i);
            let bot_uuid = self.identity.generate_uuid(&internal_id);
            let username = format!("{}{}", self.config.prefix, i);

            let skin = self
                .skin_manager
                .as_ref()
                .and_then(|sm| sm.get_random_skin().ok());

            let bot_cfg = BotConfig {
                id: bot_id,
                internal_id,
                username,
                uuid: bot_uuid,
                server_host: self.config.server_host.clone(),
                server_port: self.config.server_port,
                mode: self.config.mode,
                proxy: self.config.proxy.clone(),
                skin,
            };

            let (runner, handle) = BotRunner::new(
                bot_cfg,
                self.identity.clone(),
                self.world.clone(),
                self.plugin_host.clone(),
            );

            self.bots.insert(bot_id, handle);
            tokio::spawn(runner.run());
        }
    }

    pub async fn send_action_all(&self, action: BotAction) {
        for handle in self.bots.values() {
            let _ = handle.send_action(action.clone()).await;
        }
    }

    pub fn disconnect_all(&self) {
        for handle in self.bots.values() {
            handle.stop();
        }
    }

    pub fn bot_count(&self) -> usize {
        self.bots.len()
    }
}
