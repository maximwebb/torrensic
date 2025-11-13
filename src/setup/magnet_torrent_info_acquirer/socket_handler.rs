use std::time::Duration;

use tokio::io::AsyncReadExt;
use tokio::{
    io::{self, AsyncWriteExt},
    net::TcpStream,
    time::timeout,
};

use crate::{client::handshake_message::get_handshake_bytes, log};

pub struct MagnetSocketHandler {
    socket: TcpStream,
    buf: [u8; 2048],
}

impl MagnetSocketHandler {
    pub async fn try_new(addr: &str) -> io::Result<MagnetSocketHandler> {
        let socket = TcpStream::connect(addr);
        let socket = timeout(Duration::from_millis(3000), socket).await??;

        Ok(MagnetSocketHandler {
            socket,
            buf: [0; 2048],
        })
    }

    pub async fn read(&mut self) -> io::Result<Vec<u8>> {
        loop {
            let len = self.socket.read_u32().await?;

            if len <= 1 {
                log!("Got keepalive message, ignoring");
                continue;
            }

            let msg_type = self.socket.read_u8().await?;

            if msg_type != 20 {
                continue;
            }
            let mut msg = vec![0u8; (len - 1) as usize];
            self.socket.read_exact(&mut msg).await?;

            return Ok(msg[1..].to_vec());
        }
    }

    pub async fn write(&mut self, bytes: &Vec<u8>) -> io::Result<()> {
        let _ = self.socket.write(bytes).await?;
        Ok(())
    }

    pub async fn handshake(&mut self, info_hash: &Vec<u8>) -> io::Result<Vec<u8>> {
        let handshake_bytes = get_handshake_bytes(info_hash);
        let _ = self.socket.write(&handshake_bytes).await?;

        const HANDSHAKE_LEN: usize = 68;

        self.socket
            .read_exact(&mut self.buf[..HANDSHAKE_LEN as usize])
            .await?;

        Ok(self.buf[..HANDSHAKE_LEN].to_vec())
    }
}
