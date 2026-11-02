use bytes::{Buf, BufMut, Bytes};

use crate::types::{
    read_string, write_string, BlockPosition,
};
use crate::varint::{read_varint, write_varint, ProtocolError};

// ---------------------------------------------------------------------------
// Clientbound Play Packets
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct ClientboundLoginPacket {
    pub entity_id: i32,
    pub is_hardcore: bool,
    pub dimension_names: Vec<String>,
    pub max_players: i32,
    pub view_distance: i32,
    pub simulation_distance: i32,
    pub reduced_debug_info: bool,
    pub enable_respawn_screen: bool,
    pub do_limited_crafting: bool,
    pub dimension_type: i32,
    pub dimension_name: String,
    pub hashed_seed: i64,
    pub game_mode: u8,
    pub previous_game_mode: i8,
    pub is_debug: bool,
    pub is_flat: bool,
    pub portal_cooldown: i32,
    pub enforces_secure_chat: bool,
}

impl ClientboundLoginPacket {
    pub const PACKET_ID: i32 = 0x2B;

    pub fn read<B: Buf>(buf: &mut B) -> Result<Self, ProtocolError> {
        if buf.remaining() < 5 {
            return Err(ProtocolError::UnexpectedEof);
        }
        let entity_id = buf.get_i32();
        let is_hardcore = buf.get_u8() != 0;
        let dim_count = read_varint(buf)?;
        let mut dimension_names = Vec::with_capacity(dim_count.max(0) as usize);
        for _ in 0..dim_count {
            dimension_names.push(read_string(buf, 32767)?);
        }
        let max_players = read_varint(buf)?;
        let view_distance = read_varint(buf)?;
        let simulation_distance = read_varint(buf)?;
        let reduced_debug_info = buf.get_u8() != 0;
        let enable_respawn_screen = buf.get_u8() != 0;
        let do_limited_crafting = buf.get_u8() != 0;
        let dimension_type = read_varint(buf)?;
        let dimension_name = read_string(buf, 32767)?;
        let hashed_seed = buf.get_i64();
        let game_mode = buf.get_u8();
        let previous_game_mode = buf.get_i8();
        let is_debug = buf.get_u8() != 0;
        let is_flat = buf.get_u8() != 0;
        let has_death_location = buf.get_u8() != 0;
        if has_death_location {
            let _death_dim = read_string(buf, 32767)?;
            let _death_pos = BlockPosition::read(buf)?;
        }
        let portal_cooldown = read_varint(buf)?;
        let enforces_secure_chat = if buf.has_remaining() {
            buf.get_u8() != 0
        } else {
            false
        };
        Ok(Self {
            entity_id,
            is_hardcore,
            dimension_names,
            max_players,
            view_distance,
            simulation_distance,
            reduced_debug_info,
            enable_respawn_screen,
            do_limited_crafting,
            dimension_type,
            dimension_name,
            hashed_seed,
            game_mode,
            previous_game_mode,
            is_debug,
            is_flat,
            portal_cooldown,
            enforces_secure_chat,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClientboundPlayerPositionPacket {
    pub teleport_id: i32,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub vx: f64,
    pub vy: f64,
    pub vz: f64,
    pub yaw: f32,
    pub pitch: f32,
    pub relative_flags: i32,
}

impl ClientboundPlayerPositionPacket {
    pub const PACKET_ID: i32 = 0x40;

    pub fn read<B: Buf>(buf: &mut B) -> Result<Self, ProtocolError> {
        let teleport_id = read_varint(buf)?;
        if buf.remaining() < 8 * 3 + 4 * 2 {
            return Err(ProtocolError::UnexpectedEof);
        }
        let x = buf.get_f64();
        let y = buf.get_f64();
        let z = buf.get_f64();
        let vx = buf.get_f64();
        let vy = buf.get_f64();
        let vz = buf.get_f64();
        let yaw = buf.get_f32();
        let pitch = buf.get_f32();
        let relative_flags = buf.get_i32();
        Ok(Self {
            teleport_id,
            x,
            y,
            z,
            vx,
            vy,
            vz,
            yaw,
            pitch,
            relative_flags,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientboundKeepAlivePacket {
    pub id: i64,
}

impl ClientboundKeepAlivePacket {
    pub const PACKET_ID: i32 = 0x26;

    pub fn read<B: Buf>(buf: &mut B) -> Result<Self, ProtocolError> {
        if buf.remaining() < 8 {
            return Err(ProtocolError::UnexpectedEof);
        }
        let id = buf.get_i64();
        Ok(Self { id })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientboundPingPacket {
    pub id: i32,
}

impl ClientboundPingPacket {
    pub const PACKET_ID: i32 = 0x35;

    pub fn read<B: Buf>(buf: &mut B) -> Result<Self, ProtocolError> {
        if buf.remaining() < 4 {
            return Err(ProtocolError::UnexpectedEof);
        }
        let id = buf.get_i32();
        Ok(Self { id })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClientboundChunkDataPacket {
    pub x: i32,
    pub z: i32,
    pub data: Bytes,
}

impl ClientboundChunkDataPacket {
    pub const PACKET_ID: i32 = 0x27;

    pub fn read<B: Buf>(buf: &mut B) -> Result<Self, ProtocolError> {
        if buf.remaining() < 8 {
            return Err(ProtocolError::UnexpectedEof);
        }
        let x = buf.get_i32();
        let z = buf.get_i32();
        let rem = buf.remaining();
        let data = buf.copy_to_bytes(rem);
        Ok(Self { x, z, data })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientboundForgetChunkPacket {
    pub x: i32,
    pub z: i32,
}

impl ClientboundForgetChunkPacket {
    pub const PACKET_ID: i32 = 0x21;

    pub fn read<B: Buf>(buf: &mut B) -> Result<Self, ProtocolError> {
        if buf.remaining() < 8 {
            return Err(ProtocolError::UnexpectedEof);
        }
        let x = buf.get_i32();
        let z = buf.get_i32();
        Ok(Self { x, z })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientboundBlockUpdatePacket {
    pub position: BlockPosition,
    pub block_state_id: i32,
}

impl ClientboundBlockUpdatePacket {
    pub const PACKET_ID: i32 = 0x09;

    pub fn read<B: Buf>(buf: &mut B) -> Result<Self, ProtocolError> {
        let position = BlockPosition::read(buf)?;
        let block_state_id = read_varint(buf)?;
        Ok(Self {
            position,
            block_state_id,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientboundSystemChatPacket {
    pub content: String,
    pub overlay: bool,
}

impl ClientboundSystemChatPacket {
    pub const PACKET_ID: i32 = 0x6C;

    pub fn read<B: Buf>(buf: &mut B) -> Result<Self, ProtocolError> {
        let content = read_string(buf, 32767)?;
        let overlay = buf.has_remaining() && buf.get_u8() != 0;
        Ok(Self { content, overlay })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientboundDisconnectPacket {
    pub reason: String,
}

impl ClientboundDisconnectPacket {
    pub const PACKET_ID: i32 = 0x1D;

    pub fn read<B: Buf>(buf: &mut B) -> Result<Self, ProtocolError> {
        let reason = read_string(buf, 32767)?;
        Ok(Self { reason })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClientboundSetHealthPacket {
    pub health: f32,
    pub food: i32,
    pub food_saturation: f32,
}

impl ClientboundSetHealthPacket {
    pub const PACKET_ID: i32 = 0x5D;

    pub fn read<B: Buf>(buf: &mut B) -> Result<Self, ProtocolError> {
        if buf.remaining() < 4 {
            return Err(ProtocolError::UnexpectedEof);
        }
        let health = buf.get_f32();
        let food = read_varint(buf)?;
        if buf.remaining() < 4 {
            return Err(ProtocolError::UnexpectedEof);
        }
        let food_saturation = buf.get_f32();
        Ok(Self {
            health,
            food,
            food_saturation,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientboundCustomPayloadPacket {
    pub channel: String,
    pub data: Bytes,
}

impl ClientboundCustomPayloadPacket {
    pub const PACKET_ID: i32 = 0x19;

    pub fn read<B: Buf>(buf: &mut B) -> Result<Self, ProtocolError> {
        let channel = read_string(buf, 32767)?;
        let rem = buf.remaining();
        let data = buf.copy_to_bytes(rem);
        Ok(Self { channel, data })
    }
}

// ---------------------------------------------------------------------------
// Serverbound Play Packets
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServerboundAcceptTeleportationPacket {
    pub teleport_id: i32,
}

impl ServerboundAcceptTeleportationPacket {
    pub const PACKET_ID: i32 = 0x00;

    pub fn write<B: BufMut>(&self, buf: &mut B) {
        write_varint(self.teleport_id, buf);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerboundChatCommandPacket {
    pub command: String,
}

impl ServerboundChatCommandPacket {
    pub const PACKET_ID: i32 = 0x04;

    pub fn write<B: BufMut>(&self, buf: &mut B) {
        write_string(&self.command, buf);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerboundChatMessagePacket {
    pub message: String,
    pub timestamp: i64,
    pub salt: i64,
    pub signature: Option<Bytes>,
}

impl ServerboundChatMessagePacket {
    pub const PACKET_ID: i32 = 0x06;

    pub fn write<B: BufMut>(&self, buf: &mut B) {
        write_string(&self.message, buf);
        buf.put_i64(self.timestamp);
        buf.put_i64(self.salt);
        if let Some(sig) = &self.signature {
            buf.put_u8(1);
            buf.put_slice(sig);
        } else {
            buf.put_u8(0);
        }
        write_varint(0, buf); // count of seen message signatures
        buf.put_u8(0); // checksum / last seen
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ServerboundChunkBatchReceivedPacket {
    pub desired_chunks_per_tick: f32,
}

impl ServerboundChunkBatchReceivedPacket {
    pub const PACKET_ID: i32 = 0x08;

    pub fn write<B: BufMut>(&self, buf: &mut B) {
        buf.put_f32(self.desired_chunks_per_tick);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServerboundConfigurationAcknowledgedPacket;

impl ServerboundConfigurationAcknowledgedPacket {
    pub const PACKET_ID: i32 = 0x0C;

    pub fn write<B: BufMut>(&self, _buf: &mut B) {}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerboundCustomPayloadPacket {
    pub channel: String,
    pub data: Bytes,
}

impl ServerboundCustomPayloadPacket {
    pub const PACKET_ID: i32 = 0x12;

    pub fn write<B: BufMut>(&self, buf: &mut B) {
        write_string(&self.channel, buf);
        buf.put_slice(&self.data);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServerboundKeepAlivePacket {
    pub id: i64,
}

impl ServerboundKeepAlivePacket {
    pub const PACKET_ID: i32 = 0x18;

    pub fn write<B: BufMut>(&self, buf: &mut B) {
        buf.put_i64(self.id);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ServerboundMovePlayerPosPacket {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub on_ground: bool,
}

impl ServerboundMovePlayerPosPacket {
    pub const PACKET_ID: i32 = 0x1A;

    pub fn write<B: BufMut>(&self, buf: &mut B) {
        buf.put_f64(self.x);
        buf.put_f64(self.y);
        buf.put_f64(self.z);
        buf.put_u8(if self.on_ground { 1 } else { 0 });
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ServerboundMovePlayerPosRotPacket {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
}

impl ServerboundMovePlayerPosRotPacket {
    pub const PACKET_ID: i32 = 0x1B;

    pub fn write<B: BufMut>(&self, buf: &mut B) {
        buf.put_f64(self.x);
        buf.put_f64(self.y);
        buf.put_f64(self.z);
        buf.put_f32(self.yaw);
        buf.put_f32(self.pitch);
        buf.put_u8(if self.on_ground { 1 } else { 0 });
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ServerboundMovePlayerRotPacket {
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
}

impl ServerboundMovePlayerRotPacket {
    pub const PACKET_ID: i32 = 0x1C;

    pub fn write<B: BufMut>(&self, buf: &mut B) {
        buf.put_f32(self.yaw);
        buf.put_f32(self.pitch);
        buf.put_u8(if self.on_ground { 1 } else { 0 });
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServerboundMovePlayerStatusOnlyPacket {
    pub on_ground: bool,
}

impl ServerboundMovePlayerStatusOnlyPacket {
    pub const PACKET_ID: i32 = 0x1D;

    pub fn write<B: BufMut>(&self, buf: &mut B) {
        buf.put_u8(if self.on_ground { 1 } else { 0 });
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServerboundPlayerActionPacket {
    pub status: i32, // 0 = start digging, 1 = cancel digging, 2 = finish digging, 3 = drop all, 4 = drop one, etc.
    pub position: BlockPosition,
    pub face: u8,
    pub sequence: i32,
}

impl ServerboundPlayerActionPacket {
    pub const PACKET_ID: i32 = 0x24;

    pub fn write<B: BufMut>(&self, buf: &mut B) {
        write_varint(self.status, buf);
        self.position.write(buf);
        buf.put_u8(self.face);
        write_varint(self.sequence, buf);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServerboundPlayerCommandPacket {
    pub entity_id: i32,
    pub action_id: i32, // 0 = start sneaking, 1 = stop sneaking, 3 = start sprinting, 4 = stop sprinting
    pub jump_boost: i32,
}

impl ServerboundPlayerCommandPacket {
    pub const PACKET_ID: i32 = 0x25;

    pub fn write<B: BufMut>(&self, buf: &mut B) {
        write_varint(self.entity_id, buf);
        write_varint(self.action_id, buf);
        write_varint(self.jump_boost, buf);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServerboundPlayerInputPacket {
    pub flags: u8, // bit 0 = jump, bit 1 = sneak
}

impl ServerboundPlayerInputPacket {
    pub const PACKET_ID: i32 = 0x26;

    pub fn write<B: BufMut>(&self, buf: &mut B) {
        buf.put_u8(self.flags);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServerboundPongPacket {
    pub id: i32,
}

impl ServerboundPongPacket {
    pub const PACKET_ID: i32 = 0x27;

    pub fn write<B: BufMut>(&self, buf: &mut B) {
        buf.put_i32(self.id);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServerboundSetCarriedItemPacket {
    pub slot: i16,
}

impl ServerboundSetCarriedItemPacket {
    pub const PACKET_ID: i32 = 0x2F;

    pub fn write<B: BufMut>(&self, buf: &mut B) {
        buf.put_i16(self.slot);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServerboundSwingPacket {
    pub hand: i32, // 0 = main hand, 1 = off hand
}

impl ServerboundSwingPacket {
    pub const PACKET_ID: i32 = 0x36;

    pub fn write<B: BufMut>(&self, buf: &mut B) {
        write_varint(self.hand, buf);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ServerboundUseItemOnPacket {
    pub hand: i32,
    pub position: BlockPosition,
    pub face: i32,
    pub cursor_x: f32,
    pub cursor_y: f32,
    pub cursor_z: f32,
    pub inside_block: bool,
    pub world_border_hit: bool,
    pub sequence: i32,
}

impl ServerboundUseItemOnPacket {
    pub const PACKET_ID: i32 = 0x38;

    pub fn write<B: BufMut>(&self, buf: &mut B) {
        write_varint(self.hand, buf);
        self.position.write(buf);
        write_varint(self.face, buf);
        buf.put_f32(self.cursor_x);
        buf.put_f32(self.cursor_y);
        buf.put_f32(self.cursor_z);
        buf.put_u8(if self.inside_block { 1 } else { 0 });
        buf.put_u8(if self.world_border_hit { 1 } else { 0 });
        write_varint(self.sequence, buf);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ServerboundUseItemPacket {
    pub hand: i32,
    pub sequence: i32,
    pub yaw: f32,
    pub pitch: f32,
}

impl ServerboundUseItemPacket {
    pub const PACKET_ID: i32 = 0x39;

    pub fn write<B: BufMut>(&self, buf: &mut B) {
        write_varint(self.hand, buf);
        write_varint(self.sequence, buf);
        buf.put_f32(self.yaw);
        buf.put_f32(self.pitch);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ServerboundInteractPacket {
    pub entity_id: i32,
    pub action_type: i32, // 0 = interact, 1 = attack, 2 = interact_at
    pub target_pos: Option<(f32, f32, f32)>,
    pub hand: Option<i32>,
    pub sneaking: bool,
}

impl ServerboundInteractPacket {
    pub const PACKET_ID: i32 = 0x16;

    pub fn attack(entity_id: i32, sneaking: bool) -> Self {
        Self {
            entity_id,
            action_type: 1, // attack
            target_pos: None,
            hand: None,
            sneaking,
        }
    }

    pub fn interact(entity_id: i32, hand: i32, sneaking: bool) -> Self {
        Self {
            entity_id,
            action_type: 0,
            target_pos: None,
            hand: Some(hand),
            sneaking,
        }
    }

    pub fn write<B: BufMut>(&self, buf: &mut B) {
        write_varint(self.entity_id, buf);
        write_varint(self.action_type, buf);
        if self.action_type == 2 {
            if let Some((x, y, z)) = self.target_pos {
                buf.put_f32(x);
                buf.put_f32(y);
                buf.put_f32(z);
            }
        }
        if self.action_type == 0 || self.action_type == 2 {
            if let Some(h) = self.hand {
                write_varint(h, buf);
            }
        }
        buf.put_u8(if self.sneaking { 1 } else { 0 });
    }
}
