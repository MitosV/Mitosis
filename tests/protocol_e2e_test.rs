use std::time::Duration;
use bytes::{Bytes, BytesMut};
use tokio::net::TcpListener;
use tokio::time::timeout;
use uuid::Uuid;

use mitosis_identity::IdentityManager;
use mitosis_networking::MinecraftConnection;
use mitosis_protocol::configuration::*;
use mitosis_protocol::handshake::*;
use mitosis_protocol::login::*;
use mitosis_protocol::play::*;
use mitosis_protocol::{RawPacket, State};

#[tokio::test]
async fn test_protocol_e2e_handshake_and_play() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let secret = "test_mitosis_secret_key";
    let identity = IdentityManager::with_default_namespace(secret);
    let bot_uuid = identity.generate_uuid("group/test/bot/1");

    // Spawn Mock Server task
    let server_handle = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut conn = MinecraftConnection::new(stream);

        // 1. Handshake
        let hs = conn.read_packet().await.unwrap();
        assert_eq!(hs.id, ClientIntentionPacket::PACKET_ID);
        conn.state = State::Login;

        // 2. Login Start
        let login = conn.read_packet().await.unwrap();
        assert_eq!(login.id, ServerboundHelloPacket::PACKET_ID);
        let hello = ServerboundHelloPacket::read(&mut std::io::Cursor::new(login.data)).unwrap();
        assert_eq!(hello.name, "bot_tester");

        // 3. Send Compression (Threshold 256)
        let mut comp_buf = BytesMut::new();
        mitosis_protocol::varint::write_varint(256, &mut comp_buf);
        conn.write_packet(&RawPacket::new(ClientboundLoginCompressionPacket::PACKET_ID, comp_buf.freeze())).await.unwrap();
        conn.enable_compression(256);

        // 4. Send Login Success (GameProfile)
        let mut profile_buf = BytesMut::new();
        mitosis_protocol::types::write_uuid(&bot_uuid, &mut profile_buf);
        mitosis_protocol::types::write_string("bot_tester", &mut profile_buf);
        mitosis_protocol::varint::write_varint(0, &mut profile_buf); // 0 properties
        conn.write_packet(&RawPacket::new(ClientboundGameProfilePacket::PACKET_ID, profile_buf.freeze())).await.unwrap();

        // 5. Read Login Acknowledged
        let ack = conn.read_packet().await.unwrap();
        assert_eq!(ack.id, ServerboundLoginAcknowledgedPacket::PACKET_ID);
        conn.state = State::Configuration;

        // 6. Send Finish Configuration
        let finish_buf = BytesMut::new();
        conn.write_packet(&RawPacket::new(ClientboundFinishConfigurationPacket::PACKET_ID, finish_buf.freeze())).await.unwrap();

        // 7. Read Finish Configuration Acknowledged
        let finish_ack = conn.read_packet().await.unwrap();
        assert_eq!(finish_ack.id, ServerboundFinishConfigurationPacket::PACKET_ID);
        conn.state = State::Play;

        // 8. Server sends KeepAlive in Play state
        let mut keep_buf = BytesMut::new();
        keep_buf.extend_from_slice(&123456789i64.to_be_bytes());
        conn.write_packet(&RawPacket::new(ClientboundKeepAlivePacket::PACKET_ID, keep_buf.freeze())).await.unwrap();

        // 9. Read client KeepAlive response
        let keep_resp = conn.read_packet().await.unwrap();
        assert_eq!(keep_resp.id, ServerboundKeepAlivePacket::PACKET_ID);
        let resp_id = std::io::Cursor::new(keep_resp.data).get_i64();
        assert_eq!(resp_id, 123456789i64);
    });

    // Client connection
    let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    let mut client = MinecraftConnection::new(stream);

    // Run client handshake
    let mut hs_buf = BytesMut::new();
    ClientIntentionPacket::new("localhost", addr.port(), 2).write(&mut hs_buf);
    client.write_packet(&RawPacket::new(ClientIntentionPacket::PACKET_ID, hs_buf.freeze())).await.unwrap();
    client.state = State::Login;

    let mut login_buf = BytesMut::new();
    ServerboundHelloPacket {
        name: "bot_tester".into(),
        profile_id: bot_uuid,
    }.write(&mut login_buf);
    client.write_packet(&RawPacket::new(ServerboundHelloPacket::PACKET_ID, login_buf.freeze())).await.unwrap();

    // Client receives compression
    let pkt1 = client.read_packet().await.unwrap();
    assert_eq!(pkt1.id, ClientboundLoginCompressionPacket::PACKET_ID);
    client.enable_compression(256);

    // Client receives GameProfile
    let pkt2 = client.read_packet().await.unwrap();
    assert_eq!(pkt2.id, ClientboundGameProfilePacket::PACKET_ID);

    // Send Login Acknowledged
    let mut ack_buf = BytesMut::new();
    ServerboundLoginAcknowledgedPacket.write(&mut ack_buf);
    client.write_packet(&RawPacket::new(ServerboundLoginAcknowledgedPacket::PACKET_ID, ack_buf.freeze())).await.unwrap();
    client.state = State::Configuration;

    // Receive Finish Configuration
    let pkt3 = client.read_packet().await.unwrap();
    assert_eq!(pkt3.id, ClientboundFinishConfigurationPacket::PACKET_ID);

    // Send Finish Configuration Ack
    let mut fack_buf = BytesMut::new();
    ServerboundFinishConfigurationPacket.write(&mut fack_buf);
    client.write_packet(&RawPacket::new(ServerboundFinishConfigurationPacket::PACKET_ID, fack_buf.freeze())).await.unwrap();
    client.state = State::Play;

    // Receive Play KeepAlive
    let pkt4 = client.read_packet().await.unwrap();
    assert_eq!(pkt4.id, ClientboundKeepAlivePacket::PACKET_ID);

    // Send Play KeepAlive response
    let mut kresp_buf = BytesMut::new();
    ServerboundKeepAlivePacket { id: 123456789i64 }.write(&mut kresp_buf);
    client.write_packet(&RawPacket::new(ServerboundKeepAlivePacket::PACKET_ID, kresp_buf.freeze())).await.unwrap();

    timeout(Duration::from_secs(5), server_handle).await.unwrap().unwrap();
}
