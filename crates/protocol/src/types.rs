use bytes::{Buf, BufMut, Bytes};
use uuid::Uuid;

use crate::varint::{read_varint, write_varint, ProtocolError};

/// Reads a UTF-8 string with a VarInt length prefix.
pub fn read_string<B: Buf>(buf: &mut B, max_len: usize) -> Result<String, ProtocolError> {
    let len = read_varint(buf)? as usize;
    if len > max_len * 4 {
        return Err(ProtocolError::StringTooLong(len, max_len));
    }
    if buf.remaining() < len {
        return Err(ProtocolError::UnexpectedEof);
    }
    let mut vec = vec![0u8; len];
    buf.copy_to_slice(&mut vec);
    let s = String::from_utf8(vec)?;
    if s.chars().count() > max_len {
        return Err(ProtocolError::StringTooLong(s.len(), max_len));
    }
    Ok(s)
}

/// Writes a UTF-8 string with a VarInt length prefix.
pub fn write_string<B: BufMut>(s: &str, buf: &mut B) {
    write_varint(s.len() as i32, buf);
    buf.put_slice(s.as_bytes());
}

/// Reads a byte array with a VarInt length prefix.
pub fn read_byte_array<B: Buf>(buf: &mut B) -> Result<Bytes, ProtocolError> {
    let len = read_varint(buf)? as usize;
    if buf.remaining() < len {
        return Err(ProtocolError::UnexpectedEof);
    }
    let slice = buf.copy_to_bytes(len);
    Ok(slice)
}

/// Writes a byte array with a VarInt length prefix.
pub fn write_byte_array<B: BufMut>(bytes: &[u8], buf: &mut B) {
    write_varint(bytes.len() as i32, buf);
    buf.put_slice(bytes);
}

/// Reads a 128-bit UUID.
pub fn read_uuid<B: Buf>(buf: &mut B) -> Result<Uuid, ProtocolError> {
    if buf.remaining() < 16 {
        return Err(ProtocolError::UnexpectedEof);
    }
    let mut bytes = [0u8; 16];
    buf.copy_to_slice(&mut bytes);
    Ok(Uuid::from_bytes(bytes))
}

/// Writes a 128-bit UUID.
pub fn write_uuid<B: BufMut>(uuid: &Uuid, buf: &mut B) {
    buf.put_slice(uuid.as_bytes());
}

/// Minecraft Block Position: packed 64-bit integer
/// ((x & 0x3FFFFFF) << 38) | ((z & 0x3FFFFFF) << 12) | (y & 0xFFF)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct BlockPosition {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl BlockPosition {
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    pub fn read<B: Buf>(buf: &mut B) -> Result<Self, ProtocolError> {
        if buf.remaining() < 8 {
            return Err(ProtocolError::UnexpectedEof);
        }
        let val = buf.get_i64();
        let mut x = (val >> 38) as i32;
        let mut y = (val << 52 >> 52) as i32;
        let mut z = (val << 26 >> 38) as i32;

        // Sign extension for 26-bit and 12-bit numbers
        if x >= 1 << 25 {
            x -= 1 << 26;
        }
        if y >= 1 << 11 {
            y -= 1 << 12;
        }
        if z >= 1 << 25 {
            z -= 1 << 26;
        }

        Ok(Self { x, y, z })
    }

    pub fn write<B: BufMut>(&self, buf: &mut B) {
        let val = (((self.x as i64) & 0x3FFFFFF) << 38)
            | (((self.z as i64) & 0x3FFFFFF) << 12)
            | ((self.y as i64) & 0xFFF);
        buf.put_i64(val);
    }
}

/// Converts yaw/pitch to protocol byte angle (0..255).
#[inline]
pub fn angle_to_byte(angle: f32) -> u8 {
    (angle * 256.0 / 360.0).floor() as u8
}

/// Converts protocol byte angle back to degrees.
#[inline]
pub fn byte_to_angle(byte: u8) -> f32 {
    byte as f32 * 360.0 / 256.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn test_string_roundtrip() {
        let mut buf = BytesMut::new();
        write_string("Hello Minecraft 1.21.1", &mut buf);
        let s = read_string(&mut buf, 32767).unwrap();
        assert_eq!(s, "Hello Minecraft 1.21.1");
    }

    #[test]
    fn test_uuid_roundtrip() {
        let mut buf = BytesMut::new();
        let id = Uuid::new_v4();
        write_uuid(&id, &mut buf);
        let decoded = read_uuid(&mut buf).unwrap();
        assert_eq!(id, decoded);
    }

    #[test]
    fn test_block_position_roundtrip() {
        let positions = [
            BlockPosition::new(0, 0, 0),
            BlockPosition::new(100, 64, -200),
            BlockPosition::new(-33554432, -2048, 33554431),
            BlockPosition::new(-1, -64, -1),
        ];
        for &pos in &positions {
            let mut buf = BytesMut::new();
            pos.write(&mut buf);
            let decoded = BlockPosition::read(&mut buf).unwrap();
            assert_eq!(pos, decoded);
        }
    }
}
