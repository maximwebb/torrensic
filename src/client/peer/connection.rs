mod handshake;
mod read_task;

use std::sync::Arc;
use std::{error::Error, time::Duration};

use tokio::sync::{mpsc, oneshot};
use tokio::{
    io::{AsyncWriteExt, WriteHalf},
    net::TcpStream,
    time::timeout,
};

use crate::client::peer::connection::read_task::{run_read_task, ReadTask};
use crate::parser::{metadata::Metadata, trackerinfo::PeerInfo};

use self::handshake::handshake;
use super::message::interested::Interested;
use super::message::request::Request;
use super::message::{Message, PeerWireMessage};

pub struct Connection {
    msg_queue: Vec<Message>,
    wr: WriteHalf<TcpStream>,
    sender: mpsc::Sender<MessageRequest>,
}

impl Connection {
    pub(crate) async fn new(addr: &str, md: &Metadata) -> Result<Self, Box<dyn Error>> {
        // let addr = "185.70.186.197:6881";
        // let addr = format!("{}:{}", peer.ip, peer.port);
        let addr_: &str = &addr;
        let socket = TcpStream::connect(addr);
        let socket = match timeout(Duration::from_millis(3000), socket).await {
            Ok(v) => match v {
                Ok(v) => v,
                Err(e) => return Err(Box::new(e)),
            },
            Err(e) => return Err(Box::new(e)),
        };

        let (mut rd, mut wr) = tokio::io::split(socket);
        let rem = handshake(&md, &mut rd, &mut wr).await?;

        let (sender, receiver) = mpsc::channel(8);

        let conn = Connection {
            msg_queue: Vec::new(),
            wr,
            sender,
        };

        let read_task = ReadTask::new(rd, rem, receiver);
        tokio::spawn(run_read_task(read_task));

        // println!("Completed handshake with {}", addr_);
        Ok(conn)
    }

    // Updates message queue by polling read task, and returns top message if it exists
    pub(crate) async fn pop(&mut self) -> Result<Message, Box<dyn Error>> {
        let msg = loop {
            self.refresh_msg_queue().await?;
            match self.msg_queue.pop() {
                Some(v) => break v,
                None => continue,
            }
        };
        Ok(msg)
    }

    pub(crate) async fn try_pop(&mut self) -> Option<Message> {
        let _ = self.refresh_msg_queue().await;
        self.msg_queue.pop()
    }

    pub(crate) async fn push(&mut self, msg: Message) -> Result<(), Box<dyn Error>> {
        self.wr.write_all(&msg.serialise()).await?;
        Ok(())
    }

    pub(crate) async fn send_interested(&mut self) -> Result<(), Box<dyn Error>> {
        let interest_msg = Message::from(Interested {});
        self.wr.write_all(&interest_msg.serialise()).await?;
        Ok(())
    }

    pub(crate) async fn request_block(
        &mut self,
        md: &Metadata,
        piece_index: u32,
        block_index: u32,
    ) -> Result<(), Box<dyn Error>> {
        let request_msg = Message::from(Request {
            index: piece_index,
            begin: block_index * (2 << 13),
            length: md.block_len(piece_index, block_index),
        });
        self.wr.write_all(&request_msg.serialise()).await?;
        Ok(())
    }

    async fn refresh_msg_queue(&mut self) -> Result<(), Box<dyn Error>> {
        let (send, recv) = oneshot::channel();
        let _ = self.sender.send(MessageRequest { respond_to: send }).await;
        let msg_queue = recv.await.unwrap();
        self.msg_queue.splice(0..0, msg_queue);
        Ok(())
    }
}

pub(crate) struct MessageRequest {
    pub respond_to: oneshot::Sender<Vec<Message>>,
}
