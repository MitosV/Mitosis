pub mod configuration;
pub mod handshake;
pub mod login;
pub mod play;
pub mod types;
pub mod varint;

use bytes::{BufMut, Bytes, BytesMut};
pub use varint::{read_varint, write_varint, varint_size, ProtocolError, PROTOCOL_VERSION};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum State {
    #[default]
    Handshaking,
    Status,
    Login,
    Configuration,
    Play,
}

/// A raw Minecraft packet consisting of a packet ID and its payload bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawPacket {
    pub id: i32,
    pub data: Bytes,
}

impl RawPacket {
    pub fn new(id: i32, data: impl Into<Bytes>) -> Self {
        Self {
            id,
            data: data.into(),
        }
    }

    /// Encodes a raw packet with uncompressed framing:
    /// VarInt(length) + VarInt(packet_id) + payload
    pub fn encode_uncompressed(&self) -> Bytes {
        let id_size = varint_size(self.id);
        let total_size = id_size + self.data.len();
        let frame_size = varint_size(total_size as i32) + total_size;

        let mut buf = BytesMut::with_capacity(frame_size);
        write_varint(total_size as i32, &mut buf);
        write_varint(self.id, &mut buf);
        buf.put_slice(&self.data);
        buf.freeze()
    }

    /// Decodes an uncompressed packet from an internal buffer slice containing:
    /// VarInt(packet_id) + payload
    pub fn decode_uncompressed(mut buf: Bytes) -> Result<Self, ProtocolError> {
        let id = read_varint(&mut buf)?;
        Ok(Self { id, data: buf })
    }
}
