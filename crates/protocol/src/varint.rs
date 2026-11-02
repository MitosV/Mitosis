use bytes::{Buf, BufMut};
use thiserror::Error;

pub const PROTOCOL_VERSION: i32 = 767; // Minecraft 1.21.1

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ProtocolError {
    #[error("VarInt is too wide (more than 5 bytes)")]
    VarIntTooWide,
    #[error("VarLong is too wide (more than 10 bytes)")]
    VarLongTooWide,
    #[error("Buffer underflow: unexpected end of buffer")]
    UnexpectedEof,
    #[error("Invalid UTF-8 string: {0}")]
    InvalidUtf8(#[from] std::string::FromUtf8Error),
    #[error("String too long: {0} > {1}")]
    StringTooLong(usize, usize),
    #[error("Unknown packet ID: 0x{0:02X} in state {1:?}")]
    UnknownPacket(i32, crate::State),
    #[error("Decompression failed: {0}")]
    DecompressionFailed(String),
    #[error("Packet exceeds maximum length: {0}")]
    PacketTooLarge(usize),
    #[error("Custom protocol error: {0}")]
    Custom(String),
}

/// Reads a VarInt from a buffer.
pub fn read_varint<B: Buf>(buf: &mut B) -> Result<i32, ProtocolError> {
    let mut val = 0i32;
    let mut shift = 0;
    while shift < 35 {
        if !buf.has_remaining() {
            return Err(ProtocolError::UnexpectedEof);
        }
        let b = buf.get_u8();
        val |= ((b & 0x7F) as i32) << shift;
        if (b & 0x80) == 0 {
            return Ok(val);
        }
        shift += 7;
    }
    Err(ProtocolError::VarIntTooWide)
}

/// Writes a VarInt to a buffer.
pub fn write_varint<B: BufMut>(mut val: i32, buf: &mut B) {
    loop {
        if (val & !0x7F) == 0 {
            buf.put_u8(val as u8);
            return;
        } else {
            buf.put_u8(((val & 0x7F) | 0x80) as u8);
            val = ((val as u32) >> 7) as i32;
        }
    }
}

/// Calculates the byte size of a VarInt.
#[inline]
pub fn varint_size(mut val: i32) -> usize {
    let mut len = 0;
    loop {
        len += 1;
        if (val & !0x7F) == 0 {
            return len;
        }
        val = ((val as u32) >> 7) as i32;
    }
}

/// Reads a VarLong from a buffer.
pub fn read_varlong<B: Buf>(buf: &mut B) -> Result<i64, ProtocolError> {
    let mut val = 0i64;
    let mut shift = 0;
    while shift < 70 {
        if !buf.has_remaining() {
            return Err(ProtocolError::UnexpectedEof);
        }
        let b = buf.get_u8();
        val |= ((b & 0x7F) as i64) << shift;
        if (b & 0x80) == 0 {
            return Ok(val);
        }
        shift += 7;
    }
    Err(ProtocolError::VarLongTooWide)
}

/// Writes a VarLong to a buffer.
pub fn write_varlong<B: BufMut>(mut val: i64, buf: &mut B) {
    loop {
        if (val & !0x7F) == 0 {
            buf.put_u8(val as u8);
            return;
        } else {
            buf.put_u8(((val & 0x7F) | 0x80) as u8);
            val = ((val as u64) >> 7) as i64;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn test_varint_roundtrip() {
        let values = [0, 1, 2, 127, 128, 255, 256, 2147483647, -1, -2147483648];
        for &v in &values {
            let mut buf = BytesMut::new();
            write_varint(v, &mut buf);
            assert_eq!(buf.len(), varint_size(v));
            let decoded = read_varint(&mut buf).unwrap();
            assert_eq!(v, decoded);
        }
    }

    #[test]
    fn test_varlong_roundtrip() {
        let values = [0, 1, 127, 128, 9223372036854775807, -1, -9223372036854775808];
        for &v in &values {
            let mut buf = BytesMut::new();
            write_varlong(v, &mut buf);
            let decoded = read_varlong(&mut buf).unwrap();
            assert_eq!(v, decoded);
        }
    }
}
