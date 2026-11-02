use bytes::{Buf, BufMut, Bytes};
use uuid::Uuid;

use crate::types::{read_byte_array, read_string, read_uuid, write_byte_array, write_string, write_uuid};
use crate::varint::{read_varint, write_varint, ProtocolError};

// ---------------------------------------------------------------------------
// Serverbound Login Packets
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerboundHelloPacket {
    pub name: String,
    pub profile_id: Uuid,
}

impl ServerboundHelloPacket {
    pub const PACKET_ID: i32 = 0x00;

    pub fn write<B: BufMut>(&self, buf: &mut B) {
        write_string(&self.name, buf);
        write_uuid(&self.profile_id, buf);
    }

    pub fn read<B: Buf>(buf: &mut B) -> Result<Self, ProtocolError> {
        let name = read_string(buf, 16)?;
        let profile_id = read_uuid(buf)?;
        Ok(Self { name, profile_id })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerboundKeyPacket {
    pub shared_secret: Bytes,
    pub verify_token: Bytes,
}

impl ServerboundKeyPacket {
    pub const PACKET_ID: i32 = 0x01;

    pub fn write<B: BufMut>(&self, buf: &mut B) {
        write_byte_array(&self.shared_secret, buf);
        write_byte_array(&self.verify_token, buf);
    }

    pub fn read<B: Buf>(buf: &mut B) -> Result<Self, ProtocolError> {
        let shared_secret = read_byte_array(buf)?;
        let verify_token = read_byte_array(buf)?;
        Ok(Self {
            shared_secret,
            verify_token,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerboundCustomQueryAnswerPacket {
    pub message_id: i32,
    pub successful: bool,
    pub data: Option<Bytes>,
}

impl ServerboundCustomQueryAnswerPacket {
    pub const PACKET_ID: i32 = 0x02;

    pub fn write<B: BufMut>(&self, buf: &mut B) {
        write_varint(self.message_id, buf);
        if let Some(data) = &self.data {
            buf.put_u8(1);
            buf.put_slice(data);
        } else {
            buf.put_u8(0);
        }
    }

    pub fn read<B: Buf>(buf: &mut B) -> Result<Self, ProtocolError> {
        let message_id = read_varint(buf)?;
        if !buf.has_remaining() {
            return Err(ProtocolError::UnexpectedEof);
        }
        let successful = buf.get_u8() != 0;
        let data = if successful && buf.has_remaining() {
            let rem = buf.remaining();
            Some(buf.copy_to_bytes(rem))
        } else {
            None
        };
        Ok(Self {
            message_id,
            successful,
            data,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServerboundLoginAcknowledgedPacket;

impl ServerboundLoginAcknowledgedPacket {
    pub const PACKET_ID: i32 = 0x03;

    pub fn write<B: BufMut>(&self, _buf: &mut B) {}
    pub fn read<B: Buf>(_buf: &mut B) -> Result<Self, ProtocolError> {
        Ok(Self)
    }
}

// ---------------------------------------------------------------------------
// Clientbound Login Packets
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientboundLoginDisconnectPacket {
    pub reason: String,
}

impl ClientboundLoginDisconnectPacket {
    pub const PACKET_ID: i32 = 0x00;

    pub fn read<B: Buf>(buf: &mut B) -> Result<Self, ProtocolError> {
        let reason = read_string(buf, 32767)?;
        Ok(Self { reason })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientboundHelloPacket {
    pub server_id: String,
    pub public_key: Bytes,
    pub verify_token: Bytes,
    pub should_authenticate: bool,
}

impl ClientboundHelloPacket {
    pub const PACKET_ID: i32 = 0x01;

    pub fn read<B: Buf>(buf: &mut B) -> Result<Self, ProtocolError> {
        let server_id = read_string(buf, 20)?;
        let public_key = read_byte_array(buf)?;
        let verify_token = read_byte_array(buf)?;
        let should_authenticate = if buf.has_remaining() {
            buf.get_u8() != 0
        } else {
            true
        };
        Ok(Self {
            server_id,
            public_key,
            verify_token,
            should_authenticate,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileProperty {
    pub name: String,
    pub value: String,
    pub signature: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientboundGameProfilePacket {
    pub uuid: Uuid,
    pub username: String,
    pub properties: Vec<ProfileProperty>,
}

impl ClientboundGameProfilePacket {
    pub const PACKET_ID: i32 = 0x02;

    pub fn read<B: Buf>(buf: &mut B) -> Result<Self, ProtocolError> {
        let uuid = read_uuid(buf)?;
        let username = read_string(buf, 16)?;
        let prop_count = read_varint(buf)?;
        let mut properties = Vec::with_capacity(prop_count.max(0) as usize);
        for _ in 0..prop_count {
            let name = read_string(buf, 32767)?;
            let value = read_string(buf, 32767)?;
            let has_sig = if buf.has_remaining() { buf.get_u8() != 0 } else { false };
            let signature = if has_sig {
                Some(read_string(buf, 32767)?)
            } else {
                None
            };
            properties.push(ProfileProperty {
                name,
                value,
                signature,
            });
        }
        Ok(Self {
            uuid,
            username,
            properties,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientboundLoginCompressionPacket {
    pub threshold: i32,
}

impl ClientboundLoginCompressionPacket {
    pub const PACKET_ID: i32 = 0x03;

    pub fn read<B: Buf>(buf: &mut B) -> Result<Self, ProtocolError> {
        let threshold = read_varint(buf)?;
        Ok(Self { threshold })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientboundCustomQueryPacket {
    pub message_id: i32,
    pub channel: String,
    pub data: Bytes,
}

impl ClientboundCustomQueryPacket {
    pub const PACKET_ID: i32 = 0x04;

    pub fn read<B: Buf>(buf: &mut B) -> Result<Self, ProtocolError> {
        let message_id = read_varint(buf)?;
        let channel = read_string(buf, 32767)?;
        let rem = buf.remaining();
        let data = buf.copy_to_bytes(rem);
        Ok(Self {
            message_id,
            channel,
            data,
        })
    }
}
