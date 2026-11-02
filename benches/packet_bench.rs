use std::time::Instant;
use bytes::BytesMut;
use mitosis_protocol::play::ServerboundMovePlayerPosRotPacket;
use mitosis_protocol::RawPacket;

fn main() {
    println!("=== MITOSIS Protocol Framing & Encoding Benchmark ===");

    let packet_count = 1_000_000;
    println!("Encoding & decoding {} MovePlayerPosRot packets...", packet_count);

    let start = Instant::now();
    let mut total_bytes = 0usize;

    for i in 0..packet_count {
        let pkt = ServerboundMovePlayerPosRotPacket {
            x: (i as f64) * 0.1,
            y: 64.0,
            z: (i as f64) * 0.1,
            yaw: 180.0,
            pitch: 0.0,
            on_ground: true,
        };

        let mut buf = BytesMut::with_capacity(64);
        pkt.write(&mut buf);
        let raw = RawPacket::new(ServerboundMovePlayerPosRotPacket::PACKET_ID, buf.freeze());
        let frame = raw.encode_uncompressed();

        total_bytes += frame.len();
    }

    let elapsed = start.elapsed();
    let packets_per_sec = packet_count as f64 / elapsed.as_secs_f64();
    let mb_per_sec = (total_bytes as f64 / (1024.0 * 1024.0)) / elapsed.as_secs_f64();

    println!("Total time: {:?}", elapsed);
    println!("Throughput: {:.2} packets/second", packets_per_sec);
    println!("Data throughput: {:.2} MB/s", mb_per_sec);
}
