use std::io::{Read, Write};
use bytes::{BufMut, Bytes, BytesMut};
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use mitosis_protocol::varint::{read_varint, write_varint, varint_size, ProtocolError};
use mitosis_protocol::RawPacket;

pub fn decompress_packet(
    mut frame_data: Bytes,
    threshold: i32,
) -> Result<RawPacket, ProtocolError> {
    let uncompressed_len = read_varint(&mut frame_data)?;

    if uncompressed_len == 0 {
        // Below compression threshold; uncompressed payload follows
        RawPacket::decode_uncompressed(frame_data)
    } else {
        if uncompressed_len < threshold {
            return Err(ProtocolError::Custom(format!(
                "Uncompressed size {} is below threshold {}",
                uncompressed_len, threshold
            )));
        }
        // Decompress with Zlib
        let mut decoder = ZlibDecoder::new(&frame_data[..]);
        let mut decompressed = Vec::with_capacity(uncompressed_len as usize);
        decoder
            .read_to_end(&mut decompressed)
            .map_err(|e| ProtocolError::DecompressionFailed(e.to_string()))?;

        let bytes = Bytes::from(decompressed);
        RawPacket::decode_uncompressed(bytes)
    }
}

pub fn compress_packet(
    packet: &RawPacket,
    threshold: i32,
) -> Result<Bytes, ProtocolError> {
    let id_size = varint_size(packet.id);
    let uncompressed_len = id_size + packet.data.len();

    if (uncompressed_len as i32) < threshold {
        // Frame: VarInt(packet_length) + VarInt(0) + VarInt(packet_id) + payload
        let data_len_size = varint_size(0);
        let total_packet_length = data_len_size + uncompressed_len;
        let total_frame_size = varint_size(total_packet_length as i32) + total_packet_length;

        let mut buf = BytesMut::with_capacity(total_frame_size);
        write_varint(total_packet_length as i32, &mut buf);
        write_varint(0, &mut buf);
        write_varint(packet.id, &mut buf);
        buf.put_slice(&packet.data);
        Ok(buf.freeze())
    } else {
        // Compress (VarInt(packet_id) + payload)
        let mut uncompressed_buf = BytesMut::with_capacity(uncompressed_len);
        write_varint(packet.id, &mut uncompressed_buf);
        uncompressed_buf.put_slice(&packet.data);

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(&uncompressed_buf)
            .map_err(|e| ProtocolError::Custom(e.to_string()))?;
        let compressed_bytes = encoder
            .finish()
            .map_err(|e| ProtocolError::Custom(e.to_string()))?;

        let uncompressed_len_varint_size = varint_size(uncompressed_len as i32);
        let total_packet_length = uncompressed_len_varint_size + compressed_bytes.len();
        let total_frame_size = varint_size(total_packet_length as i32) + total_packet_length;

        let mut buf = BytesMut::with_capacity(total_frame_size);
        write_varint(total_packet_length as i32, &mut buf);
        write_varint(uncompressed_len as i32, &mut buf);
        buf.put_slice(&compressed_bytes);
        Ok(buf.freeze())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compression_below_threshold() {
        let packet = RawPacket::new(0x01, Bytes::from_static(b"Short"));
        let encoded = compress_packet(&packet, 256).unwrap();

        let mut buf = encoded;
        let _total_len = read_varint(&mut buf).unwrap();
        let decoded = decompress_packet(buf, 256).unwrap();
        assert_eq!(decoded.id, packet.id);
        assert_eq!(decoded.data, packet.data);
    }

    #[test]
    fn test_compression_above_threshold() {
        let large_payload = vec![0xAB; 1024];
        let packet = RawPacket::new(0x27, Bytes::from(large_payload));
        let encoded = compress_packet(&packet, 256).unwrap();

        let mut buf = encoded;
        let _total_len = read_varint(&mut buf).unwrap();
        let decoded = decompress_packet(buf, 256).unwrap();
        assert_eq!(decoded.id, packet.id);
        assert_eq!(decoded.data, packet.data);
    }
}
