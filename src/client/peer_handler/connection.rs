mod handshake;
mod read_task;

use std::{error::Error, time::Duration};

use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::oneshot::error::RecvError;
use tokio::sync::{mpsc, oneshot};
use tokio::{
    io::{AsyncWriteExt, WriteHalf},
    net::TcpStream,
    time::timeout,
};

use read_task::{run_read_task, ReadTask};

use self::handshake::handshake;

// Move to common / networking mod (perhaps merge utils + this into common)
pub trait Serialisable {
    fn serialise(&self) -> Vec<u8>;
}

pub trait Deserialisable {
    fn deserialise(raw: &Vec<u8>) -> Result<(Option<Self>, Vec<u8>), ()>
    where
        Self: Sized;
}

pub struct Connection<T: Serialisable + Deserialisable + Send + 'static> {
    msg_queue: Vec<T>,
    wr: WriteHalf<TcpStream>,
    sender: mpsc::Sender<MessageRequest<T>>,
}

impl<T: Serialisable + Deserialisable + Send + 'static> Connection<T> {
    pub(crate) async fn new(
        addr: &str,
        info_hash: &Vec<u8>,
        cancel_sender: mpsc::Sender<()>,
        req_metadata: bool,
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
        let rem = handshake(&info_hash, &mut rd, &mut wr, req_metadata).await?;

        let (sender, receiver): (Sender<MessageRequest<T>>, Receiver<MessageRequest<T>>) =
            mpsc::channel(8);

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
    pub(crate) async fn pop(&mut self) -> Result<T, RecvError> {
        let msg = loop {
            match self.refresh_msg_queue().await {
                Ok(_) => {}
                Err(err) => return Err(err),
            }
            match self.msg_queue.pop() {
                Some(v) => break v,
                None => continue,
            }
        };
        Ok(msg)
    }

    pub(crate) async fn push(&mut self, msg: T) -> Result<(), Box<dyn Error>> {
        self.wr.write_all(&msg.serialise()).await?;
        Ok(())
    }

    pub(crate) async fn push_raw(&mut self, msg_bytes: &Vec<u8>) -> Result<(), Box<dyn Error>> {
        self.wr.write_all(&msg_bytes).await?;
        Ok(())
    }

    async fn refresh_msg_queue(&mut self) -> Result<(), RecvError> {
        let (send, recv) = oneshot::channel();
        // Wait on the read_task to return one or more messages over the channel
        let _ = self.sender.send(MessageRequest { respond_to: send }).await;
        let msg_queue = match recv.await {
            Ok(queue) => queue,
            Err(err) => return Err(err),
        };
        self.msg_queue.splice(0..0, msg_queue);
        Ok(())
    }
}

pub(crate) struct MessageRequest<T: Serialisable + Deserialisable> {
    pub respond_to: oneshot::Sender<Vec<T>>,
}
