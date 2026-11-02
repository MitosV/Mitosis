use std::io;
use bytes::BytesMut;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use mitosis_protocol::varint::{read_varint, ProtocolError};
use mitosis_protocol::{RawPacket, State};

use crate::compression::{compress_packet, decompress_packet};
use crate::encryption::EncryptionCipher;

pub struct MinecraftConnection {
    stream: TcpStream,
    read_buf: BytesMut,
    write_buf: Vec<u8>,
    cipher: Option<EncryptionCipher>,
    compression_threshold: Option<i32>,
    pub state: State,
}

impl MinecraftConnection {
    pub fn new(stream: TcpStream) -> Self {
        Self {
            stream,
            read_buf: BytesMut::with_capacity(32 * 1024),
            write_buf: Vec::with_capacity(32 * 1024),
            cipher: None,
            compression_threshold: None,
            state: State::Handshaking,
        }
    }

    pub fn enable_encryption(&mut self, key: &[u8]) {
        self.cipher = Some(EncryptionCipher::new(key));
    }

    pub fn enable_compression(&mut self, threshold: i32) {
        if threshold >= 0 {
            self.compression_threshold = Some(threshold);
        } else {
            self.compression_threshold = None;
        }
    }

    pub fn compression_threshold(&self) -> Option<i32> {
        self.compression_threshold
    }

    /// Reads one framed packet from the network connection.
    pub async fn read_packet(&mut self) -> Result<RawPacket, ProtocolError> {
        loop {
            // Check if we have a full packet in read_buf
            if let Some(packet) = self.try_decode_packet()? {
                return Ok(packet);
            }

            // Read more bytes from the stream
            let mut temp_buf = [0u8; 8192];
            let n = self
                .stream
                .read(&mut temp_buf)
                .await
                .map_err(|e| ProtocolError::Custom(format!("Socket read error: {}", e)))?;

            if n == 0 {
                return Err(ProtocolError::UnexpectedEof);
            }

            let slice = &mut temp_buf[..n];
            if let Some(cipher) = &mut self.cipher {
                cipher.decrypt(slice);
            }
            self.read_buf.extend_from_slice(slice);
        }
    }

    fn try_decode_packet(&mut self) -> Result<Option<RawPacket>, ProtocolError> {
        if self.read_buf.is_empty() {
            return Ok(None);
        }

        // Try reading length VarInt without consuming permanently unless valid
        let mut cursor = std::io::Cursor::new(&self.read_buf[..]);
        let packet_length = match read_varint(&mut cursor) {
            Ok(len) => len as usize,
            Err(ProtocolError::UnexpectedEof) => return Ok(None),
            Err(e) => return Err(e),
        };

        let varint_len = cursor.position() as usize;
        let total_required = varint_len + packet_length;

        if self.read_buf.len() < total_required {
            return Ok(None);
        }

        // We have the complete frame
        let _ = self.read_buf.split_to(varint_len);
        let frame_bytes = self.read_buf.split_to(packet_length).freeze();

        if let Some(threshold) = self.compression_threshold {
            let packet = decompress_packet(frame_bytes, threshold)?;
            Ok(Some(packet))
        } else {
            let packet = RawPacket::decode_uncompressed(frame_bytes)?;
            Ok(Some(packet))
        }
    }

    /// Writes one packet to the network connection.
    pub async fn write_packet(&mut self, packet: &RawPacket) -> Result<(), ProtocolError> {
        let frame_data = if let Some(threshold) = self.compression_threshold {
            compress_packet(packet, threshold)?
        } else {
            packet.encode_uncompressed()
        };

        self.write_buf.clear();
        self.write_buf.extend_from_slice(&frame_data);

        if let Some(cipher) = &mut self.cipher {
            cipher.encrypt(&mut self.write_buf);
        }

        self.stream
            .write_all(&self.write_buf)
            .await
            .map_err(|e| ProtocolError::Custom(format!("Socket write error: {}", e)))?;

        Ok(())
    }

    pub async fn flush(&mut self) -> Result<(), io::Error> {
        self.stream.flush().await
    }
}
