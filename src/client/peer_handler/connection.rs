mod handshake;
mod read_task;

use std::{error::Error, time::Duration};

use tokio::sync::oneshot::error::RecvError;
use tokio::sync::{mpsc, oneshot};
use tokio::{
    io::{AsyncWriteExt, WriteHalf},
    net::TcpStream,
    time::timeout,
};

use read_task::{run_read_task, ReadTask};
use crate::parser::bootstrap_info::BootstrapInfo;

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
    pub(crate) async fn new(
        addr: &str,
        md: &BootstrapInfo,
        cancel_sender: mpsc::Sender<()>,
    ) -> Result<Self, Box<dyn Error>> {
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

        let read_task = ReadTask::new(rd, rem, receiver, cancel_sender);
        tokio::spawn(run_read_task(read_task));
        
        Ok(conn)
    }

    // Updates message queue by polling read task, and returns top message if it exists
    pub(crate) async fn pop(&mut self) -> Result<Message, RecvError> {
        let msg = loop {
            match self.refresh_msg_queue().await {
                Ok(_) => {},
                Err(err) => return Err(err)
            }
            match self.msg_queue.pop() {
                Some(v) => break v,
                None => continue,
            }
        };
        Ok(msg)
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
        md: &BootstrapInfo,
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

    async fn refresh_msg_queue(&mut self) -> Result<(), RecvError> {
        let (send, recv) = oneshot::channel();
        let _ = self.sender.send(MessageRequest { respond_to: send }).await;
        let msg_queue = match recv.await {
            Ok(queue) => queue,
            Err(err) => {
                return Err(err)
            },
        };
        self.msg_queue.splice(0..0, msg_queue);
        Ok(())
    }
}

pub(crate) struct MessageRequest {
    pub respond_to: oneshot::Sender<Vec<Message>>,
}
