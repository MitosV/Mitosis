use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use bytes::{Bytes, BytesMut};
use tokio::sync::{mpsc, RwLock};
use tokio::time::interval;
use tracing::{error, info, warn};
use uuid::Uuid;

use mitosis_identity::IdentityManager;
use mitosis_networking::{BackoffConfig, ExponentialBackoff, MinecraftConnection};
use mitosis_physics::{PhysicsState, PlayerInput};
use mitosis_plugin_host::PluginHost;
use mitosis_protocol::configuration as cfg_pkt;
use mitosis_protocol::handshake::*;
use mitosis_protocol::login::*;
use mitosis_protocol::play as play_pkt;
use mitosis_protocol::varint::ProtocolError;
use mitosis_protocol::{RawPacket, State};
use mitosis_proxy::{ProxyConnector, ProxyType};
use mitosis_skins::SkinInfo;
use mitosis_world_cache::SharedWorldCache;

use crate::action::BotAction;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BotMode {
    Fidelity,
    Minimal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BotStatus {
    Disconnected,
    Connecting,
    Handshaking,
    LoggingIn,
    Configuring,
    Playing,
    Reconnecting,
    Failed,
}

pub struct BotConfig {
    pub id: u32,
    pub internal_id: String,
    pub username: String,
    pub uuid: Uuid,
    pub server_host: String,
    pub server_port: u16,
    pub mode: BotMode,
    pub proxy: ProxyType,
    pub skin: Option<SkinInfo>,
}

pub struct BotHandle {
    pub config: BotConfig,
    pub status: Arc<RwLock<BotStatus>>,
    pub physics: Arc<RwLock<PhysicsState>>,
    pub action_tx: mpsc::Sender<BotAction>,
    pub running: Arc<AtomicBool>,
}

impl BotHandle {
    pub async fn send_action(&self, action: BotAction) -> Result<(), mpsc::error::SendError<BotAction>> {
        self.action_tx.send(action).await
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

pub struct BotRunner {
    config: BotConfig,
    identity: IdentityManager,
    world: SharedWorldCache,
    plugin_host: Option<Arc<RwLock<PluginHost>>>,
    status: Arc<RwLock<BotStatus>>,
    physics: Arc<RwLock<PhysicsState>>,
    action_rx: mpsc::Receiver<BotAction>,
    running: Arc<AtomicBool>,
    input: PlayerInput,
}

impl BotRunner {
    pub fn new(
        config: BotConfig,
        identity: IdentityManager,
        world: SharedWorldCache,
        plugin_host: Option<Arc<RwLock<PluginHost>>>,
    ) -> (Self, BotHandle) {
        let (action_tx, action_rx) = mpsc::channel(64);
        let status = Arc::new(RwLock::new(BotStatus::Disconnected));
        let physics = Arc::new(RwLock::new(PhysicsState::new(0.0, 64.0, 0.0)));
        let running = Arc::new(AtomicBool::new(true));

        let handle = BotHandle {
            config: BotConfig {
                id: config.id,
                internal_id: config.internal_id.clone(),
                username: config.username.clone(),
                uuid: config.uuid,
                server_host: config.server_host.clone(),
                server_port: config.server_port,
                mode: config.mode,
                proxy: config.proxy.clone(),
                skin: config.skin.clone(),
            },
            status: status.clone(),
            physics: physics.clone(),
            action_tx,
            running: running.clone(),
        };

        let runner = Self {
            config,
            identity,
            world,
            plugin_host,
            status,
            physics,
            action_rx,
            running,
            input: PlayerInput::default(),
        };

        (runner, handle)
    }

    pub async fn run(mut self) {
        let mut backoff = ExponentialBackoff::new(BackoffConfig::default());

        while self.running.load(Ordering::SeqCst) {
            *self.status.write().await = BotStatus::Connecting;
            info!("[Bot {}] Connecting to {}:{}", self.config.username, self.config.server_host, self.config.server_port);

            match ProxyConnector::connect(
                &self.config.proxy,
                &self.config.server_host,
                self.config.server_port,
            ).await {
                Ok(stream) => {
                    backoff.reset();
                    let conn = MinecraftConnection::new(stream);
                    if let Err(e) = self.session_loop(conn).await {
                        warn!("[Bot {}] Session ended: {}", self.config.username, e);
                    }
                }
                Err(e) => {
                    error!("[Bot {}] Connection failed: {}", self.config.username, e);
                }
            }

            if !self.running.load(Ordering::SeqCst) {
                break;
            }

            *self.status.write().await = BotStatus::Reconnecting;
            let delay = backoff.next_delay();
            info!("[Bot {}] Reconnecting in {:?}", self.config.username, delay);
            tokio::time::sleep(delay).await;
        }

        *self.status.write().await = BotStatus::Disconnected;
        info!("[Bot {}] Terminated.", self.config.username);
    }

    async fn session_loop(&mut self, mut conn: MinecraftConnection) -> Result<(), ProtocolError> {
        // 1. Handshake
        *self.status.write().await = BotStatus::Handshaking;
        let mut hs_buf = BytesMut::new();
        ClientIntentionPacket::new(&self.config.server_host, self.config.server_port, 2).write(&mut hs_buf);
        conn.write_packet(&RawPacket::new(ClientIntentionPacket::PACKET_ID, hs_buf.freeze())).await?;
        conn.state = State::Login;

        // 2. Login Start
        *self.status.write().await = BotStatus::LoggingIn;
        let mut login_buf = BytesMut::new();
        ServerboundHelloPacket {
            name: self.config.username.clone(),
            profile_id: self.config.uuid,
        }.write(&mut login_buf);
        conn.write_packet(&RawPacket::new(ServerboundHelloPacket::PACKET_ID, login_buf.freeze())).await?;

        // 3. Login Handshake Loop
        while conn.state == State::Login && self.running.load(Ordering::SeqCst) {
            let packet = conn.read_packet().await?;
            match packet.id {
                ClientboundHelloPacket::PACKET_ID => {
                    let mut key_buf = BytesMut::new();
                    ServerboundKeyPacket {
                        shared_secret: Bytes::from_static(&[0x01; 16]),
                        verify_token: Bytes::from_static(&[0x02; 4]),
                    }.write(&mut key_buf);
                    conn.write_packet(&RawPacket::new(ServerboundKeyPacket::PACKET_ID, key_buf.freeze())).await?;
                }
                ClientboundLoginCompressionPacket::PACKET_ID => {
                    let comp = ClientboundLoginCompressionPacket::read(&mut std::io::Cursor::new(packet.data))?;
                    conn.enable_compression(comp.threshold);
                }
                ClientboundCustomQueryPacket::PACKET_ID => {
                    let query = ClientboundCustomQueryPacket::read(&mut std::io::Cursor::new(packet.data))?;
                    if query.channel == "mitosis:auth" {
                        let token = self.identity.generate_auth_token(&self.config.uuid, &self.config.username, 300);
                        let mut resp_buf = BytesMut::new();
                        ServerboundCustomQueryAnswerPacket {
                            message_id: query.message_id,
                            successful: true,
                            data: Some(Bytes::copy_from_slice(token.as_bytes())),
                        }.write(&mut resp_buf);
                        conn.write_packet(&RawPacket::new(ServerboundCustomQueryAnswerPacket::PACKET_ID, resp_buf.freeze())).await?;
                    } else {
                        let mut resp_buf = BytesMut::new();
                        ServerboundCustomQueryAnswerPacket {
                            message_id: query.message_id,
                            successful: false,
                            data: None,
                        }.write(&mut resp_buf);
                        conn.write_packet(&RawPacket::new(ServerboundCustomQueryAnswerPacket::PACKET_ID, resp_buf.freeze())).await?;
                    }
                }
                ClientboundGameProfilePacket::PACKET_ID => {
                    let _profile = ClientboundGameProfilePacket::read(&mut std::io::Cursor::new(packet.data))?;
                    let mut ack_buf = BytesMut::new();
                    ServerboundLoginAcknowledgedPacket.write(&mut ack_buf);
                    conn.write_packet(&RawPacket::new(ServerboundLoginAcknowledgedPacket::PACKET_ID, ack_buf.freeze())).await?;
                    conn.state = State::Configuration;
                    *self.status.write().await = BotStatus::Configuring;
                    info!("[Bot {}] Logged in successfully. Switched to Configuration state.", self.config.username);
                }
                ClientboundLoginDisconnectPacket::PACKET_ID => {
                    let disc = ClientboundLoginDisconnectPacket::read(&mut std::io::Cursor::new(packet.data))?;
                    return Err(ProtocolError::Custom(format!("Kicked during login: {}", disc.reason)));
                }
                _ => {}
            }
        }

        // 4. Configuration Loop
        while conn.state == State::Configuration && self.running.load(Ordering::SeqCst) {
            let packet = conn.read_packet().await?;
            match packet.id {
                cfg_pkt::ClientboundSelectKnownPacksPacket::PACKET_ID => {
                    let mut packs_buf = BytesMut::new();
                    cfg_pkt::ServerboundSelectKnownPacksPacket { known_packs: Vec::new() }.write(&mut packs_buf);
                    conn.write_packet(&RawPacket::new(cfg_pkt::ServerboundSelectKnownPacksPacket::PACKET_ID, packs_buf.freeze())).await?;

                    let mut client_info_buf = BytesMut::new();
                    cfg_pkt::ServerboundClientInformationPacket::default().write(&mut client_info_buf);
                    conn.write_packet(&RawPacket::new(cfg_pkt::ServerboundClientInformationPacket::PACKET_ID, client_info_buf.freeze())).await?;
                }
                cfg_pkt::ClientboundKeepAlivePacket::PACKET_ID => {
                    let keep = cfg_pkt::ClientboundKeepAlivePacket::read(&mut std::io::Cursor::new(packet.data))?;
                    let mut buf = BytesMut::new();
                    cfg_pkt::ServerboundKeepAlivePacket { id: keep.id }.write(&mut buf);
                    conn.write_packet(&RawPacket::new(cfg_pkt::ServerboundKeepAlivePacket::PACKET_ID, buf.freeze())).await?;
                }
                cfg_pkt::ClientboundPingPacket::PACKET_ID => {
                    let ping = cfg_pkt::ClientboundPingPacket::read(&mut std::io::Cursor::new(packet.data))?;
                    let mut buf = BytesMut::new();
                    cfg_pkt::ServerboundPongPacket { id: ping.id }.write(&mut buf);
                    conn.write_packet(&RawPacket::new(cfg_pkt::ServerboundPongPacket::PACKET_ID, buf.freeze())).await?;
                }
                cfg_pkt::ClientboundFinishConfigurationPacket::PACKET_ID => {
                    let mut buf = BytesMut::new();
                    cfg_pkt::ServerboundFinishConfigurationPacket.write(&mut buf);
                    conn.write_packet(&RawPacket::new(cfg_pkt::ServerboundFinishConfigurationPacket::PACKET_ID, buf.freeze())).await?;
                    conn.state = State::Play;
                    *self.status.write().await = BotStatus::Playing;
                    info!("[Bot {}] Configuration finished. Switched to Play state.", self.config.username);
                }
                cfg_pkt::ClientboundDisconnectPacket::PACKET_ID => {
                    let disc = cfg_pkt::ClientboundDisconnectPacket::read(&mut std::io::Cursor::new(packet.data))?;
                    return Err(ProtocolError::Custom(format!("Disconnected in config: {}", disc.reason)));
                }
                _ => {}
            }
        }

        // 5. Play State Tick Loop (50ms = 20 TPS)
        let mut tick_interval = interval(Duration::from_millis(50));

        while conn.state == State::Play && self.running.load(Ordering::SeqCst) {
            tokio::select! {
                pkt_res = conn.read_packet() => {
                    let packet = pkt_res?;
                    self.handle_play_packet(&mut conn, packet).await?;
                }

                _ = tick_interval.tick() => {
                    self.tick_physics_and_send(&mut conn).await?;
                }

                Some(action) = self.action_rx.recv() => {
                    self.handle_action(&mut conn, action).await?;
                }
            }
        }

        Ok(())
    }

    async fn handle_play_packet(
        &mut self,
        conn: &mut MinecraftConnection,
        packet: RawPacket,
    ) -> Result<(), ProtocolError> {
        match packet.id {
            play_pkt::ClientboundKeepAlivePacket::PACKET_ID => {
                let keep = play_pkt::ClientboundKeepAlivePacket::read(&mut std::io::Cursor::new(packet.data))?;
                let mut buf = BytesMut::new();
                play_pkt::ServerboundKeepAlivePacket { id: keep.id }.write(&mut buf);
                conn.write_packet(&RawPacket::new(play_pkt::ServerboundKeepAlivePacket::PACKET_ID, buf.freeze())).await?;
            }
            play_pkt::ClientboundPingPacket::PACKET_ID => {
                let ping = play_pkt::ClientboundPingPacket::read(&mut std::io::Cursor::new(packet.data))?;
                let mut buf = BytesMut::new();
                play_pkt::ServerboundPongPacket { id: ping.id }.write(&mut buf);
                conn.write_packet(&RawPacket::new(play_pkt::ServerboundPongPacket::PACKET_ID, buf.freeze())).await?;
            }
            play_pkt::ClientboundPlayerPositionPacket::PACKET_ID => {
                let pos = play_pkt::ClientboundPlayerPositionPacket::read(&mut std::io::Cursor::new(packet.data))?;
                {
                    let mut p = self.physics.write().await;
                    p.x = pos.x;
                    p.y = pos.y;
                    p.z = pos.z;
                    p.yaw = pos.yaw;
                    p.pitch = pos.pitch;
                }
                let mut buf = BytesMut::new();
                play_pkt::ServerboundAcceptTeleportationPacket { teleport_id: pos.teleport_id }.write(&mut buf);
                conn.write_packet(&RawPacket::new(play_pkt::ServerboundAcceptTeleportationPacket::PACKET_ID, buf.freeze())).await?;
            }
            play_pkt::ClientboundBlockUpdatePacket::PACKET_ID => {
                let b = play_pkt::ClientboundBlockUpdatePacket::read(&mut std::io::Cursor::new(packet.data))?;
                self.world.set_block(b.position.x, b.position.y, b.position.z, b.block_state_id as u32);
            }
            play_pkt::ClientboundDisconnectPacket::PACKET_ID => {
                let disc = play_pkt::ClientboundDisconnectPacket::read(&mut std::io::Cursor::new(packet.data))?;
                return Err(ProtocolError::Custom(format!("Kicked: {}", disc.reason)));
            }
            _ => {}
        }
        Ok(())
    }

    async fn tick_physics_and_send(&mut self, conn: &mut MinecraftConnection) -> Result<(), ProtocolError> {
        if self.config.mode == BotMode::Fidelity {
            let mut p = self.physics.write().await;
            p.tick(&self.input, &self.world);

            let mut buf = BytesMut::new();
            play_pkt::ServerboundMovePlayerPosRotPacket {
                x: p.x,
                y: p.y,
                z: p.z,
                yaw: p.yaw,
                pitch: p.pitch,
                on_ground: p.on_ground,
            }.write(&mut buf);
            conn.write_packet(&RawPacket::new(play_pkt::ServerboundMovePlayerPosRotPacket::PACKET_ID, buf.freeze())).await?;
        } else {
            let mut buf = BytesMut::new();
            play_pkt::ServerboundMovePlayerStatusOnlyPacket { on_ground: true }.write(&mut buf);
            conn.write_packet(&RawPacket::new(play_pkt::ServerboundMovePlayerStatusOnlyPacket::PACKET_ID, buf.freeze())).await?;
        }
        Ok(())
    }

    async fn handle_action(
        &mut self,
        conn: &mut MinecraftConnection,
        action: BotAction,
    ) -> Result<(), ProtocolError> {
        match action {
            BotAction::Move { direction } => {
                match direction.as_str() {
                    "forward" => self.input.forward = 1.0,
                    "backward" => self.input.forward = -1.0,
                    "left" => self.input.sideways = 1.0,
                    "right" => self.input.sideways = -1.0,
                    _ => {
                        self.input.forward = 0.0;
                        self.input.sideways = 0.0;
                    }
                }
            }
            BotAction::LookAt { yaw, pitch } => {
                let mut p = self.physics.write().await;
                p.yaw = yaw;
                p.pitch = pitch;
            }
            BotAction::LookAtPos { x, y, z } => {
                let mut p = self.physics.write().await;
                let dx = x - p.x;
                let dy = y - (p.y + 1.62);
                let dz = z - p.z;
                let dist_xz = (dx * dx + dz * dz).sqrt();
                p.yaw = ((-dx).atan2(dz).to_degrees()) as f32;
                p.pitch = ((-dy).atan2(dist_xz).to_degrees()) as f32;
            }
            BotAction::Jump => {
                self.input.jumping = true;
            }
            BotAction::Stop => {
                self.input = PlayerInput::default();
            }
            BotAction::Attack { continuous: _ } => {
                let mut buf = BytesMut::new();
                play_pkt::ServerboundSwingPacket { hand: 0 }.write(&mut buf);
                conn.write_packet(&RawPacket::new(play_pkt::ServerboundSwingPacket::PACKET_ID, buf.freeze())).await?;
            }
            BotAction::Use { continuous: _ } => {
                let mut buf = BytesMut::new();
                play_pkt::ServerboundUseItemPacket { hand: 0, sequence: 0, yaw: 0.0, pitch: 0.0 }.write(&mut buf);
                conn.write_packet(&RawPacket::new(play_pkt::ServerboundUseItemPacket::PACKET_ID, buf.freeze())).await?;
            }
            BotAction::Chat { message } => {
                let mut buf = BytesMut::new();
                play_pkt::ServerboundChatMessagePacket {
                    message,
                    timestamp: 0,
                    salt: 0,
                    signature: None,
                }.write(&mut buf);
                conn.write_packet(&RawPacket::new(play_pkt::ServerboundChatMessagePacket::PACKET_ID, buf.freeze())).await?;
            }
            BotAction::Command { command } => {
                let mut buf = BytesMut::new();
                play_pkt::ServerboundChatCommandPacket { command }.write(&mut buf);
                conn.write_packet(&RawPacket::new(play_pkt::ServerboundChatCommandPacket::PACKET_ID, buf.freeze())).await?;
            }
            BotAction::Plugin { plugin_id, action, args } => {
                if let Some(host) = &self.plugin_host {
                    let host_guard = host.read().await;
                    let _ = host_guard.execute_action(&plugin_id, self.config.id, &action, &args);
                }
            }
        }
        Ok(())
    }
}
