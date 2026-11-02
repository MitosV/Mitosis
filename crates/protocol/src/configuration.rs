use bytes::{Buf, BufMut, Bytes};

use crate::types::{read_string, write_string};
use crate::varint::{read_varint, write_varint, ProtocolError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnownPack {
    pub namespace: String,
    pub id: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerboundClientInformationPacket {
    pub locale: String,
    pub view_distance: i8,
    pub chat_mode: i32,
    pub chat_colors: bool,
    pub skin_parts: u8,
    pub main_hand: i32,
    pub text_filtering: bool,
    pub allows_listing: bool,
}

impl Default for ServerboundClientInformationPacket {
    fn default() -> Self {
        Self {
            locale: "en_us".to_string(),
            view_distance: 8,
            chat_mode: 0, // Enabled
            chat_colors: true,
            skin_parts: 0x7F, // All parts enabled (cape, jacket, left/right sleeve, left/right pants, hat)
            main_hand: 1,     // Right hand
            text_filtering: false,
            allows_listing: true,
        }
    }
}

impl ServerboundClientInformationPacket {
    pub const PACKET_ID: i32 = 0x00;

    pub fn write<B: BufMut>(&self, buf: &mut B) {
        write_string(&self.locale, buf);
        buf.put_i8(self.view_distance);
        write_varint(self.chat_mode, buf);
        buf.put_u8(if self.chat_colors { 1 } else { 0 });
        buf.put_u8(self.skin_parts);
        write_varint(self.main_hand, buf);
        buf.put_u8(if self.text_filtering { 1 } else { 0 });
        buf.put_u8(if self.allows_listing { 1 } else { 0 });
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerboundCustomPayloadPacket {
    pub channel: String,
    pub data: Bytes,
}

impl ServerboundCustomPayloadPacket {
    pub const PACKET_ID: i32 = 0x02;

    pub fn write<B: BufMut>(&self, buf: &mut B) {
        write_string(&self.channel, buf);
        buf.put_slice(&self.data);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServerboundFinishConfigurationPacket;

impl ServerboundFinishConfigurationPacket {
    pub const PACKET_ID: i32 = 0x03;

    pub fn write<B: BufMut>(&self, _buf: &mut B) {}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServerboundKeepAlivePacket {
    pub id: i64,
}

impl ServerboundKeepAlivePacket {
    pub const PACKET_ID: i32 = 0x04;

    pub fn write<B: BufMut>(&self, buf: &mut B) {
        buf.put_i64(self.id);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServerboundPongPacket {
    pub id: i32,
}

impl ServerboundPongPacket {
    pub const PACKET_ID: i32 = 0x05;

    pub fn write<B: BufMut>(&self, buf: &mut B) {
        buf.put_i32(self.id);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerboundSelectKnownPacksPacket {
    pub known_packs: Vec<KnownPack>,
}

impl ServerboundSelectKnownPacksPacket {
    pub const PACKET_ID: i32 = 0x07;

    pub fn write<B: BufMut>(&self, buf: &mut B) {
        write_varint(self.known_packs.len() as i32, buf);
        for pack in &self.known_packs {
            write_string(&pack.namespace, buf);
            write_string(&pack.id, buf);
            write_string(&pack.version, buf);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientboundCustomPayloadPacket {
    pub channel: String,
    pub data: Bytes,
}

impl ClientboundCustomPayloadPacket {
    pub const PACKET_ID: i32 = 0x01;

    pub fn read<B: Buf>(buf: &mut B) -> Result<Self, ProtocolError> {
        let channel = read_string(buf, 32767)?;
        let rem = buf.remaining();
        let data = buf.copy_to_bytes(rem);
        Ok(Self { channel, data })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientboundDisconnectPacket {
    pub reason: String,
}

impl ClientboundDisconnectPacket {
    pub const PACKET_ID: i32 = 0x02;

    pub fn read<B: Buf>(buf: &mut B) -> Result<Self, ProtocolError> {
        let reason = read_string(buf, 32767)?;
        Ok(Self { reason })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientboundFinishConfigurationPacket;

impl ClientboundFinishConfigurationPacket {
    pub const PACKET_ID: i32 = 0x03;

    pub fn read<B: Buf>(_buf: &mut B) -> Result<Self, ProtocolError> {
        Ok(Self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientboundKeepAlivePacket {
    pub id: i64,
}

impl ClientboundKeepAlivePacket {
    pub const PACKET_ID: i32 = 0x04;

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
    pub const PACKET_ID: i32 = 0x05;

    pub fn read<B: Buf>(buf: &mut B) -> Result<Self, ProtocolError> {
        if buf.remaining() < 4 {
            return Err(ProtocolError::UnexpectedEof);
        }
        let id = buf.get_i32();
        Ok(Self { id })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientboundRegistryDataPacket {
    pub registry_id: String,
    pub raw_data: Bytes,
}

impl ClientboundRegistryDataPacket {
    pub const PACKET_ID: i32 = 0x07;

    pub fn read<B: Buf>(buf: &mut B) -> Result<Self, ProtocolError> {
        let registry_id = read_string(buf, 32767)?;
        let rem = buf.remaining();
        let raw_data = buf.copy_to_bytes(rem);
        Ok(Self {
            registry_id,
            raw_data,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientboundSelectKnownPacksPacket {
    pub known_packs: Vec<KnownPack>,
}

impl ClientboundSelectKnownPacksPacket {
    pub const PACKET_ID: i32 = 0x0E;

    pub fn read<B: Buf>(buf: &mut B) -> Result<Self, ProtocolError> {
        let count = read_varint(buf)?;
        let mut known_packs = Vec::with_capacity(count.max(0) as usize);
        for _ in 0..count {
            let namespace = read_string(buf, 32767)?;
            let id = read_string(buf, 32767)?;
            let version = read_string(buf, 32767)?;
            known_packs.push(KnownPack {
                namespace,
                id,
                version,
            });
        }
        Ok(Self { known_packs })
    }
}
