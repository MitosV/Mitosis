use std::io;
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[derive(Debug, Error)]
pub enum ProxyError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("SOCKS5 error: {0}")]
    Socks5(String),
    #[error("HTTP CONNECT error: {0}")]
    HttpConnect(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProxyType {
    None,
    Socks5 { host: String, port: u16 },
    Http { host: String, port: u16 },
}

pub struct ProxyConnector;

impl ProxyConnector {
    /// Connects to a target host:port, optionally tunneling through a SOCKS5 or HTTP proxy.
    pub async fn connect(
        proxy: &ProxyType,
        target_host: &str,
        target_port: u16,
    ) -> Result<TcpStream, ProxyError> {
        match proxy {
            ProxyType::None => {
                let stream = TcpStream::connect(format!("{}:{}", target_host, target_port)).await?;
                Ok(stream)
            }
            ProxyType::Socks5 { host, port } => {
                let mut stream = TcpStream::connect(format!("{}:{}", host, port)).await?;
                Self::socks5_handshake(&mut stream, target_host, target_port).await?;
                Ok(stream)
            }
            ProxyType::Http { host, port } => {
                let mut stream = TcpStream::connect(format!("{}:{}", host, port)).await?;
                Self::http_connect_handshake(&mut stream, target_host, target_port).await?;
                Ok(stream)
            }
        }
    }

    async fn socks5_handshake(
        stream: &mut TcpStream,
        target_host: &str,
        target_port: u16,
    ) -> Result<(), ProxyError> {
        // 1. Initial greeting: SOCKS version 5, 1 auth method: 0 (No authentication)
        stream.write_all(&[0x05, 0x01, 0x00]).await?;

        let mut resp = [0u8; 2];
        stream.read_exact(&mut resp).await?;
        if resp[0] != 0x05 || resp[1] != 0x00 {
            return Err(ProxyError::Socks5(format!(
                "Unsupported SOCKS5 auth: 0x{:02x} 0x{:02x}",
                resp[0], resp[1]
            )));
        }

        // 2. CONNECT request: version 5, CMD 1 (CONNECT), RSV 0, ATYP 3 (Domain name)
        let host_bytes = target_host.as_bytes();
        let mut req = Vec::with_capacity(7 + host_bytes.len());
        req.extend_from_slice(&[0x05, 0x01, 0x00, 0x03, host_bytes.len() as u8]);
        req.extend_from_slice(host_bytes);
        req.extend_from_slice(&target_port.to_be_bytes());

        stream.write_all(&req).await?;

        // 3. Response: version 5, REP, RSV, ATYP, BND.ADDR, BND.PORT
        let mut head = [0u8; 4];
        stream.read_exact(&mut head).await?;
        if head[0] != 0x05 || head[1] != 0x00 {
            return Err(ProxyError::Socks5(format!(
                "SOCKS5 connect rejected with status: 0x{:02x}",
                head[1]
            )));
        }

        // Skip bound address
        match head[3] {
            0x01 => {
                // IPv4: 4 bytes + 2 bytes port
                let mut discard = [0u8; 6];
                stream.read_exact(&mut discard).await?;
            }
            0x03 => {
                // Domain: 1 byte len + domain bytes + 2 bytes port
                let mut len = [0u8; 1];
                stream.read_exact(&mut len).await?;
                let mut discard = vec![0u8; len[0] as usize + 2];
                stream.read_exact(&mut discard).await?;
            }
            0x04 => {
                // IPv6: 16 bytes + 2 bytes port
                let mut discard = [0u8; 18];
                stream.read_exact(&mut discard).await?;
            }
            _ => return Err(ProxyError::Socks5("Unknown address type in SOCKS5 response".into())),
        }

        Ok(())
    }

    async fn http_connect_handshake(
        stream: &mut TcpStream,
        target_host: &str,
        target_port: u16,
    ) -> Result<(), ProxyError> {
        let req = format!(
            "CONNECT {}:{} HTTP/1.1\r\nHost: {}:{}\r\nProxy-Connection: Keep-Alive\r\n\r\n",
            target_host, target_port, target_host, target_port
        );
        stream.write_all(req.as_bytes()).await?;

        let mut buf = [0u8; 512];
        let n = stream.read(&mut buf).await?;
        let resp = String::from_utf8_lossy(&buf[..n]);
        if !resp.starts_with("HTTP/1.1 200") && !resp.starts_with("HTTP/1.0 200") {
            return Err(ProxyError::HttpConnect(format!(
                "Proxy responded with: {}",
                resp.lines().next().unwrap_or("Empty response")
            )));
        }

        Ok(())
    }
}
