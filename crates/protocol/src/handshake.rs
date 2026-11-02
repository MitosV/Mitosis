use bytes::{Buf, BufMut};
use crate::types::{read_string, write_string};
use crate::varint::{read_varint, write_varint, ProtocolError, PROTOCOL_VERSION};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientIntentionPacket {
    pub protocol_version: i32,
    pub server_address: String,
    pub server_port: u16,
    pub next_state: i32, // 1 = Status, 2 = Login
}

impl ClientIntentionPacket {
    pub const PACKET_ID: i32 = 0x00;

    pub fn new(server_address: impl Into<String>, server_port: u16, next_state: i32) -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            server_address: server_address.into(),
            server_port,
            next_state,
        }
    }

    pub fn write<B: BufMut>(&self, buf: &mut B) {
        write_varint(self.protocol_version, buf);
        write_string(&self.server_address, buf);
        buf.put_u16(self.server_port);
        write_varint(self.next_state, buf);
    }

    pub fn read<B: Buf>(buf: &mut B) -> Result<Self, ProtocolError> {
        let protocol_version = read_varint(buf)?;
        let server_address = read_string(buf, 255)?;
        if buf.remaining() < 2 {
            return Err(ProtocolError::UnexpectedEof);
        }
        let server_port = buf.get_u16();
        let next_state = read_varint(buf)?;
        Ok(Self {
            protocol_version,
            server_address,
            server_port,
            next_state,
        })
    }
}
