use std::{mem, time::Duration};
use tokio::{
    io::{AsyncReadExt, ReadHalf},
    net::TcpStream,
    sync::mpsc,
    time::timeout,
};

use super::super::message::{Message, parse};

use super::MessageRequest;

pub struct ReadTask {
    rd: ReadHalf<TcpStream>,
    buf: Vec<u8>,
    msg_queue: Vec<Message>,
    receiver: mpsc::Receiver<MessageRequest>,
}

impl ReadTask {
    pub(crate) fn new(
        rd: ReadHalf<TcpStream>,
        buf: Vec<u8>,
        receiver: mpsc::Receiver<MessageRequest>,
    ) -> Self {
        ReadTask {
            rd,
            buf,
            msg_queue: Vec::new(),
            receiver,
        }
    }

    async fn read_socket_task(&mut self) {
        loop {
            match self.receiver.try_recv() {
                Ok(MessageRequest { respond_to }) => {
                    let msg_queue = mem::take(&mut self.msg_queue);
                    match respond_to.send(msg_queue) {
                        Ok(_) => {}
                        Err(v) => self.msg_queue = v,
                    }
                }
                Err(_) => {}
            }

            let mut buf = [0; 17000];
            let read_fut = self.rd.read(&mut buf[..]);
            let n = match timeout(Duration::from_millis(300), read_fut).await {
                Ok(v) => v.unwrap(),
                Err(_) => 0,
            };

            let mut buf = buf[..n].to_vec();
            self.buf.append(&mut buf);

            // Repeatedly parse messages from buffered bytes until unable to do so
            let rem = loop {
                match parse(&self.buf) {
                    Ok((Some(msg), rem)) => {
                        self.msg_queue.insert(0, msg);
                        self.buf = rem;
                    }
                    Ok((None, rem)) => break rem,
                    Err(_) => {
                        return;
                    }
                }
            };
            self.buf = rem;
        }
    }
}

pub(crate) async fn run_read_task(mut read_task: ReadTask) {
    read_task.read_socket_task().await;
}
